'use client';

import { useCallback, useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import {
  AudioLines,
  Download,
  FileArchive,
  FolderOpen,
  Loader2,
  Trash2,
} from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';
import { Button } from '@/components/ui/button';
import { ModelAssetRow, type ModelAssetState } from '@/components/ModelAssetRow';
import {
  SherpaAsrAPI,
  SherpaAsrDownloadProgress,
  SherpaAsrModelStatus,
} from '@/lib/sherpa-asr';

interface SherpaAsrModelManagerProps {
  selectedModel?: string;
  additionalSelectedModels?: string[];
  onOpenServices?: () => void;
  mode?: 'manage' | 'select';
  onModelSelect?: (modelId: string) => void;
}

function formatSize(bytes: number) {
  const mib = bytes / 1024 / 1024;
  return mib >= 1024 ? `${(mib / 1024).toFixed(1)} GiB` : `${Math.round(mib)} MiB`;
}

export function SherpaAsrModelManager({
  selectedModel,
  additionalSelectedModels = [],
  onOpenServices,
  mode = 'manage',
  onModelSelect,
}: SherpaAsrModelManagerProps) {
  const { t } = useTranslation('models');
  const [models, setModels] = useState<SherpaAsrModelStatus[]>([]);
  const [busyModels, setBusyModels] = useState<Set<string>>(new Set());
  const [progress, setProgress] = useState<Record<string, number>>({});
  const [loading, setLoading] = useState(true);
  const [importing, setImporting] = useState<'archive' | 'directory' | null>(null);

  const refresh = useCallback(async () => {
    const nextModels = await SherpaAsrAPI.listModels();
    setModels(
      [...nextModels].sort((left, right) => {
        if (left.recommended !== right.recommended) return left.recommended ? -1 : 1;
        if (left.beta !== right.beta) return left.beta ? 1 : -1;
        return left.name.localeCompare(right.name);
      })
    );
  }, []);

  useEffect(() => {
    refresh()
      .catch((error) =>
        toast.error(t('sherpa.errors.load'), { description: String(error) })
      )
      .finally(() => setLoading(false));
  }, [refresh, t]);

  useEffect(() => {
    const unlisteners: Array<() => void> = [];
    const setup = async () => {
      unlisteners.push(
        await listen<SherpaAsrDownloadProgress>(
          'sherpa-asr-model-download-progress',
          ({ payload }) =>
            setProgress((current) => ({
              ...current,
              [payload.model_id]: payload.progress,
            }))
        )
      );
      unlisteners.push(
        await listen<{ model_id: string }>(
          'sherpa-asr-model-download-complete',
          ({ payload }) => {
            setBusyModels((current) => {
              const next = new Set(current);
              next.delete(payload.model_id);
              return next;
            });
            setProgress((current) => {
              const next = { ...current };
              delete next[payload.model_id];
              return next;
            });
            void refresh();
          }
        )
      );
      unlisteners.push(
        await listen<{ model_id: string; error: string }>(
          'sherpa-asr-model-download-error',
          ({ payload }) => {
            setBusyModels((current) => {
              const next = new Set(current);
              next.delete(payload.model_id);
              return next;
            });
            toast.error(t('sherpa.errors.download'), {
              description: payload.error,
            });
          }
        )
      );
    };
    void setup();
    return () => unlisteners.forEach((unlisten) => unlisten());
  }, [refresh, t]);

  const download = async (model: SherpaAsrModelStatus) => {
    setBusyModels((current) => new Set(current).add(model.id));
    setProgress((current) => ({ ...current, [model.id]: 0 }));
    try {
      await SherpaAsrAPI.downloadModel(model.id);
      await refresh();
      toast.success(t('sherpa.downloaded', { model: model.name }));
    } catch (error) {
      toast.error(t('sherpa.errors.download'), { description: String(error) });
    } finally {
      setBusyModels((current) => {
        const next = new Set(current);
        next.delete(model.id);
        return next;
      });
      setProgress((current) => {
        const next = { ...current };
        delete next[model.id];
        return next;
      });
    }
  };

  const remove = async (model: SherpaAsrModelStatus) => {
    if (selectedModel === model.id) {
      onOpenServices?.();
      return;
    }
    setBusyModels((current) => new Set(current).add(model.id));
    try {
      await SherpaAsrAPI.deleteModel(model.id);
      await refresh();
      toast.success(t('sherpa.removed', { model: model.name }));
    } catch (error) {
      toast.error(t('sherpa.errors.delete'), { description: String(error) });
    } finally {
      setBusyModels((current) => {
        const next = new Set(current);
        next.delete(model.id);
        return next;
      });
    }
  };

  const importOffline = async (kind: 'archive' | 'directory') => {
    setImporting(kind);
    try {
      const imported =
        kind === 'archive'
          ? await SherpaAsrAPI.importModelArchive()
          : await SherpaAsrAPI.importModelDirectory();
      if (!imported) return;
      const modelName =
        models.find((model) => model.id === imported.model_id)?.name ?? imported.model_id;
      await refresh();
      toast.success(t('sherpa.offlineImport.success', { model: modelName }));
    } catch (error) {
      toast.error(t('sherpa.offlineImport.failed'), { description: String(error) });
    } finally {
      setImporting(null);
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

  return (
    <div className="space-y-2">
      {mode === 'manage' && (
        <div className="rounded-lg border bg-muted/30 p-4">
          <div className="text-sm font-medium">{t('sherpa.offlineImport.title')}</div>
          <p className="mt-1 text-sm text-muted-foreground">
            {t('sherpa.offlineImport.description')}
          </p>
          <div className="mt-3 flex flex-wrap gap-2">
            <Button
              variant="outline"
              size="sm"
              disabled={importing !== null}
              onClick={() => void importOffline('archive')}
            >
              {importing === 'archive' ? (
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              ) : (
                <FileArchive className="mr-2 h-4 w-4" />
              )}
              {t('sherpa.offlineImport.archive')}
            </Button>
            <Button
              variant="outline"
              size="sm"
              disabled={importing !== null}
              onClick={() => void importOffline('directory')}
            >
              {importing === 'directory' ? (
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              ) : (
                <FolderOpen className="mr-2 h-4 w-4" />
              )}
              {t('sherpa.offlineImport.directory')}
            </Button>
          </div>
          <p className="mt-2 text-xs text-muted-foreground">
            {t('sherpa.offlineImport.hint')}
          </p>
        </div>
      )}
      {models.map((model) => {
        const installed = model.status === 'available';
        const inUse = installed && (
          selectedModel === model.id || additionalSelectedModels.includes(model.id)
        );
        const busy = busyModels.has(model.id);
        const downloadProgress = progress[model.id];
        const state: ModelAssetState =
          downloadProgress !== undefined
            ? 'downloading'
            : installed
              ? 'installed'
              : model.status === 'corrupt'
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
        const badges = [
          ...(model.recommended
            ? [{ label: t('status.recommended'), tone: 'accent' as const }]
            : []),
          ...(model.streaming_mode === 'continuous'
            ? [{ label: t('status.streaming'), tone: 'accent' as const }]
            : []),
          ...(model.beta ? [{ label: 'Beta', tone: 'warning' as const }] : []),
        ];
        const actions = installed && mode === 'select' ? (
          <Button
            size="sm"
            disabled={busy || inUse}
            onClick={() => onModelSelect?.(model.id)}
          >
            {inUse ? t('status.inUse') : t('actions.use')}
          </Button>
        ) : installed ? (
          <Button
            variant="outline"
            size="sm"
            disabled={busy || inUse}
            onClick={() => void remove(model)}
            title={inUse ? t('delete.activeBlocked') : undefined}
          >
            {busy ? (
              <Loader2 className="mr-2 h-4 w-4 animate-spin" />
            ) : (
              <Trash2 className="mr-2 h-4 w-4" />
            )}
            {inUse ? t('status.inUse') : t('actions.delete')}
          </Button>
        ) : (
          <Button size="sm" disabled={busy} onClick={() => void download(model)}>
            {busy ? (
              <Loader2 className="mr-2 h-4 w-4 animate-spin" />
            ) : (
              <Download className="mr-2 h-4 w-4" />
            )}
            {model.status === 'corrupt' ? t('actions.repair') : t('actions.download')}
          </Button>
        );

        return (
          <ModelAssetRow
            key={model.id}
            icon={AudioLines}
            name={model.name}
            provider="Sherpa ONNX"
            description={t(`sherpa.models.${model.id}.description`)}
            metadata={[formatSize(model.download_size), model.license]}
            state={state}
            statusLabel={statusLabel}
            inUse={inUse}
            badges={badges}
            progress={downloadProgress}
            progressLabel={
              downloadProgress !== undefined
                ? t('download.progress', { progress: Math.round(downloadProgress) })
                : undefined
            }
            actions={actions}
          />
        );
      })}
    </div>
  );
}
