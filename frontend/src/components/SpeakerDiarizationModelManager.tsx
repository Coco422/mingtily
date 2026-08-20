import { useCallback, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { Download, Loader2, Trash2, Users } from 'lucide-react';
import { toast } from 'sonner';
import { Button } from './ui/button';
import { useTranslation } from 'react-i18next';
import { ModelAssetRow, type ModelAssetState } from './ModelAssetRow';
import { notifyModelAssetsChanged } from '@/lib/model-assets-events';

interface SpeakerModelStatus {
  id: string;
  status: 'available' | 'missing' | 'corrupt';
  size_mb: number;
  path: string;
  error?: string | null;
}

interface DownloadProgress {
  model_id: string;
  progress: number;
  downloaded_mb: number;
  total_mb: number;
  status: string;
}

interface SpeakerDiarizationModelManagerProps {
  serviceEnabled?: boolean;
  onOpenServices?: () => void;
}

export function SpeakerDiarizationModelManager({
  serviceEnabled = false,
  onOpenServices,
}: SpeakerDiarizationModelManagerProps) {
  const { t } = useTranslation('models');
  const [status, setStatus] = useState<SpeakerModelStatus | null>(null);
  const [progress, setProgress] = useState<number | null>(null);
  const [busy, setBusy] = useState(false);

  const refresh = useCallback(async () => {
    const next = await invoke<SpeakerModelStatus>('speaker_diarization_get_status');
    setStatus(next);
  }, []);

  useEffect(() => {
    refresh().catch(error => console.warn('Failed to read speaker model status:', error));
  }, [refresh]);

  useEffect(() => {
    const unlisteners: Array<() => void> = [];
    const setup = async () => {
      unlisteners.push(await listen<DownloadProgress>(
        'speaker-diarization-model-download-progress',
        ({ payload }) => setProgress(payload.progress)
      ));
      unlisteners.push(await listen('speaker-diarization-model-download-complete', async () => {
        setBusy(false);
        setProgress(null);
        await refresh();
        toast.success(t('speaker.ready'));
      }));
      unlisteners.push(await listen<{ error: string }>(
        'speaker-diarization-model-download-error',
        ({ payload }) => {
          setBusy(false);
          setProgress(null);
          toast.error(t('speaker.downloadFailed'), { description: payload.error });
        }
      ));
    };
    setup().catch(error => console.warn('Failed to listen for speaker model events:', error));
    return () => unlisteners.forEach(unlisten => unlisten());
  }, [refresh, t]);

  const download = async () => {
    setBusy(true);
    setProgress(0);
    try {
      await invoke('speaker_diarization_download_model');
      notifyModelAssetsChanged('speaker-diarization');
    } catch (error) {
      setBusy(false);
      setProgress(null);
      toast.error(t('speaker.downloadFailed'), { description: String(error) });
    }
  };

  const remove = async () => {
    setBusy(true);
    try {
      await invoke('speaker_diarization_delete_model');
      notifyModelAssetsChanged('speaker-diarization');
      await refresh();
      toast.success(t('speaker.removed'));
    } catch (error) {
      toast.error(t('speaker.removeFailed'), { description: String(error) });
    } finally {
      setBusy(false);
    }
  };

  const available = status?.status === 'available';
  const inUse = available && serviceEnabled;
  const state: ModelAssetState = progress !== null
    ? 'downloading'
    : available
      ? 'installed'
      : status?.status === 'corrupt'
        ? 'corrupt'
        : 'missing';
  const statusLabel = inUse
    ? t('status.inUse')
    : state === 'downloading'
      ? t('status.downloading')
      : state === 'installed'
        ? t('status.installed')
        : state === 'corrupt'
          ? t('status.needsRepair')
          : t('status.notInstalled');
  const actions = available ? (
    <Button
      variant="outline"
      size="sm"
      onClick={serviceEnabled ? onOpenServices : remove}
      disabled={busy}
      title={serviceEnabled ? t('delete.disableSpeakerFirst') : undefined}
    >
      {busy ? (
        <Loader2 className="mr-2 h-4 w-4 animate-spin" />
      ) : (
        <Trash2 className="mr-2 h-4 w-4" />
      )}
      {serviceEnabled ? t('status.inUse') : t('actions.delete')}
    </Button>
  ) : (
    <Button size="sm" onClick={download} disabled={busy}>
      {busy ? (
        <Loader2 className="mr-2 h-4 w-4 animate-spin" />
      ) : (
        <Download className="mr-2 h-4 w-4" />
      )}
      {status?.status === 'corrupt' ? t('actions.repair') : t('actions.download')}
    </Button>
  );

  return (
    <ModelAssetRow
      icon={Users}
      name={t('speaker.title')}
      provider="Sherpa ONNX"
      description={t('speaker.description', { size: Math.round(status?.size_mb ?? 44) })}
      state={state}
      statusLabel={statusLabel}
      inUse={inUse}
      progress={progress}
      progressLabel={
        progress !== null
          ? t('download.progress', { progress: Math.round(progress) })
          : undefined
      }
      actions={actions}
    />
  );
}
