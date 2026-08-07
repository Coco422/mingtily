import React, { useState, useEffect, useRef } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { motion, AnimatePresence } from 'framer-motion';
import { Download, RefreshCw, Trash2 } from 'lucide-react';
import { toast } from 'sonner';
import {
  ModelInfo,
  ModelStatus,
  getModelIcon,
  formatFileSize,
  getModelPerformanceBadge,
  isQuantizedModel,
  WhisperAPI
} from '../lib/whisper';
import { Accordion, AccordionContent, AccordionItem, AccordionTrigger } from '@/components/ui/accordion';
import { Button } from '@/components/ui/button';
import { ModelAssetRow, type ModelAssetState } from '@/components/ModelAssetRow';
import { useTranslation } from 'react-i18next';
import type { TFunction } from 'i18next';

const WHISPER_TAGLINE_KEYS: Record<string, string> = {
  'large-v3': 'largeV3',
  'large-v3-turbo': 'largeV3Turbo',
  medium: 'medium',
  small: 'small',
  base: 'base',
  tiny: 'tiny',
};

function localizedAccuracy(t: TFunction, accuracy: ModelInfo['accuracy']) {
  return t(`metrics.accuracy.${accuracy.toLowerCase()}`);
}

function localizedSpeed(t: TFunction, speed: ModelInfo['speed']) {
  const key = speed === 'Very Fast' ? 'veryFast' : speed.toLowerCase();
  return t(`metrics.speed.${key}`);
}

function localizedWhisperTagline(t: TFunction, modelName: string) {
  const baseName = modelName.replace(/-q[45]_[01]$/, '');
  const key = WHISPER_TAGLINE_KEYS[baseName];
  if (!key) return modelName;
  return `${t(`whisper.taglines.${key}`)}${isQuantizedModel(modelName) ? t('whisper.optimizedSuffix') : ''}`;
}

function localizedPrecision(t: TFunction, modelName: string) {
  if (modelName.includes('-q5_1')) return t('precision.balancedPlus');
  if (modelName.includes('-q5_0')) return t('precision.balanced');
  if (modelName.includes('-q4_0')) return t('precision.fast');
  if (!isQuantizedModel(modelName)) return t('precision.full');
  return t('precision.standard');
}

interface ModelManagerProps {
  selectedModel?: string;
  onModelSelect?: (modelName: string) => void;
  className?: string;
  autoSave?: boolean;
  mode?: 'select' | 'manage';
}

export function ModelManager({
  selectedModel,
  onModelSelect,
  className = '',
  autoSave = false,
  mode = 'select'
}: ModelManagerProps) {
  const { t } = useTranslation('models');
  const [models, setModels] = useState<ModelInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [initialized, setInitialized] = useState(false);
  const [downloadingModels, setDownloadingModels] = useState<Set<string>>(new Set());
  const [hasUserSelection, setHasUserSelection] = useState(false);

  // Refs for stable callbacks
  const onModelSelectRef = useRef(onModelSelect);
  const autoSaveRef = useRef(autoSave);

  // Progress throttle map to prevent rapid updates
  const progressThrottleRef = useRef<Map<string, { progress: number; timestamp: number }>>(new Map());

  // Update refs when props change
  useEffect(() => {
    onModelSelectRef.current = onModelSelect;
    autoSaveRef.current = autoSave;
  }, [onModelSelect, autoSave]);

  // Load persisted downloading state from localStorage
  const getPersistedDownloadingModels = (): Set<string> => {
    try {
      const saved = localStorage.getItem('downloading-models');
      return saved ? new Set<string>(JSON.parse(saved) as string[]) : new Set<string>();
    } catch {
      return new Set<string>();
    }
  };

  // Persist downloading state to localStorage
  const updateDownloadingModels = (updater: (prev: Set<string>) => Set<string>) => {
    setDownloadingModels(prev => {
      const newSet = updater(prev);
      localStorage.setItem('downloading-models', JSON.stringify(Array.from(newSet)));
      return newSet;
    });
  };

  // Initialize models
  useEffect(() => {
    if (initialized) return;

    const initializeModels = async () => {
      try {
        setLoading(true);
        await WhisperAPI.init();
        const modelList = await WhisperAPI.getAvailableModels();

        // Apply persisted downloading states
        const persistedDownloading = getPersistedDownloadingModels();
        const modelsWithDownloadState = modelList.map(model => {
          if (persistedDownloading.has(model.name) && model.status !== 'Available') {
            if (typeof model.status === 'object' && 'Corrupted' in model.status) {
              updateDownloadingModels(prev => {
                const newSet = new Set(prev);
                newSet.delete(model.name);
                return newSet;
              });
              return model;
            } else if (model.status === 'Missing') {
              updateDownloadingModels(prev => {
                const newSet = new Set(prev);
                newSet.delete(model.name);
                return newSet;
              });
              return model;
            } else {
              return { ...model, status: { Downloading: 0 } as ModelStatus };
            }
          }
          return model;
        });

        setModels(modelsWithDownloadState);
        setInitialized(true);
      } catch (err) {
        console.error('Failed to initialize Whisper:', err);
        setError(err instanceof Error ? err.message : 'Failed to load models');
        toast.error(t('errors.loadTranscriptionModels'), {
          description: err instanceof Error ? err.message : t('errors.unknown'),
          duration: 5000
        });
      } finally {
        setLoading(false);
      }
    };

    initializeModels();
  }, [initialized, selectedModel, onModelSelect]);

  // Set up event listeners for download progress
  useEffect(() => {
    let unlistenProgress: (() => void) | null = null;
    let unlistenComplete: (() => void) | null = null;
    let unlistenError: (() => void) | null = null;

    const setupListeners = async () => {
      console.log('[ModelManager] Setting up event listeners...');

      // Download progress with throttling
      unlistenProgress = await listen<{ modelName: string; progress: number }>(
        'model-download-progress',
        (event) => {
          const { modelName, progress } = event.payload;
          const now = Date.now();
          const throttleData = progressThrottleRef.current.get(modelName);

          // Throttle: only update if 300ms passed OR progress jumped by 5%+
          const shouldUpdate = !throttleData ||
            now - throttleData.timestamp > 300 ||
            Math.abs(progress - throttleData.progress) >= 5;

          if (shouldUpdate) {
            console.log(`[ModelManager] Progress update for ${modelName}: ${progress}%`);
            progressThrottleRef.current.set(modelName, { progress, timestamp: now });

            setModels(prevModels =>
              prevModels.map(model =>
                model.name === modelName
                  ? { ...model, status: { Downloading: progress } as ModelStatus }
                  : model
              )
            );
          }
        }
      );

      // Download complete
      unlistenComplete = await listen<{ modelName: string }>(
        'model-download-complete',
        (event) => {
          const { modelName } = event.payload;
          const model = models.find(m => m.name === modelName);
          const displayName = getDisplayName(modelName);

          setModels(prevModels =>
            prevModels.map(model =>
              model.name === modelName
                ? { ...model, status: 'Available' as ModelStatus }
                : model
            )
          );

          setDownloadingModels(prev => {
            const newSet = new Set(prev);
            newSet.delete(modelName);
            return newSet;
          });

          // Clean up throttle data
          progressThrottleRef.current.delete(modelName);

          toast.success(t('download.readyTitle', { icon: getModelIcon(model?.accuracy || 'Good'), model: displayName }), {
            description: t('download.readyDescription'),
            duration: 4000
          });

          // Auto-select after download using stable refs
          if (mode === 'select' && onModelSelectRef.current) {
            onModelSelectRef.current(modelName);
            if (autoSaveRef.current) {
              saveModelSelection(modelName);
            }
          }
        }
      );

      // Download error
      unlistenError = await listen<{ modelName: string; error: string }>(
        'model-download-error',
        (event) => {
          const { modelName, error } = event.payload;
          const displayName = getDisplayName(modelName);

          setModels(prevModels =>
            prevModels.map(model =>
              model.name === modelName
                ? { ...model, status: { Error: error } as ModelStatus }
                : model
            )
          );

          setDownloadingModels(prev => {
            const newSet = new Set(prev);
            newSet.delete(modelName);
            return newSet;
          });

          // Clean up throttle data
          progressThrottleRef.current.delete(modelName);

          toast.error(t('download.failed', { model: displayName }), {
            description: error,
            duration: 6000,
            action: {
              label: t('actions.retry'),
              onClick: () => downloadModel(modelName)
            }
          });
        }
      );
    };

    setupListeners();

    return () => {
      console.log('[ModelManager] Cleaning up event listeners...');
      if (unlistenProgress) unlistenProgress();
      if (unlistenComplete) unlistenComplete();
      if (unlistenError) unlistenError();
    };
  }, [mode, t]);

  const saveModelSelection = async (modelName: string) => {
    try {
      await invoke('api_save_transcript_config', {
        provider: 'localWhisper',
        model: modelName,
        apiKey: null
      });
    } catch (error) {
      console.error('Failed to save model selection:', error);
    }
  };

  const cancelDownload = async (modelName: string) => {
    const displayName = getDisplayName(modelName);

    try {
      await WhisperAPI.cancelDownload(modelName);

      updateDownloadingModels(prev => {
        const newSet = new Set(prev);
        newSet.delete(modelName);
        return newSet;
      });

      setModels(prevModels =>
        prevModels.map(model =>
          model.name === modelName
            ? { ...model, status: 'Missing' as ModelStatus }
            : model
        )
      );

      // Clean up throttle data
      progressThrottleRef.current.delete(modelName);

      toast.info(t('download.cancelled', { model: displayName }), {
        duration: 3000
      });
    } catch (err) {
      console.error('Failed to cancel download:', err);
      toast.error(t('download.cancelFailed'), {
        description: err instanceof Error ? err.message : t('errors.unknown'),
        duration: 4000
      });
    }
  };

  const downloadModel = async (modelName: string) => {
    if (downloadingModels.has(modelName)) return;

    const displayName = getDisplayName(modelName);

    try {
      updateDownloadingModels(prev => new Set([...prev, modelName]));

      setModels(prevModels =>
        prevModels.map(model =>
          model.name === modelName
            ? { ...model, status: { Downloading: 0 } as ModelStatus }
            : model
        )
      );

      toast.info(t('download.starting', { model: displayName }), {
        description: t('download.mayTakeMinutes'),
        duration: 5000
      });

      await WhisperAPI.downloadModel(modelName);
    } catch (err) {
      console.error('Download failed:', err);
      updateDownloadingModels(prev => {
        const newSet = new Set(prev);
        newSet.delete(modelName);
        return newSet;
      });

      const errorMessage = err instanceof Error ? err.message : t('download.genericFailed');
      setModels(prev =>
        prev.map(model =>
          model.name === modelName ? { ...model, status: { Error: errorMessage } } : model
        )
      );
    }
  };

  const selectModel = async (modelName: string) => {
    setHasUserSelection(true);

    if (onModelSelect) {
      onModelSelect(modelName);
    }

    if (autoSave) {
      await saveModelSelection(modelName);
    }

    const displayName = getDisplayName(modelName);
    toast.success(t('selection.switched', { model: displayName }), {
      duration: 3000
    });
  };

  const deleteModel = async (modelName: string) => {
    const displayName = getDisplayName(modelName);

    try {
      await WhisperAPI.deleteCorruptedModel(modelName);

      // Refresh models list
      const modelList = await WhisperAPI.getAvailableModels();
      setModels(modelList);

      toast.success(t('delete.deleted', { model: displayName }), {
        description: t('delete.freedSpace'),
        duration: 3000
      });

      // If deleted model was selected, clear selection
      if (selectedModel === modelName && onModelSelect) {
        onModelSelect('');
      }
    } catch (err) {
      console.error('Failed to delete model:', err);
      toast.error(t('delete.failed', { model: displayName }), {
        description: err instanceof Error ? err.message : t('delete.genericFailed'),
        duration: 4000
      });
    }
  };

  const getDisplayName = (modelName: string): string => {
    const modelNameMapping: { [key: string]: string } = {
      "small": "Small",
      "medium-q5_0": "Medium",
      "large-v3-q5_0": "Large V3 Compressed",
      "large-v3-turbo": "Large V3 Turbo",
      "large-v3": "Large V3"
    };

    const basicModelNames = ["small", "medium-q5_0", "large-v3-q5_0", "large-v3-turbo", "large-v3"];
    if (basicModelNames.includes(modelName)) {
      return modelNameMapping[modelName] || modelName;
    }
    return `Whisper ${modelName}`;
  };

  if (loading) {
    return (
      <div className={`space-y-3 ${className}`}>
        <div className="animate-pulse space-y-3">
          <div className="h-20 bg-gray-100 rounded-lg"></div>
          <div className="h-20 bg-gray-100 rounded-lg"></div>
          <div className="h-20 bg-gray-100 rounded-lg"></div>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className={`bg-red-50 border border-red-200 rounded-lg p-4 ${className}`}>
        <p className="text-sm text-red-800">{t('errors.loadModels')}</p>
        <p className="text-xs text-red-600 mt-1">{error}</p>
      </div>
    );
  }

  const featuredModelName = 'small';
  const featuredModels = models.filter((model) => model.name === featuredModelName);
  const advancedModels = models
    .filter((model) => model.name !== featuredModelName)
    .sort((left, right) => left.name.localeCompare(right.name));

  const renderManagedModel = (model: ModelInfo) => {
    const isAvailable = model.status === 'Available';
    const isError = typeof model.status === 'object' && 'Error' in model.status;
    const isCorrupted = typeof model.status === 'object' && 'Corrupted' in model.status;
    const downloadProgress =
      typeof model.status === 'object' && 'Downloading' in model.status
        ? model.status.Downloading
        : null;
    const isDownloading = downloadingModels.has(model.name) || downloadProgress !== null;
    const isSelected = selectedModel === model.name && isAvailable;
    const state: ModelAssetState = isDownloading
      ? 'downloading'
      : isAvailable
        ? 'installed'
        : isCorrupted
          ? 'corrupt'
          : isError
            ? 'error'
            : 'missing';
    const statusLabel = isSelected
      ? t('status.inUse')
      : state === 'downloading'
        ? t('status.downloading')
        : state === 'installed'
          ? t('status.installed')
          : state === 'corrupt'
            ? t('status.needsRepair')
            : state === 'error'
              ? t('status.error')
              : t('status.notInstalled');

    const actions = isDownloading ? (
      <Button variant="outline" size="sm" onClick={() => void cancelDownload(model.name)}>
        {t('actions.cancel')}
      </Button>
    ) : isAvailable ? (
      <Button
        variant="outline"
        size="sm"
        disabled={isSelected}
        onClick={() => void deleteModel(model.name)}
        title={isSelected ? t('delete.activeBlocked') : t('delete.freeSpace')}
      >
        <Trash2 className="h-4 w-4" />
        {isSelected ? t('status.inUse') : t('actions.delete')}
      </Button>
    ) : isCorrupted ? (
      <>
        <Button variant="outline" size="sm" onClick={() => void deleteModel(model.name)}>
          <Trash2 className="h-4 w-4" />
          {t('actions.delete')}
        </Button>
        <Button size="sm" onClick={() => void downloadModel(model.name)}>
          <RefreshCw className="h-4 w-4" />
          {t('actions.repair')}
        </Button>
      </>
    ) : (
      <Button size="sm" onClick={() => void downloadModel(model.name)}>
        {isError ? <RefreshCw className="h-4 w-4" /> : <Download className="h-4 w-4" />}
        {isError ? t('actions.retry') : t('actions.download')}
      </Button>
    );

    return (
      <ModelAssetRow
        key={model.name}
        name={getDisplayName(model.name)}
        provider="whisper.cpp"
        description={localizedWhisperTagline(t, model.name)}
        metadata={[
          formatFileSize(model.size_mb),
          t('specs.accuracy', { accuracy: localizedAccuracy(t, model.accuracy) }),
          t('specs.processing', { speed: localizedSpeed(t, model.speed) }),
          ...(isQuantizedModel(model.name) ? [localizedPrecision(t, model.name)] : []),
        ]}
        state={state}
        statusLabel={statusLabel}
        inUse={isSelected}
        progress={downloadProgress}
        progressLabel={
          downloadProgress !== null
            ? t('download.progress', { progress: Math.round(downloadProgress) })
            : undefined
        }
        actions={actions}
      />
    );
  };

  if (mode === 'manage') {
    return (
      <div className={`space-y-2 ${className}`}>
        {featuredModels.map(renderManagedModel)}
        {advancedModels.length > 0 && (
          <Accordion type="single" collapsible className="w-full">
            <AccordionItem value="advanced-whisper-models" className="border-0">
              <AccordionTrigger className="rounded-md px-1 py-2 text-xs font-semibold text-gray-600 hover:no-underline">
                {t('sections.advanced')}
              </AccordionTrigger>
              <AccordionContent>
                <div className="space-y-2 pt-1">{advancedModels.map(renderManagedModel)}</div>
              </AccordionContent>
            </AccordionItem>
          </Accordion>
        )}
      </div>
    );
  }

  return (
    <div className={`space-y-3 ${className}`}>
      {/* Basic Models */}
      <div className="space-y-3">
        {featuredModels.map((model) => {
          return (
            <ModelCard
              key={model.name}
              model={model}
              isSelected={selectedModel === model.name}
              isRecommended={false}
              onSelect={() => {
                if (mode === 'select' && model.status === 'Available') {
                  selectModel(model.name);
                }
              }}
              onDownload={() => downloadModel(model.name)}
              onCancel={() => cancelDownload(model.name)}
              onDelete={() => deleteModel(model.name)}
              isDownloading={downloadingModels.has(model.name)}
              displayName={getDisplayName(model.name)}
              canSelect={mode === 'select'}
            />
          );
        })}
      </div>

      {/* Advanced Models */}
      {advancedModels.length > 0 && (
        <Accordion type="single" collapsible className="w-full">
          <AccordionItem value="advanced-models">
            <AccordionTrigger>
              <span className='text-lg'>{t('sections.advanced')}</span>
            </AccordionTrigger>
            <AccordionContent>
              <div className="space-y-3 pt-4">
                {advancedModels.map((model) => (
                  <ModelCard
                    key={model.name}
                    model={model}
                    isSelected={selectedModel === model.name}
                    isRecommended={false}
                    onSelect={() => {
                      if (mode === 'select' && model.status === 'Available') {
                        selectModel(model.name);
                      }
                    }}
                    onDownload={() => downloadModel(model.name)}
                    onCancel={() => cancelDownload(model.name)}
                    onDelete={() => deleteModel(model.name)}
                    isDownloading={downloadingModels.has(model.name)}
                    displayName={getDisplayName(model.name)}
                    canSelect={mode === 'select'}
                  />
                ))}
              </div>
            </AccordionContent>
          </AccordionItem>
        </Accordion>
      )}

      {/* Helper text */}
      {mode === 'select' && selectedModel && (
        <motion.div
          initial={{ opacity: 0, y: -5 }}
          animate={{ opacity: 1, y: 0 }}
          className="text-xs text-gray-500 text-center pt-2"
        >
          {t('selection.usingForTranscription', { model: getDisplayName(selectedModel) })}
        </motion.div>
      )}
    </div>
  );
}

// Model Card Component
interface ModelCardProps {
  model: ModelInfo;
  isSelected: boolean;
  isRecommended: boolean;
  onSelect: () => void;
  onDownload: () => void;
  onCancel: () => void;
  onDelete: () => void;
  isDownloading: boolean;
  displayName: string;
  canSelect: boolean;
}

function ModelCard({
  model,
  isSelected,
  isRecommended,
  onSelect,
  onDownload,
  onCancel,
  onDelete,
  isDownloading,
  displayName,
  canSelect
}: ModelCardProps) {
  const { t } = useTranslation('models');
  const [isHovered, setIsHovered] = useState(false);

  const isAvailable = model.status === 'Available';
  const isMissing = model.status === 'Missing';
  const isError = typeof model.status === 'object' && 'Error' in model.status;
  const isCorrupted = typeof model.status === 'object' && 'Corrupted' in model.status;
  const downloadProgress =
    typeof model.status === 'object' && 'Downloading' in model.status
      ? model.status.Downloading
      : null;

  return (
    <motion.div
      initial={{ opacity: 0, y: 5 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.2 }}
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
      className={`
        relative rounded-lg border-2 transition-all
        ${isSelected && isAvailable
          ? 'border-blue-500 bg-blue-50'
          : isAvailable
            ? 'border-gray-200 hover:border-gray-300 bg-white'
            : 'border-gray-200 bg-gray-50'
        }
        ${isAvailable && canSelect ? 'cursor-pointer' : 'cursor-default'}
      `}
      onClick={() => {
        if (isAvailable && canSelect) onSelect();
      }}
    >
      {/* Recommended Badge */}
      {isRecommended && (
        <div className="absolute -top-2 -right-2 bg-blue-600 text-white text-xs px-2 py-0.5 rounded-full font-medium">
          {t('status.recommended')}
        </div>
      )}

      <div className="p-3">
        <div className="flex items-start justify-between mb-2">
          <div className="flex-1">
            {/* Model Name and Tagline */}
            <div className="flex items-center gap-2 flex-wrap mb-2">
              <span className="text-2xl">{getModelIcon(model.accuracy)}</span>
              <h3 className="font-semibold text-gray-900">{displayName}</h3>
              <span className="text-sm text-gray-500">•</span>
              <span className="text-sm text-gray-500">{localizedWhisperTagline(t, model.name)}</span>
              {isSelected && isAvailable && (
                <motion.span
                  initial={{ scale: 0 }}
                  animate={{ scale: 1 }}
                  className="bg-blue-600 text-white px-2 py-0.5 rounded-full text-xs font-medium flex items-center gap-1"
                >
                  ✓
                </motion.span>
              )}
              {isQuantizedModel(model.name) && (
                <span className={`px-2 py-0.5 rounded-full text-xs ${getModelPerformanceBadge(model.name).color === 'green'
                  ? 'bg-green-100 text-green-700'
                  : getModelPerformanceBadge(model.name).color === 'orange'
                    ? 'bg-orange-100 text-orange-700'
                    : 'bg-gray-100 text-gray-700'
                  }`}>
                  {localizedPrecision(t, model.name)}
                </span>
              )}
            </div>

            {/* Model Specs */}
            <div className="flex items-center space-x-4 text-sm text-gray-600 ml-9 mt-1.5">
              <span className="flex items-center space-x-1">
                <span>📦</span>
                <span>{formatFileSize(model.size_mb)}</span>
              </span>
              <span className="flex items-center space-x-1">
                <span>🎯</span>
                <span>{t('specs.accuracy', { accuracy: localizedAccuracy(t, model.accuracy) })}</span>
              </span>
              <span className="flex items-center space-x-1">
                <span>⚡</span>
                <span>{t('specs.processing', { speed: localizedSpeed(t, model.speed) })}</span>
              </span>
            </div>
          </div>

          {/* Status/Action */}
          <div className="ml-4 flex items-center gap-2">
            {isAvailable && (
              <>
                <div className="flex items-center gap-1.5 text-green-600">
                  <div className="w-2 h-2 bg-green-500 rounded-full"></div>
                  <span className="text-xs font-medium">{t('status.ready')}</span>
                </div>
                <AnimatePresence>
                  {isHovered && !isSelected && (
                    <motion.button
                      initial={{ opacity: 0, scale: 0.8 }}
                      animate={{ opacity: 1, scale: 1 }}
                      exit={{ opacity: 0, scale: 0.8 }}
                      transition={{ duration: 0.15 }}
                      onClick={(e) => {
                        e.stopPropagation();
                        onDelete();
                      }}
                      className="text-gray-400 hover:text-red-600 transition-colors p-1"
                      title={t('delete.freeSpace')}
                    >
                      <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                      </svg>
                    </motion.button>
                  )}
                </AnimatePresence>
              </>
            )}

            {isMissing && (
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  onDownload();
                }}
                className="bg-blue-600 text-white px-3 py-1.5 rounded-md text-sm font-medium hover:bg-blue-700 transition-colors"
              >
                {t('actions.download')}
              </button>
            )}

            {downloadProgress === null && isError && (
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  onDownload();
                }}
                className="bg-red-600 text-white px-3 py-1.5 rounded-md text-sm font-medium hover:bg-red-700 transition-colors"
              >
                {t('actions.retry')}
              </button>
            )}

            {isCorrupted && (
              <div className="flex gap-2">
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    onDelete();
                  }}
                  className="bg-orange-600 text-white px-3 py-1.5 rounded-md text-sm font-medium hover:bg-orange-700 transition-colors"
                >
                  {t('actions.delete')}
                </button>
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    onDownload();
                  }}
                  className="bg-blue-600 text-white px-3 py-1.5 rounded-md text-sm font-medium hover:bg-blue-700 transition-colors"
                >
                  {t('actions.redownload')}
                </button>
              </div>
            )}
          </div>
        </div>

        {/* Full-width Download Progress Bar - PROMINENT */}
        {downloadProgress !== null && (
          <motion.div
            initial={{ opacity: 0, height: 0 }}
            animate={{ opacity: 1, height: 'auto' }}
            exit={{ opacity: 0, height: 0 }}
            className="mt-3 pt-3 border-t border-gray-200"
          >
            <div className="flex items-center justify-between mb-2">
              <div className="flex items-center gap-2">
                <span className="text-sm font-medium text-blue-600">{t('status.downloading')}</span>
                <span className="text-sm font-semibold text-blue-600">{Math.round(downloadProgress)}%</span>
              </div>
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  onCancel();
                }}
                className="text-xs text-gray-600 hover:text-red-600 font-medium transition-colors px-2 py-1 rounded hover:bg-red-50"
                title={t('actions.cancelDownload')}
              >
                {t('actions.cancel')}
              </button>
            </div>
            <div className="w-full h-2 bg-gray-200 rounded-full overflow-hidden">
              <motion.div
                className="h-full bg-gradient-to-r from-blue-500 to-blue-600 rounded-full"
                initial={{ width: 0 }}
                animate={{ width: `${downloadProgress}%` }}
                transition={{ duration: 0.3, ease: 'easeOut' }}
              />
            </div>
            <p className="text-xs text-gray-500 mt-1">
              {model.size_mb ? (
                <>
                  {formatFileSize(model.size_mb * downloadProgress / 100)} / {formatFileSize(model.size_mb)}
                </>
              ) : (
                t('status.downloading')
              )}
            </p>
          </motion.div>
        )}
      </div>
    </motion.div>
  );
}
