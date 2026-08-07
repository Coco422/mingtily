'use client';

import { useCallback, useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { Download, FileCog, FilePlus2, Loader2, Trash2 } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';
import { Button } from '@/components/ui/button';
import { ModelAssetRow, type ModelAssetState } from '@/components/ModelAssetRow';
import {
  SherpaAsrAPI,
  type HomophoneReplacerStatus,
  type SherpaAsrDownloadProgress,
} from '@/lib/sherpa-asr';

function formatSize(bytes: number) {
  return bytes >= 1024 * 1024
    ? `${(bytes / 1024 / 1024).toFixed(1)} MiB`
    : `${Math.max(1, Math.round(bytes / 1024))} KiB`;
}

export function HomophoneReplacerManager() {
  const { t } = useTranslation('models');
  const [status, setStatus] = useState<HomophoneReplacerStatus | null>(null);
  const [progress, setProgress] = useState<number | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  const refresh = useCallback(async () => {
    setStatus(await SherpaAsrAPI.getHomophoneStatus());
  }, []);

  useEffect(() => {
    refresh()
      .catch((error) =>
        toast.error(t('homophone.loadFailed'), { description: String(error) })
      )
      .finally(() => setLoading(false));
  }, [refresh, t]);

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void listen<SherpaAsrDownloadProgress>(
      'sherpa-homophone-lexicon-download-progress',
      ({ payload }) => setProgress(payload.progress)
    ).then((dispose) => {
      if (disposed) dispose();
      else unlisten = dispose;
    });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  const download = async () => {
    setBusy('lexicon');
    setProgress(0);
    try {
      await SherpaAsrAPI.downloadHomophoneLexicon();
      await refresh();
      toast.success(t('homophone.lexiconReady'));
    } catch (error) {
      toast.error(t('homophone.downloadFailed'), { description: String(error) });
    } finally {
      setBusy(null);
      setProgress(null);
    }
  };

  const removeLexicon = async () => {
    setBusy('lexicon');
    try {
      await SherpaAsrAPI.deleteHomophoneLexicon();
      await refresh();
      toast.success(t('homophone.lexiconRemoved'));
    } catch (error) {
      toast.error(t('homophone.removeFailed'), { description: String(error) });
    } finally {
      setBusy(null);
    }
  };

  const importRules = async () => {
    setBusy('import');
    try {
      await SherpaAsrAPI.importHomophoneRules();
      await refresh();
      toast.success(t('homophone.rulesImported'));
    } catch (error) {
      toast.error(t('homophone.importFailed'), { description: String(error) });
    } finally {
      setBusy(null);
    }
  };

  const removeRule = async (ruleId: string) => {
    setBusy(ruleId);
    try {
      await SherpaAsrAPI.deleteHomophoneRule(ruleId);
      await refresh();
      toast.success(t('homophone.ruleRemoved'));
    } catch (error) {
      toast.error(t('homophone.ruleRemoveFailed'), { description: String(error) });
    } finally {
      setBusy(null);
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

  return (
    <div className="space-y-3">
      <ModelAssetRow
        icon={FileCog}
        name={t('homophone.lexiconTitle')}
        provider="Sherpa ONNX"
        description={t('homophone.lexiconDescription')}
        metadata={[
          formatSize(status?.download_size ?? 1_366_297),
          status?.license ?? 'Apache-2.0',
        ]}
        state={state}
        statusLabel={statusLabel}
        badges={[{ label: 'Beta', tone: 'warning' }]}
        progress={progress}
        progressLabel={
          progress !== null
            ? t('download.progress', { progress: Math.round(progress) })
            : undefined
        }
        actions={available ? (
          <Button
            variant="outline"
            size="sm"
            disabled={busy !== null}
            onClick={() => void removeLexicon()}
          >
            {busy === 'lexicon' ? (
              <Loader2 className="mr-2 h-4 w-4 animate-spin" />
            ) : (
              <Trash2 className="mr-2 h-4 w-4" />
            )}
            {t('actions.delete')}
          </Button>
        ) : (
          <Button size="sm" disabled={busy !== null} onClick={() => void download()}>
            {busy === 'lexicon' ? (
              <Loader2 className="mr-2 h-4 w-4 animate-spin" />
            ) : (
              <Download className="mr-2 h-4 w-4" />
            )}
            {status?.status === 'corrupt' ? t('actions.repair') : t('actions.download')}
          </Button>
        )}
      />

      <div className="rounded-md border border-black/[0.08] bg-gray-50/50 p-4">
        <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
          <div>
            <h3 className="text-sm font-semibold text-gray-900">
              {t('homophone.rulesTitle')}
            </h3>
            <p className="mt-1 text-xs leading-5 text-gray-600">
              {t('homophone.rulesDescription')}
            </p>
          </div>
          <Button
            variant="outline"
            size="sm"
            disabled={busy !== null}
            onClick={() => void importRules()}
          >
            {busy === 'import' ? (
              <Loader2 className="mr-2 h-4 w-4 animate-spin" />
            ) : (
              <FilePlus2 className="mr-2 h-4 w-4" />
            )}
            {t('homophone.importRules')}
          </Button>
        </div>

        {status?.rules.length ? (
          <div className="mt-3 space-y-2">
            {status.rules.map((rule) => (
              <div
                key={rule.id}
                className="flex items-center justify-between gap-3 rounded-md border bg-white px-3 py-2"
              >
                <div className="min-w-0">
                  <div className="truncate text-sm font-medium text-gray-800">{rule.name}</div>
                  <div className="text-xs text-gray-500">{formatSize(rule.size)}</div>
                </div>
                <Button
                  variant="ghost"
                  size="sm"
                  disabled={busy !== null}
                  onClick={() => void removeRule(rule.id)}
                  aria-label={t('homophone.deleteRule', { name: rule.name })}
                >
                  {busy === rule.id ? (
                    <Loader2 className="h-4 w-4 animate-spin" />
                  ) : (
                    <Trash2 className="h-4 w-4" />
                  )}
                </Button>
              </div>
            ))}
          </div>
        ) : (
          <p className="mt-3 text-xs text-gray-500">{t('homophone.noRules')}</p>
        )}
      </div>
    </div>
  );
}
