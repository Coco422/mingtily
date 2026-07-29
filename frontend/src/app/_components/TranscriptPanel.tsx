import { VirtualizedTranscriptView } from '@/components/VirtualizedTranscriptView';
import { PermissionWarning } from '@/components/PermissionWarning';
import { Button } from '@/components/ui/button';
import { ButtonGroup } from '@/components/ui/button-group';
import { Copy, GlobeIcon } from 'lucide-react';
import { useTranscripts } from '@/contexts/TranscriptContext';
import { useConfig } from '@/contexts/ConfigContext';
import { useRecordingState } from '@/contexts/RecordingStateContext';
import { usePermissionCheck } from '@/hooks/usePermissionCheck';
import { ModalType } from '@/hooks/useModalState';
import { useIsLinux } from '@/hooks/usePlatform';
import { useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
  isAutomaticLanguageOnly,
  isStreamingRecognitionModel,
} from '@/lib/sherpa-asr';
import { transcriptService } from '@/services/transcriptService';
import type { LiveTranscriptUpdate } from '@/types';

/**
 * TranscriptPanel Component
 *
 * Displays transcript content with controls for copying and language settings.
 * Uses TranscriptContext, ConfigContext, and RecordingStateContext internally.
 */

interface TranscriptPanelProps {
  // indicates stop-processing state for transcripts; derived from backend statuses.
  isProcessingStop: boolean;
  isStopping: boolean;
  showModal: (name: ModalType, message?: string) => void;
}

export function TranscriptPanel({
  isProcessingStop,
  isStopping,
  showModal
}: TranscriptPanelProps) {
  const { t } = useTranslation('recording');
  // Contexts
  const { transcripts, transcriptContainerRef, copyTranscript } = useTranscripts();
  const { transcriptModelConfig } = useConfig();
  const { isRecording, isPaused } = useRecordingState();
  const { checkPermissions, isChecking, hasSystemAudio, hasMicrophone } = usePermissionCheck();
  const isLinux = useIsLinux();
  const [liveTranscript, setLiveTranscript] = useState<LiveTranscriptUpdate | null>(null);
  const usesStreamingRecognition = isStreamingRecognitionModel(
    transcriptModelConfig.provider,
    transcriptModelConfig.model
  );

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;

    void transcriptService.onLiveTranscriptUpdate((update) => {
      if (!cancelled) setLiveTranscript(update);
    }).then((nextUnlisten) => {
      if (cancelled) nextUnlisten();
      else unlisten = nextUnlisten;
    });

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  useEffect(() => {
    if (!isRecording || !usesStreamingRecognition) {
      setLiveTranscript(null);
    }
  }, [isRecording, usesStreamingRecognition]);

  useEffect(() => {
    if (!liveTranscript?.is_final) return;
    const finalizedOverlap = transcripts.some((transcript) => {
      const start = transcript.audio_start_time ?? 0;
      const end = transcript.audio_end_time ?? start;
      return end >= liveTranscript.audio_start_time
        && start <= liveTranscript.audio_end_time;
    });
    if (finalizedOverlap) setLiveTranscript(null);
  }, [liveTranscript, transcripts]);

  // Convert transcripts to segments for virtualized view
  const segments = useMemo(() =>
    transcripts.map(t => ({
      id: t.id,
      timestamp: t.audio_start_time ?? 0,
      endTime: t.audio_end_time,
      text: t.text,
      confidence: t.confidence,
      speaker: t.speaker,
      speakerIsProvisional: t.speaker_is_provisional,
    })),
    [transcripts]
  );
  const liveSegment = useMemo(() => {
    if (!usesStreamingRecognition || !liveTranscript?.text.trim()) return null;
    return {
      id: `live-${liveTranscript.utterance_id}`,
      timestamp: liveTranscript.audio_start_time,
      endTime: liveTranscript.audio_end_time,
      text: liveTranscript.text,
    };
  }, [liveTranscript, usesStreamingRecognition]);

  return (
    <div ref={transcriptContainerRef} className="w-full border-r border-gray-200 bg-white flex flex-col overflow-y-auto">
      {/* Title area - Sticky header */}
      <div className="sticky top-0 z-10 bg-white p-4 border-gray-200">
        <div className="flex flex-col space-y-3">
          <div className="flex  flex-col space-y-2">
            <div className="flex justify-center  items-center space-x-2">
              <ButtonGroup>
                {transcripts?.length > 0 && (
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={copyTranscript}
                    title={t('copyTranscript')}
                  >
                    <Copy />
                    <span className='hidden md:inline'>
                      {t('common:copy')}
                    </span>
                  </Button>
                )}
                {!isAutomaticLanguageOnly(
                  transcriptModelConfig.provider,
                  transcriptModelConfig.model
                ) &&
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => showModal('languageSettings')}
                    title={t('transcriptLanguage')}
                  >
                    <GlobeIcon />
                    <span className='hidden md:inline'>
                      {t('transcriptLanguage')}
                    </span>
                  </Button>
                }
              </ButtonGroup>
            </div>
          </div>
        </div>
      </div>

      {/* Permission Warning - Not needed on Linux */}
      {!isRecording && !isChecking && !isLinux && (
        <div className="flex justify-center px-4 pt-4">
          <PermissionWarning
            hasMicrophone={hasMicrophone}
            hasSystemAudio={hasSystemAudio}
            onRecheck={checkPermissions}
            isRechecking={isChecking}
          />
        </div>
      )}

      {/* Transcript content */}
      <div className="pb-20">
        <div className="flex justify-center">
          <div className="w-2/3 max-w-[750px]">
            <VirtualizedTranscriptView
              segments={segments}
              isRecording={isRecording}
              isPaused={isPaused}
              isProcessing={isProcessingStop}
              isStopping={isStopping}
              enableStreaming={isRecording && !usesStreamingRecognition}
              liveSegment={liveSegment}
              showConfidence={true}
            />
          </div>
        </div>
      </div>
    </div>
  );
}
