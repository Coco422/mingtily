'use client';

import { useCallback, useRef, useReducer, startTransition, useEffect, useState, memo } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { useAutoScroll } from "@/hooks/useAutoScroll";
import { useTranscriptStreaming } from "@/hooks/useTranscriptStreaming";
import { ConfidenceIndicator } from "./ConfidenceIndicator";
import { Tooltip, TooltipContent, TooltipTrigger } from "./ui/tooltip";
import { RecordingStatusBar } from "./RecordingStatusBar";
import { motion, AnimatePresence } from "framer-motion";
import { TranscriptSegmentData } from "@/types";
import { formatSpeakerLabel, speakerColor } from "@/lib/speaker-label";
import { resolveSpeaker, type SpeakerParticipant } from '@/lib/speaker-map';
import { useTranslation } from 'react-i18next';

export interface VirtualizedTranscriptViewProps {
    /** Transcript segments to display */
    segments: TranscriptSegmentData[];
    /** Whether recording is in progress */
    isRecording?: boolean;
    /** Whether recording is paused */
    isPaused?: boolean;
    /** Whether processing/finalizing transcription */
    isProcessing?: boolean;
    /** Whether stopping */
    isStopping?: boolean;
    /** Enable streaming effect for latest segment */
    enableStreaming?: boolean;
    /** True streaming hypothesis that may be revised by the recognizer */
    liveSegment?: TranscriptSegmentData | null;
    /** Show confidence indicators */
    showConfidence?: boolean;
    /** Completely disable auto-scroll behavior (for meeting details page) */
    disableAutoScroll?: boolean;

    speakerParticipants?: SpeakerParticipant[];
    onSpeakerClick?: (sourceSpeaker: string) => void;
}

// Threshold for enabling virtualization (below this, use simple rendering)
const VIRTUALIZATION_THRESHOLD = 10;
const DEFAULT_TRANSCRIPT_PANEL_WIDTH = 360;
const TRANSCRIPT_HORIZONTAL_CHROME = 90;
const TEXT_LINE_HEIGHT = 26;

function estimateTranscriptHeight(segment: TranscriptSegmentData, panelWidth: number): number {
    const textWidth = Math.max(120, panelWidth - TRANSCRIPT_HORIZONTAL_CHROME);
    const unitsPerLine = Math.max(8, textWidth / 16);
    let textUnits = 0;
    for (const character of segment.text) {
        textUnits += (character.codePointAt(0) ?? 0) <= 0xff ? 0.55 : 1;
    }
    const lineCount = Math.max(1, Math.ceil(textUnits / unitsPerLine));
    const speakerHeight = segment.speaker ? 20 : 0;
    return 12 + speakerHeight + lineCount * TEXT_LINE_HEIGHT;
}

// Helper function to format seconds as recording-relative time [MM:SS]
function formatRecordingTime(seconds: number | undefined): string {
    if (seconds === undefined) return '[--:--]';

    const totalSeconds = Math.floor(seconds);
    const minutes = Math.floor(totalSeconds / 60);
    const secs = totalSeconds % 60;

    return `[${minutes.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}]`;
}

// Helper function to remove filler words and repetitions
function cleanStopWords(text: string): string {
    const stopWords = ['uh', 'um', 'er', 'ah', 'hmm', 'hm', 'eh', 'oh'];

    let cleanedText = text;
    stopWords.forEach(word => {
        const pattern = new RegExp(`\\b${word}\\b[,\\s]*`, 'gi');
        cleanedText = cleanedText.replace(pattern, ' ');
    });

    return cleanedText.replace(/\s+/g, ' ').trim();
}

// Memoized transcript segment component
const TranscriptSegment = memo(function TranscriptSegment({
    id,
    timestamp,
    text,
    confidence,
    speaker,
    speakerIsProvisional,
    isStreaming,
    isLiveHypothesis,
    showConfidence,
    speakerParticipants,
    onSpeakerClick,
}: {
    id: string;
    timestamp: number;
    text: string;
    confidence?: number;
    speaker?: string | null;
    speakerIsProvisional?: boolean;
    isStreaming: boolean;
    isLiveHypothesis?: boolean;
    showConfidence: boolean;
    speakerParticipants: SpeakerParticipant[];
    onSpeakerClick?: (sourceSpeaker: string) => void;
}) {
    const { t } = useTranslation(['common', 'recording']);
    const displayText = cleanStopWords(text) || (text.trim() === '' ? t('silence') : text);
    const resolvedSpeaker = resolveSpeaker(speaker, speakerParticipants, t);
    const speakerLabel = resolvedSpeaker?.label || formatSpeakerLabel(speaker, t);

    return (
        <div id={`segment-${id}`} className="mb-3">
            <div className="flex items-start gap-2">
                <Tooltip>
                    <TooltipTrigger>
                        <span className="text-xs text-gray-400 mt-1 flex-shrink-0 min-w-[50px]">
                            {formatRecordingTime(timestamp)}
                        </span>
                    </TooltipTrigger>
                    <TooltipContent>
                        {confidence !== undefined && showConfidence && (
                            <ConfidenceIndicator confidence={confidence} showIndicator={showConfidence} />
                        )}
                    </TooltipContent>
                </Tooltip>
                <div className="flex-1">
                    {speakerLabel && (
                        <button
                            type="button"
                            disabled={!speaker || !onSpeakerClick}
                            onClick={() => speaker && onSpeakerClick?.(speaker)}
                            className="mb-1 text-left text-xs font-semibold disabled:cursor-default"
                            style={{ color: resolvedSpeaker?.color || speakerColor(speaker) }}
                        >
                            {speakerLabel}{speakerIsProvisional ? ` · ${t('live')}` : ''}
                        </button>
                    )}
                    {isStreaming ? (
                        <div className={isLiveHypothesis
                            ? 'rounded-md border border-sky-200 bg-sky-50/50 px-3 py-2 transition-colors duration-150'
                            : 'rounded-md border border-gray-200 bg-gray-100 px-3 py-2'
                        }>
                            {isLiveHypothesis && (
                                <div className="mb-1 flex items-center gap-1.5 text-[11px] font-medium text-sky-700">
                                    <span className="h-1.5 w-1.5 rounded-full bg-sky-600 animate-pulse" />
                                    {t('recording:liveRevisionHint')}
                                </div>
                            )}
                            <p className="text-base text-gray-800 leading-relaxed">{displayText}</p>
                        </div>
                    ) : (
                        <p className="text-base text-gray-800 leading-relaxed">{displayText}</p>
                    )}
                </div>
            </div>
        </div>
    );
});

export const VirtualizedTranscriptView: React.FC<VirtualizedTranscriptViewProps> = ({
    segments,
    isRecording = false,
    isPaused = false,
    isProcessing = false,
    isStopping = false,
    enableStreaming = false,
    liveSegment = null,
    showConfidence = true,
    disableAutoScroll = false,
    speakerParticipants = [],
    onSpeakerClick,
}) => {
    const { t } = useTranslation('recording');
    // Create scroll ref first - shared between virtualizer and auto-scroll hook
    const scrollRef = useRef<HTMLDivElement>(null);
    const [panelWidth, setPanelWidth] = useState(DEFAULT_TRANSCRIPT_PANEL_WIDTH);

    // Force re-render without flushSync (avoids React warning)
    const [, rerender] = useReducer((x: number) => x + 1, 0);

    const estimateSize = useCallback(
        (index: number) => estimateTranscriptHeight(segments[index], panelWidth),
        [panelWidth, segments],
    );
    const getItemKey = useCallback(
        (index: number) => segments[index]?.id ?? index,
        [segments],
    );

    // Setup virtualizer for efficient rendering of large lists.
    const virtualizer = useVirtualizer({
        count: segments.length,
        getScrollElement: () => scrollRef.current,
        estimateSize,
        getItemKey,
        overscan: 10,
        onChange: () => {
            startTransition(() => {
                rerender();
            });
        },
    });

    useEffect(() => {
        const element = scrollRef.current;
        if (!element) return;

        const observer = new ResizeObserver(([entry]) => {
            if (!entry) return;
            const nextWidth = Math.round(entry.contentRect.width);
            setPanelWidth((currentWidth) => currentWidth === nextWidth ? currentWidth : nextWidth);
        });
        observer.observe(element);
        return () => observer.disconnect();
    }, []);

    useEffect(() => {
        virtualizer.measure();
    }, [panelWidth, virtualizer]);

    // Custom hook for auto-scrolling (supports both virtualized and non-virtualized)
    useAutoScroll({
        scrollRef,
        segments,
        isRecording,
        isPaused,
        virtualizer,
        virtualizationThreshold: VIRTUALIZATION_THRESHOLD,
        disableAutoScroll,
    });

    // Streaming text effect hook (typewriter animation for new transcripts)
    const { streamingSegmentId, getDisplayText } = useTranscriptStreaming(
        segments,
        isRecording,
        enableStreaming
    );

    // Use simple rendering for small lists, virtualization for large lists
    const useVirtualization = segments.length >= VIRTUALIZATION_THRESHOLD;

    return (
        <div ref={scrollRef} className="flex flex-col h-full overflow-y-auto px-4 py-2">
            {/* Recording Status Bar - Sticky at top, always visible when recording */}
            <AnimatePresence>
                {isRecording && (
                    <div className="sticky top-0 z-10 bg-white pb-2">
                        <RecordingStatusBar isPaused={isPaused} />
                    </div>
                )}
            </AnimatePresence>

            {/* Content - add padding when recording to prevent overlap */}
            <div className={isRecording ? 'pt-2' : ''}>
            {segments.length === 0 && !liveSegment ? (
                // Empty state
                <motion.div
                    initial={{ opacity: 0 }}
                    animate={{ opacity: 1 }}
                    className="text-center text-gray-500 mt-8"
                >
                    {isRecording ? (
                        <>
                            <div className="flex items-center justify-center mb-3">
                                <div className={`w-3 h-3 rounded-full ${isPaused ? 'bg-orange-500' : 'bg-blue-500 animate-pulse'}`}></div>
                            </div>
                            <p className="text-sm text-gray-600">
                                {isPaused ? t('pausedWaiting') : t('listeningForSpeech')}
                            </p>
                            <p className="text-xs mt-1 text-gray-400">
                                {isPaused ? t('resumeHint') : t('liveTranscriptionHint')}
                            </p>
                        </>
                    ) : (
                        <>
                            <p className="text-lg font-semibold">{t('welcome')}</p>
                            <p className="text-xs mt-1">{t('welcomeHint')}</p>
                        </>
                    )}
                </motion.div>
            ) : useVirtualization ? (
                // Virtualized rendering for large lists
                <>
                    <div
                        style={{
                            height: virtualizer.getTotalSize(),
                            width: "100%",
                            position: "relative",
                        }}
                    >
                        {virtualizer.getVirtualItems().map((virtualRow) => {
                            const segment = segments[virtualRow.index];
                            const isStreaming = streamingSegmentId === segment.id;

                            return (
                                <div
                                    key={segment.id}
                                    data-index={virtualRow.index}
                                    ref={virtualizer.measureElement}
                                    style={{
                                        position: "absolute",
                                        top: 0,
                                        left: 0,
                                        width: "100%",
                                        transform: `translateY(${virtualRow.start}px)`,
                                    }}
                                >
                                    <TranscriptSegment
                                        id={segment.id}
                                        timestamp={segment.timestamp}
                                        text={getDisplayText(segment)}
                                        confidence={segment.confidence}
                                        speaker={segment.speaker}
                                        speakerIsProvisional={segment.speakerIsProvisional}
                                        isStreaming={isStreaming}
                                        isLiveHypothesis={false}
                                        showConfidence={showConfidence}
                                        speakerParticipants={speakerParticipants}
                                        onSpeakerClick={onSpeakerClick}
                                    />
                                </div>
                            );
                        })}
                    </div>

                    {/* Listening indicator when recording */}
                    {!isStopping && isRecording && !isPaused && !isProcessing && segments.length > 0 && (
                        <motion.div
                            initial={{ opacity: 0 }}
                            animate={{ opacity: 1 }}
                            exit={{ opacity: 0 }}
                            className="flex items-center gap-2 mt-4 text-gray-500"
                        >
                            <div className="w-2 h-2 bg-blue-500 rounded-full animate-pulse"></div>
                            <span className="text-sm">{t('listening')}</span>
                        </motion.div>
                    )}
                </>
            ) : (
                // Simple rendering for small lists (better animations)
                <>
                    <div className="space-y-1">
                        {segments.map((segment) => {
                            const isStreaming = streamingSegmentId === segment.id;

                            return (
                                <motion.div
                                    key={segment.id}
                                    initial={{ opacity: 0, y: 5 }}
                                    animate={{ opacity: 1, y: 0 }}
                                    transition={{ duration: 0.15 }}
                                >
                                    <TranscriptSegment
                                        id={segment.id}
                                        timestamp={segment.timestamp}
                                        text={getDisplayText(segment)}
                                        confidence={segment.confidence}
                                        speaker={segment.speaker}
                                        speakerIsProvisional={segment.speakerIsProvisional}
                                        isStreaming={isStreaming}
                                        isLiveHypothesis={false}
                                        showConfidence={showConfidence}
                                        speakerParticipants={speakerParticipants}
                                        onSpeakerClick={onSpeakerClick}
                                    />
                                </motion.div>
                            );
                        })}
                    </div>

                    {/* Listening indicator when recording */}
                    {!isStopping && isRecording && !isPaused && !isProcessing && segments.length > 0 && (
                        <motion.div
                            initial={{ opacity: 0 }}
                            animate={{ opacity: 1 }}
                            exit={{ opacity: 0 }}
                            className="flex items-center gap-2 mt-4 text-gray-500"
                        >
                            <div className="w-2 h-2 bg-blue-500 rounded-full animate-pulse"></div>
                            <span className="text-sm">{t('listening')}</span>
                        </motion.div>
                    )}
                </>
            )}
            {isRecording && liveSegment && liveSegment.text.trim() && (
                <motion.div
                    key={liveSegment.id}
                    initial={{ opacity: 0, y: 4 }}
                    animate={{ opacity: 1, y: 0 }}
                    transition={{ duration: 0.15 }}
                    className="mt-2"
                >
                    <TranscriptSegment
                        id={liveSegment.id}
                        timestamp={liveSegment.timestamp}
                        text={liveSegment.text}
                        confidence={liveSegment.confidence}
                        speaker={liveSegment.speaker}
                        speakerIsProvisional={liveSegment.speakerIsProvisional}
                        isStreaming
                        isLiveHypothesis
                        showConfidence={false}
                        speakerParticipants={speakerParticipants}
                        onSpeakerClick={onSpeakerClick}
                    />
                </motion.div>
            )}
            </div>
        </div>
    );
};
