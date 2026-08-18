'use client';

import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { toast } from 'sonner';
import { ModelConfig, ModelSettingsModal } from '@/components/ModelSettingsModal';
import { SummaryLanguageSettings } from '@/components/SummaryLanguageSettings';
import { Switch } from './ui/switch';
import { Input } from './ui/input';
import { Button } from './ui/button';
import { useConfig } from '@/contexts/ConfigContext';
import { useTranslation } from 'react-i18next';

interface SummaryModelSettingsProps {
  refetchTrigger?: number; // Change this to trigger refetch
  showAssetManagement?: boolean;
}

interface SummaryRuntimeConfig {
  requestTimeoutSecs: number;
}

export function SummaryModelSettings({ refetchTrigger, showAssetManagement = true }: SummaryModelSettingsProps) {
  const { t } = useTranslation('settings');
  const [modelConfig, setModelConfig] = useState<ModelConfig>({
    provider: 'ollama',
    model: 'llama3.2:latest',
    whisperModel: 'large-v3',
    apiKey: null,
    ollamaEndpoint: null
  });

  const { isAutoSummary, toggleIsAutoSummary } = useConfig();
  const [requestTimeoutMinutes, setRequestTimeoutMinutes] = useState('30');
  const [isTimeoutSaving, setIsTimeoutSaving] = useState(false);

  useEffect(() => {
    invoke<SummaryRuntimeConfig>('api_get_summary_runtime_config')
      .then((config) => {
        setRequestTimeoutMinutes(String(config.requestTimeoutSecs / 60));
      })
      .catch((error) => {
        console.error('Failed to load summary runtime config:', error);
        toast.error(t('services.summary.requestTimeoutLoadFailed'));
      });
  }, [t]);

  // Reusable fetch function
  const fetchModelConfig = useCallback(async () => {
    try {
      const data = await invoke('api_get_model_config') as any;
      if (data && data.provider !== null) {
        // Fetch API key if not included and provider requires it
        if (data.provider !== 'ollama' && data.provider !== 'builtin-ai' && !data.apiKey) {
          try {
            const apiKeyData = await invoke('api_get_api_key', {
              provider: data.provider
            }) as string;
            data.apiKey = apiKeyData;
          } catch (err) {
            console.error('Failed to fetch API key:', err);
          }
        }
        // Fetch Custom OpenAI config if that's the active provider
        if (data.provider === 'custom-openai') {
          try {
            const customConfig = (await invoke('api_get_custom_openai_config')) as any;
            if (customConfig) {
              data.customOpenAIDisplayName = customConfig.displayName || null;
              data.customOpenAIEndpoint = customConfig.endpoint || null;
              data.customOpenAIModel = customConfig.model || null;
              data.customOpenAIApiKey = customConfig.apiKey || null;
              data.maxTokens = customConfig.maxTokens || null;
              data.temperature = customConfig.temperature || null;
              data.topP = customConfig.topP || null;
              // For custom-openai, model field should match customOpenAIModel
              data.model = customConfig.model || data.model;
            }
          } catch (err) {
            console.error('Failed to fetch custom OpenAI config:', err);
          }
        }
        setModelConfig(data);
      }
    } catch (error) {
      console.error('Failed to fetch model config:', error);
      toast.error(t('services.summary.loadFailed'));
    }
  }, []);

  // Fetch on mount
  useEffect(() => {
    fetchModelConfig();
  }, [fetchModelConfig]);

  // Refetch when trigger changes (optional external control)
  useEffect(() => {
    if (refetchTrigger !== undefined && refetchTrigger > 0) {
      fetchModelConfig();
    }
  }, [refetchTrigger, fetchModelConfig]);

  // Listen for model config updates from other components
  useEffect(() => {
    const setupListener = async () => {
      const { listen } = await import('@tauri-apps/api/event');
      const unlisten = await listen<ModelConfig>('model-config-updated', (event) => {
        console.log('SummaryModelSettings received model-config-updated event:', event.payload);
        setModelConfig(event.payload);
      });

      return unlisten;
    };

    let cleanup: (() => void) | undefined;
    setupListener().then(fn => cleanup = fn);

    return () => {
      cleanup?.();
    };
  }, []);

  // Save handler
  const handleSaveModelConfig = async (config: ModelConfig) => {
    try {
      await invoke('api_save_model_config', {
        provider: config.provider,
        model: config.model,
        whisperModel: config.whisperModel,
        apiKey: config.apiKey,
        ollamaEndpoint: config.ollamaEndpoint,
      });

      setModelConfig(config);

      // Emit event to sync other components
      const { emit } = await import('@tauri-apps/api/event');
      await emit('model-config-updated', config);

      toast.success(t('services.summary.saved'));
    } catch (error) {
      console.error('Error saving model config:', error);
      toast.error(t('services.summary.saveFailed'));
    }
  };

  const timeoutMinutes = Number(requestTimeoutMinutes);
  const timeoutIsValid = Number.isInteger(timeoutMinutes)
    && timeoutMinutes >= 5
    && timeoutMinutes <= 1440;

  const handleSaveRequestTimeout = async () => {
    if (!timeoutIsValid) {
      toast.error(t('services.summary.requestTimeoutInvalid'));
      return;
    }

    setIsTimeoutSaving(true);
    try {
      const saved = await invoke<SummaryRuntimeConfig>('api_save_summary_runtime_config', {
        requestTimeoutSecs: timeoutMinutes * 60,
      });
      setRequestTimeoutMinutes(String(saved.requestTimeoutSecs / 60));
      toast.success(t('services.summary.requestTimeoutSaved'));
    } catch (error) {
      console.error('Failed to save summary runtime config:', error);
      toast.error(t('services.summary.requestTimeoutSaveFailed'));
    } finally {
      setIsTimeoutSaving(false);
    }
  };

  return (
    <div className='flex flex-col gap-4'>
      <div className="bg-white rounded-lg border border-gray-200 p-6 shadow-sm">
        <div className="flex items-center justify-between">
          <div>
            <h3 className="text-lg font-semibold text-gray-900 mb-2">{t('services.summary.autoSummary')}</h3>
            <p className="text-sm text-gray-600">{t('services.summary.autoSummaryDescription')}</p>
          </div>
          <Switch checked={isAutoSummary} onCheckedChange={toggleIsAutoSummary} />
        </div>
      </div>

      <SummaryLanguageSettings />

      <div className="bg-white rounded-lg border border-gray-200 p-6 shadow-sm">
        <h3 className="text-lg font-semibold mb-4">{t('services.summary.configuration')}</h3>
        <p className="text-sm text-gray-600 mb-6">
          {t('services.summary.configurationDescription')}
        </p>

        <ModelSettingsModal
          modelConfig={modelConfig}
          setModelConfig={setModelConfig}
          onSave={handleSaveModelConfig}
          skipInitialFetch={true}
          showAssetManagement={showAssetManagement}
        />

        <div className="mt-6 border-t border-gray-200 pt-6">
          <div className="max-w-2xl">
            <label htmlFor="summary-request-timeout" className="text-sm font-medium text-gray-900">
              {t('services.summary.requestTimeout')}
            </label>
            <p className="mt-1 text-sm text-gray-600">
              {t('services.summary.requestTimeoutDescription')}
            </p>
            <div className="mt-3 flex max-w-md items-center gap-3">
              <Input
                id="summary-request-timeout"
                type="number"
                min={5}
                max={1440}
                step={5}
                value={requestTimeoutMinutes}
                onChange={(event) => setRequestTimeoutMinutes(event.target.value)}
                aria-invalid={!timeoutIsValid}
                className="w-32"
              />
              <span className="text-sm text-gray-600">
                {t('services.summary.requestTimeoutMinutes')}
              </span>
              <Button
                type="button"
                variant="outline"
                onClick={handleSaveRequestTimeout}
                disabled={!timeoutIsValid || isTimeoutSaving}
              >
                {isTimeoutSaving
                  ? t('services.summary.requestTimeoutSaving')
                  : t('services.summary.requestTimeoutSave')}
              </Button>
            </div>
            {!timeoutIsValid && (
              <p className="mt-2 text-sm text-red-600">
                {t('services.summary.requestTimeoutInvalid')}
              </p>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
