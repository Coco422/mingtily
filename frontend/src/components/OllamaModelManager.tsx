'use client';

import { useCallback, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Download, Loader2, RefreshCw, Trash2 } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';

interface OllamaModel {
  name: string;
  id: string;
  size: string;
  modified: string;
}

interface OllamaModelManagerProps {
  endpoint?: string | null;
  selectedModel?: string;
}

export function OllamaModelManager({ endpoint, selectedModel }: OllamaModelManagerProps) {
  const { t } = useTranslation('models');
  const [models, setModels] = useState<OllamaModel[]>([]);
  const [modelName, setModelName] = useState('gemma3:1b');
  const [loading, setLoading] = useState(false);
  const [pulling, setPulling] = useState(false);
  const [removing, setRemoving] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const result = await invoke<OllamaModel[]>('get_ollama_models', {
        endpoint: endpoint?.trim() || null,
      });
      setModels(result);
    } catch (nextError) {
      const message = nextError instanceof Error ? nextError.message : String(nextError);
      setError(message);
    } finally {
      setLoading(false);
    }
  }, [endpoint]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const pull = async () => {
    const nextModel = modelName.trim();
    if (!nextModel || pulling) return;
    setPulling(true);
    try {
      await invoke('pull_ollama_model', {
        modelName: nextModel,
        endpoint: endpoint?.trim() || null,
      });
      toast.success(t('ollama.pullComplete', { model: nextModel }));
      await refresh();
    } catch (nextError) {
      toast.error(t('ollama.pullFailed'), { description: String(nextError) });
    } finally {
      setPulling(false);
    }
  };

  const remove = async (name: string) => {
    if (name === selectedModel) return;
    setRemoving(name);
    try {
      await invoke('delete_ollama_model', {
        modelName: name,
        endpoint: endpoint?.trim() || null,
      });
      toast.success(t('ollama.deleted', { model: name }));
      await refresh();
    } catch (nextError) {
      toast.error(t('ollama.deleteFailed'), { description: String(nextError) });
    } finally {
      setRemoving(null);
    }
  };

  return (
    <div className="space-y-4">
      <div className="flex flex-col gap-2 sm:flex-row">
        <Input
          value={modelName}
          onChange={(event) => setModelName(event.target.value)}
          placeholder={t('ollama.modelPlaceholder')}
          aria-label={t('ollama.modelPlaceholder')}
        />
        <Button onClick={pull} disabled={pulling || !modelName.trim()} className="sm:min-w-32">
          {pulling ? <Loader2 className="mr-2 h-4 w-4 animate-spin" /> : <Download className="mr-2 h-4 w-4" />}
          {t('ollama.pull')}
        </Button>
        <Button variant="outline" onClick={refresh} disabled={loading} className="sm:min-w-28">
          <RefreshCw className={`mr-2 h-4 w-4 ${loading ? 'animate-spin' : ''}`} />
          {t('actions.refresh')}
        </Button>
      </div>

      <p className="text-xs text-muted-foreground">
        {t('ollama.endpoint', { endpoint: endpoint?.trim() || 'http://localhost:11434' })}
      </p>

      {error && (
        <div className="rounded-md border border-amber-200 bg-amber-50 p-3 text-sm text-amber-800">
          {t('ollama.unavailable')}: {error}
        </div>
      )}

      {!error && !loading && models.length === 0 && (
        <p className="rounded-md border bg-muted/30 p-4 text-sm text-muted-foreground">
          {t('ollama.empty')}
        </p>
      )}

      <div className="grid gap-3">
        {models.map((model) => {
          const inUse = model.name === selectedModel;
          return (
            <div key={model.id || model.name} className="flex items-center justify-between gap-4 rounded-lg border p-4">
              <div className="min-w-0">
                <div className="flex flex-wrap items-center gap-2">
                  <span className="break-all font-medium">{model.name}</span>
                  {inUse && (
                    <span className="rounded bg-blue-100 px-2 py-0.5 text-xs font-medium text-blue-700">
                      {t('status.inUse')}
                    </span>
                  )}
                </div>
                <p className="mt-1 text-xs text-muted-foreground">{model.size}</p>
              </div>
              <Button
                variant="ghost"
                size="icon"
                disabled={inUse || removing === model.name}
                onClick={() => void remove(model.name)}
                title={inUse ? t('delete.activeBlocked') : t('actions.delete')}
              >
                {removing === model.name ? <Loader2 className="h-4 w-4 animate-spin" /> : <Trash2 className="h-4 w-4" />}
              </Button>
            </div>
          );
        })}
      </div>
    </div>
  );
}
