import React, { useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { Mic, Sparkles, Check, Loader2, Download } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { OnboardingContainer } from '../OnboardingContainer';
import { useOnboarding } from '@/contexts/OnboardingContext';
import { toast } from 'sonner';
import { motion, AnimatePresence } from 'framer-motion';
import { getSummaryModelSizeLabel, getSummaryModelSizeMb } from '@/lib/onboarding-summary-model';
import { useTranslation } from 'react-i18next';
import {
  applyRecommendedPipeline,
  downloadPipelineAsset,
  loadPipelineAssetRequirements,
  type PipelineAssetRequirement,
} from '@/lib/pipeline-recommendations';
import { pipelineService } from '@/services/pipelineService';

type DownloadStatus = 'waiting' | 'downloading' | 'completed' | 'error';

interface DownloadState {
  status: DownloadStatus;
  progress: number;
  downloadedMb: number;
  totalMb: number;
  speedMbps: number;
  error?: string;
}

export function DownloadProgressStep() {
  const { t } = useTranslation(['onboarding', 'common']);
  const {
    goNext,
    selectedPipelinePreset,
    selectedSummaryModel,
    recommendedSummaryModel,
    transcriptionModelDownloaded,
    setTranscriptionModelDownloaded,
    summaryModelDownloaded,
    setSummaryModelDownloaded,
    startBackgroundDownloads,
    completeOnboarding,
  } = useOnboarding();

  const [isMac, setIsMac] = useState(false);

  const [transcriptionState, setTranscriptionState] = useState<DownloadState>({
    status: transcriptionModelDownloaded ? 'completed' : 'waiting',
    progress: transcriptionModelDownloaded ? 100 : 0,
    downloadedMb: 0,
    totalMb: 0,
    speedMbps: 0,
  });

  const [summaryState, setSummaryState] = useState<DownloadState>({
    status: summaryModelDownloaded ? 'completed' : 'waiting',
    progress: summaryModelDownloaded ? 100 : 0,
    downloadedMb: 0,
    totalMb: 0,
    speedMbps: 0,
  });

  const [isCompleting, setIsCompleting] = useState(false);
  const [pipelineAssets, setPipelineAssets] = useState<PipelineAssetRequirement[]>([]);

  const refreshPipelineAssets = async () => {
    const assets = await loadPipelineAssetRequirements(selectedPipelinePreset);
    setPipelineAssets(assets);
    const totalMb = assets.reduce((sum, asset) => sum + asset.downloadSizeMiB, 0);
    const downloadedMb = assets.filter((asset) => asset.installed)
      .reduce((sum, asset) => sum + asset.downloadSizeMiB, 0);
    const ready = assets.length > 0 && assets.every((asset) => asset.installed);
    setTranscriptionState((previous) => ({
      ...previous,
      status: ready ? 'completed' : previous.status === 'downloading' ? 'downloading' : 'waiting',
      progress: totalMb > 0 ? downloadedMb / totalMb * 100 : 0,
      downloadedMb,
      totalMb,
      error: undefined,
    }));
    setTranscriptionModelDownloaded(ready);
    return { assets, ready };
  };

  useEffect(() => {
    void refreshPipelineAssets().catch((error) => {
      setTranscriptionState((previous) => ({ ...previous, status: 'error', error: String(error) }));
    });
  }, [selectedPipelinePreset]);

  // Detect platform on mount
  useEffect(() => {
    const checkPlatform = async () => {
      try {
        const { platform } = await import('@tauri-apps/plugin-os');
        setIsMac(platform() === 'macos');
      } catch (e) {
        setIsMac(navigator.userAgent.includes('Mac'));
      }
    };

    checkPlatform();
  }, []);

  // Listen to Summary Model download progress (always downloading for builtin-ai)
  useEffect(() => {
    const unlisten = listen<{
      model: string;
      progress: number;
      downloaded_mb?: number;
      total_mb?: number;
      speed_mbps?: number;
      status: string;
      error?: string;
    }>('builtin-ai-download-progress', (event) => {
      const { model, progress, downloaded_mb, total_mb, speed_mbps, status, error } = event.payload;
      if (selectedSummaryModel && model === selectedSummaryModel) {
        setSummaryState((prev) => ({
          ...prev,
          status: status === 'completed'
            ? 'completed'
            : status === 'error'
            ? 'error'
            : 'downloading',
          progress,
          downloadedMb: downloaded_mb ?? prev.downloadedMb,
          totalMb: (total_mb ?? prev.totalMb) || getSummaryModelSizeMb(model),
          speedMbps: speed_mbps ?? prev.speedMbps,
          error: status === 'error' ? error : undefined,
        }));

        if (status === 'completed' || progress >= 100) {
          setSummaryModelDownloaded(true);
        }
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [selectedSummaryModel]);

  useEffect(() => {
    const modelForSize = selectedSummaryModel || recommendedSummaryModel;
    if (!modelForSize) return;

    setSummaryState((prev) => ({
      ...prev,
      status: summaryModelDownloaded
        ? 'completed'
        : prev.status === 'completed'
        ? 'waiting'
        : prev.status,
      progress: summaryModelDownloaded
        ? 100
        : prev.status === 'completed'
        ? 0
        : prev.progress,
      totalMb: prev.totalMb || getSummaryModelSizeMb(modelForSize),
    }));
  }, [selectedSummaryModel, recommendedSummaryModel, summaryModelDownloaded]);

  const startSummaryDownload = async () => {
    if (!summaryModelDownloaded && selectedSummaryModel) {
      try {
        setSummaryState((prev) => ({
          ...prev,
          status: 'downloading',
          totalMb: getSummaryModelSizeMb(selectedSummaryModel),
        }));
        await startBackgroundDownloads({
          includeTranscription: false,
          includeSummary: true,
          summaryModel: selectedSummaryModel,
        });
      } catch (error) {
        console.error('Failed to start summary model download:', error);
        setSummaryState((prev) => ({ ...prev, status: 'error', error: String(error) }));
      }
    }
  };

  const startTranscriptionDownload = async () => {
    if (transcriptionModelDownloaded || transcriptionState.status === 'downloading') return;

    try {
      setTranscriptionState((prev) => ({ ...prev, status: 'downloading', error: undefined }));
      if (selectedPipelinePreset === 'quality') {
        const beta = await pipelineService.getBetaFeatures();
        if (!beta.experimentalAsrModels) {
          await pipelineService.saveBetaFeatures({ ...beta, experimentalAsrModels: true });
        }
      }
      let assets = pipelineAssets;
      if (assets.length === 0) {
        assets = (await refreshPipelineAssets()).assets;
      }
      const totalMb = assets.reduce((sum, asset) => sum + asset.downloadSizeMiB, 0);
      let downloadedMb = assets.filter((asset) => asset.installed)
        .reduce((sum, asset) => sum + asset.downloadSizeMiB, 0);
      for (const asset of assets.filter((item) => !item.installed)) {
        await downloadPipelineAsset(asset);
        downloadedMb += asset.downloadSizeMiB;
        setTranscriptionState((previous) => ({
          ...previous,
          status: 'downloading',
          downloadedMb,
          totalMb,
          progress: totalMb > 0 ? downloadedMb / totalMb * 100 : 0,
        }));
      }
      await refreshPipelineAssets();
    } catch (error) {
      console.error('Failed to download recommended Pipeline assets:', error);
      setTranscriptionState((prev) => ({ ...prev, status: 'error', error: String(error) }));
    }
  };

  const handleContinue = async () => {
    let transcriptionReady = transcriptionModelDownloaded;

    // Verify the complete recommended setup on disk (catches state drift).
    try {
      const verified = await refreshPipelineAssets();
      transcriptionReady = verified.ready;

      if (transcriptionReady) {
        if (selectedPipelinePreset === 'quality') {
          const beta = await pipelineService.getBetaFeatures();
          if (!beta.experimentalAsrModels) {
            await pipelineService.saveBetaFeatures({ ...beta, experimentalAsrModels: true });
          }
        }
        const current = await pipelineService.getConfig();
        await pipelineService.save(applyRecommendedPipeline(current, selectedPipelinePreset));
      }
    } catch (error) {
      console.warn('[DownloadProgressStep] Failed to verify or save Pipeline:', error);
    }

    if (transcriptionState.status === 'downloading' || summaryState.status === 'downloading') {
      toast.info(t('onboarding:downloadsContinue'), {
        description: t('onboarding:downloadsContinueHint'),
        duration: 5000,
      });
    }

    if (isMac) {
      // macOS: Go to Permissions step (will complete after permissions granted)
      goNext();
    } else {
      // Non-macOS: Complete onboarding immediately (downloads continue in background)
      setIsCompleting(true);
      try {
        await completeOnboarding(transcriptionReady);

        // Small delay to ensure state is saved before reload
        await new Promise(resolve => setTimeout(resolve, 100));

        window.location.reload();
      } catch (error) {
        console.error('Failed to complete onboarding:', error);
        toast.error(t('onboarding:completeFailed'), {
          description: t('common:retry'),
        });
        setIsCompleting(false);
      }
    }
  };

  const renderDownloadCard = (
    title: string,
    icon: React.ReactNode,
    state: DownloadState,
    modelSize: string,
    sizeUnit = 'MB',
    onDownload?: () => void,
    downloadLabel = t('onboarding:download')
  ) => (
    <div className="bg-white rounded-xl border border-gray-200 p-5">
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-3">
          <div className="w-10 h-10 rounded-full bg-gray-100 flex items-center justify-center">
            {icon}
          </div>
          <div>
            <h3 className="font-medium text-gray-900">{title}</h3>
            <p className="text-sm text-gray-500">{modelSize}</p>
          </div>
        </div>
        <div>
          {state.status === 'waiting' && onDownload && (
            <Button size="sm" variant="outline" onClick={onDownload}>
              <Download className="w-4 h-4 mr-2" />
              {downloadLabel}
            </Button>
          )}
          {state.status === 'downloading' && (
            <Loader2 className="w-5 h-5 text-gray-700 animate-spin" />
          )}
          {state.status === 'completed' && (
            <div className="w-6 h-6 rounded-full bg-green-100 flex items-center justify-center">
              <Check className="w-4 h-4 text-green-600" />
            </div>
          )}
          {state.status === 'error' && (
            <span className="text-sm text-red-500">{t('onboarding:downloadFailed')}</span>
          )}
        </div>
      </div>

      {/* Progress Bar */}
      {(state.status === 'downloading' || state.status === 'completed') && (
        <div className="space-y-2">
          <div className="w-full h-2 bg-gray-200 rounded-full overflow-hidden">
            <div
              className="h-full bg-gradient-to-r from-gray-700 to-gray-900 rounded-full transition-all duration-300"
              style={{ width: `${state.progress}%` }}
            />
          </div>
          <div className="flex items-center justify-between text-sm">
            <span className="text-gray-600">
              {state.downloadedMb.toFixed(1)} {sizeUnit} / {state.totalMb.toFixed(1)} {sizeUnit}
            </span>
            <div className="flex items-center gap-2">
              {state.speedMbps > 0 && (
                <span className="text-gray-500">
                  {state.speedMbps.toFixed(1)} {sizeUnit}/s
                </span>
              )}
              <span className="font-semibold text-gray-900">
                {Math.round(state.progress)}%
              </span>
            </div>
          </div>
        </div>
      )}

      {state.status === 'error' && state.error && (
        <div className="mt-2 p-3 bg-red-50 border border-red-200 rounded-md">
          <p className="text-sm text-red-600 font-medium">{t('onboarding:downloadError')}</p>
          <p className="text-xs text-red-500 mt-1">{state.error}</p>
          {onDownload && (
            <button
              onClick={onDownload}
              className="mt-3 w-full h-9 px-4 bg-gray-900 hover:bg-gray-800 text-white text-sm font-medium rounded-md transition-colors flex items-center justify-center gap-2"
            >
              <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2}
                      d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
              </svg>
              {t('common:retry')}
            </button>
          )}
        </div>
      )}
    </div>
  );

  return (
    <OnboardingContainer
      title={t('onboarding:downloadTitle')}
      description={t('onboarding:downloadDescription')}
      step={3}
      totalSteps={isMac ? 4 : 3}
    >
      <div className="flex flex-col items-center space-y-6">
        {/* Download Cards */}
        <div className="w-full max-w-lg space-y-4">
          {renderDownloadCard(
            t('onboarding:pipelineBundle', { preset: t(`onboarding:pipelinePresets.${selectedPipelinePreset}.name`) }),
            <Mic className="w-5 h-5 text-gray-600" />,
            transcriptionState,
            pipelineAssets.map((asset) => asset.name).join(' · '),
            'MiB',
            startTranscriptionDownload,
            t('onboarding:downloadTranscription')
          )}

          {renderDownloadCard(
            t('onboarding:summaryEngine'),
            <Sparkles className="w-5 h-5 text-gray-600" />,
            summaryState,
            getSummaryModelSizeLabel(selectedSummaryModel || recommendedSummaryModel),
            'MiB',
            selectedSummaryModel ? startSummaryDownload : undefined,
            t('onboarding:downloadOptional')
          )}
        </div>

        {/* Info Message - Only show when the transcription model is downloaded */}
        <AnimatePresence>
          {transcriptionModelDownloaded && !summaryModelDownloaded && (
            <motion.div
              initial={{ opacity: 0, y: -10 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -10 }}
              transition={{ duration: 0.3, ease: 'easeOut' }}
              className="w-full max-w-lg bg-gray-100 rounded-lg p-4 text-sm text-gray-800"
            >
              <div className="flex items-start gap-3">
                <Download className="w-5 h-5 text-gray-600 flex-shrink-0 mt-0.5" />
                <div>
                  <p className="font-medium">{t('onboarding:summaryOptional')}</p>
                  <p className="text-gray-700 mt-1">
                    {t('onboarding:summaryOptionalHint')}
                  </p>
                </div>
              </div>
            </motion.div>
          )}
        </AnimatePresence>

        {/* Continue Button */}
        <div className="w-full max-w-xs">
          <Button
            onClick={handleContinue}
            disabled={isCompleting}
            className="w-full h-11 bg-gray-900 hover:bg-gray-800 text-white disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {isCompleting ? (
              <Loader2 className="w-4 h-4 mr-2 animate-spin" />
            ) : !transcriptionModelDownloaded ? (
              t('onboarding:skipModelsForNow')
            ) : (
              t('onboarding:continue')
            )}
          </Button>
        </div>
      </div>
    </OnboardingContainer>
  );
}
