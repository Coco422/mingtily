import { Loader2 } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { RecordingShutdownProgressPayload } from '@/services/recordingService';

interface StatusOverlaysProps {
  isStopping: boolean;
  isProcessing: boolean;
  isSaving: boolean;
  shutdownProgress: RecordingShutdownProgressPayload | null;
  sidebarCollapsed: boolean;
}

interface StatusOverlayProps {
  show: boolean;
  message: string;
  sidebarCollapsed: boolean;
  progress?: number;
}

function StatusOverlay({
  show,
  message,
  sidebarCollapsed,
  progress,
}: StatusOverlayProps) {
  if (!show) return null;

  const normalizedProgress = progress === undefined
    ? undefined
    : Math.min(100, Math.max(0, Math.round(progress)));

  return (
    <div className="fixed bottom-4 left-0 right-0 z-10">
      <div
        className="flex justify-center pl-8 transition-[margin] duration-300"
        style={{ marginLeft: sidebarCollapsed ? '4rem' : '16rem' }}
      >
        <div className="w-2/3 max-w-[560px]">
          <div className="rounded-lg border border-black/[0.08] bg-white px-4 py-3 shadow-[0_2px_4px_rgba(0,0,0,0.10)]">
            <div className="flex items-center gap-3">
              {normalizedProgress === undefined && (
                <Loader2 className="h-4 w-4 shrink-0 animate-spin text-gray-700" />
              )}
              <span className="min-w-0 flex-1 text-sm text-gray-700">{message}</span>
              {normalizedProgress !== undefined && (
                <span className="font-mono text-xs tabular-nums text-gray-500">
                  {normalizedProgress}%
                </span>
              )}
            </div>

            {normalizedProgress !== undefined && (
              <div
                role="progressbar"
                aria-label={message}
                aria-valuemin={0}
                aria-valuemax={100}
                aria-valuenow={normalizedProgress}
                className="mt-2 h-1 overflow-hidden rounded-full bg-black/[0.08]"
              >
                <div
                  className="h-full rounded-full bg-gray-900 transition-[width] duration-150 ease-out"
                  style={{ width: `${normalizedProgress}%` }}
                />
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}

export function StatusOverlays({
  isStopping,
  isProcessing,
  isSaving,
  shutdownProgress,
  sidebarCollapsed,
}: StatusOverlaysProps) {
  const { t } = useTranslation('recording');

  const stopMessage = (() => {
    switch (shutdownProgress?.stage) {
      case 'stopping_audio':
        return t('stopStages.stoppingAudio');
      case 'processing_transcripts':
        return t('stopStages.processingTranscripts');
      case 'unloading_model':
        return t('stopStages.unloadingModel');
      case 'finalizing':
        return t('stopStages.finalizingAudio');
      case 'refining_speakers':
        return shutdownProgress.current_window && shutdownProgress.total_windows
          ? t('stopStages.refiningSpeakers', {
              current: shutdownProgress.current_window,
              total: shutdownProgress.total_windows,
            })
          : t('stopStages.preparingSpeakers');
      case 'complete':
        return t('stopStages.complete');
      default:
        return t('stopping');
    }
  })();

  return (
    <>
      <StatusOverlay
        show={isStopping}
        message={stopMessage}
        progress={shutdownProgress?.progress}
        sidebarCollapsed={sidebarCollapsed}
      />
      <StatusOverlay
        show={isProcessing}
        message={t('finalizing')}
        sidebarCollapsed={sidebarCollapsed}
      />
      <StatusOverlay
        show={isSaving}
        message={t('saving')}
        sidebarCollapsed={sidebarCollapsed}
      />
    </>
  );
}
