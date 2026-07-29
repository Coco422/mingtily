'use client';

import { useCallback, useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { Download, Loader2, MessageSquareText, Trash2 } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';
import { Button } from '@/components/ui/button';
import { ModelAssetRow, type ModelAssetState } from '@/components/ModelAssetRow';
import {
  PunctuationAPI,
  type PunctuationDownloadProgress,
  type PunctuationModelStatus,
} from '@/lib/punctuation';

function formatSize(bytes: number) {
  return `${Math.round(bytes / 1024 / 1024)} MiB`;
}

export function PunctuationModelManager() {
  const { t } = useTranslation('models');
  const [status, setStatus] = useState<PunctuationModelStatus | null>(null);
  const [progress, setProgress] = useState<number | null>(null);
  const [busy, setBusy] = useState(false);
  const [loading, setLoading] = useState(true);

  const refresh = useCallback(async () => {
    setStatus(await PunctuationAPI.getStatus());
  }, []);

  useEffect(() => {
    refresh()
      .catch((error) =>
        toast.error(t('punctuation.loadFailed'), { description: String(error) })
      )
      .finally(() => setLoading(false));
  }, [refresh, t]);

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;

    void listen<PunctuationDownloadProgress>(
      'punctuation-model-download-progress',
      ({ payload }) => setProgress(payload.progress)
    ).then((dispose) => {
      if (disposed) {
        dispose();
      } else {
        unlisten = dispose;
      }
    });

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  const download = async () => {
    setBusy(true);
    setProgress(0);
    try {
      await PunctuationAPI.downloadModel();
      await refresh();
      toast.success(t('punctuation.ready'));
    } catch (error) {
      toast.error(t('punctuation.downloadFailed'), { description: String(error) });
    } finally {
      setBusy(false);
      setProgress(null);
    }
  };

  const remove = async () => {
    setBusy(true);
    try {
      await PunctuationAPI.deleteModel();
      await refresh();
      toast.success(t('punctuation.removed'));
    } catch (error) {
      toast.error(t('punctuation.removeFailed'), { description: String(error) });
    } finally {
      setBusy(false);
    }
  };

  if (loading) {
    return (
      <div className="flex items-center gap-2 text-sm text-muted-foreground">
        <Loader2 className="h-4 w-4 animate-spin" />
        {t('status.loading')}
      </div>
    );
  }

  const available = status?.status === 'available';
  const state: ModelAssetState = progress !== null
    ? 'downloading'
    : available
      ? 'installed'
      : status?.status === 'corrupt'
        ? 'corrupt'
        : 'missing';
  const statusLabel = state === 'downloading'
    ? t('status.downloading')
    : state === 'installed'
      ? t('status.installed')
      : state === 'corrupt'
        ? t('status.needsRepair')
        : t('status.notInstalled');
  const actions = available ? (
    <Button variant="outline" size="sm" onClick={() => void remove()} disabled={busy}>
      {busy ? (
        <Loader2 className="mr-2 h-4 w-4 animate-spin" />
      ) : (
        <Trash2 className="mr-2 h-4 w-4" />
      )}
      {t('actions.delete')}
    </Button>
  ) : (
    <Button size="sm" onClick={() => void download()} disabled={busy}>
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
      icon={MessageSquareText}
      name={t('punctuation.title')}
      provider="Sherpa ONNX"
      description={t('punctuation.description')}
      metadata={[
        formatSize(status?.download_size ?? 64_717_756),
        status?.license ?? 'Apache-2.0',
      ]}
      state={state}
      statusLabel={statusLabel}
      badges={[{ label: t('status.recommended'), tone: 'accent' }]}
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
