"use client";

import { useState, useCallback, useEffect, useRef } from 'react';
import { Button } from '@/components/ui/button';
import { ButtonGroup } from '@/components/ui/button-group';
import { Copy, FolderOpen, RefreshCw, Users } from 'lucide-react';
import { RetranscribeDialog } from './RetranscribeDialog';
import { useConfig } from '@/contexts/ConfigContext';
import { useTranslation } from 'react-i18next';


interface TranscriptButtonGroupProps {
  transcriptCount: number;
  onCopyTranscript: () => void;
  onOpenMeetingFolder: () => Promise<void>;
  meetingId?: string;
  meetingFolderPath?: string | null;
  onRefetchTranscripts?: () => Promise<void>;
  onManageSpeakers?: () => void;
}


export function TranscriptButtonGroup({
  transcriptCount,
  onCopyTranscript,
  onOpenMeetingFolder,
  meetingId,
  meetingFolderPath,
  onRefetchTranscripts,
  onManageSpeakers,
}: TranscriptButtonGroupProps) {
  const { t } = useTranslation('meeting');
  const { betaFeatures } = useConfig();
  const [showRetranscribeDialog, setShowRetranscribeDialog] = useState(false);

  // The panel is a fraction of the window, so viewport breakpoints cannot tell
  // when the labels fit. Measure the container and collapse to icon-only
  // buttons (tooltips remain) before the group can overflow.
  const containerRef = useRef<HTMLDivElement>(null);
  const [compact, setCompact] = useState(false);

  useEffect(() => {
    const element = containerRef.current;
    if (!element) return;
    const observer = new ResizeObserver((entries) => {
      for (const entry of entries) {
        setCompact(entry.contentRect.width < 400);
      }
    });
    observer.observe(element);
    return () => observer.disconnect();
  }, []);

  const labelClass = compact ? 'hidden' : 'inline';

  const handleRetranscribeComplete = useCallback(async () => {
    // Refetch transcripts to show the updated data
    if (onRefetchTranscripts) {
      await onRefetchTranscripts();
    }
  }, [onRefetchTranscripts]);

  return (
    <div ref={containerRef} className="flex items-center justify-center w-full gap-2">
      <ButtonGroup>
        <Button
          variant="outline"
          size="sm"
          onClick={() => {
            onCopyTranscript();
          }}
          disabled={transcriptCount === 0}
          title={transcriptCount === 0 ? t('noTranscript') : t('recording:copyTranscript')}
        >
          <Copy />
          <span className={labelClass}>{t('common:copy')}</span>
        </Button>

        {onManageSpeakers && (
          <Button size="sm" variant="outline" onClick={onManageSpeakers} title={t('manageSpeakers')}>
            <Users size={18} />
            <span className={labelClass}>{t('manageSpeakers')}</span>
          </Button>
        )}

        <Button
          size="sm"
          variant="outline"
          onClick={() => {
            onOpenMeetingFolder();
          }}
          title={t('openFolder')}
        >
          <FolderOpen size={18} />
          <span className={labelClass}>{t('openFolder')}</span>
        </Button>

        {betaFeatures.importAndRetranscribe && meetingId && meetingFolderPath && (
          <Button
            size="sm"
            variant="outline"
            className="bg-gradient-to-r from-blue-50 to-purple-50 hover:from-blue-100 hover:to-purple-100 border-blue-200"
            onClick={() => {
              setShowRetranscribeDialog(true);
            }}
            title={t('retranscribe')}
          >
            <RefreshCw size={18} />
            <span className={labelClass}>{t('retranscribe')}</span>
          </Button>
        )}
      </ButtonGroup>

      {betaFeatures.importAndRetranscribe && meetingId && meetingFolderPath && (
        <RetranscribeDialog
          open={showRetranscribeDialog}
          onOpenChange={setShowRetranscribeDialog}
          meetingId={meetingId}
          meetingFolderPath={meetingFolderPath}
          onComplete={handleRetranscribeComplete}
        />
      )}
    </div>
  );
}
