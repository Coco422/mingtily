import { useCallback, useEffect, useRef } from 'react';
import { Transcript, Summary } from '@/types';
import { ModelConfig } from '@/components/ModelSettingsModal';
import { invoke as invokeTauri } from '@tauri-apps/api/core';
import { toast } from 'sonner';
import { isOllamaNotInstalledError } from '@/lib/utils';
import { BuiltInModelInfo } from '@/lib/builtin-ai';
import { prefixResolvedSpeaker, type SpeakerParticipant } from '@/lib/speaker-map';
import { useTranslation } from 'react-i18next';
import {
  detectAndCacheSummaryLanguage,
  readMeetingSummaryLanguage,
  readCachedDetectedSummaryLanguage,
} from '@/lib/summary-language-preferences';
import { useSummaryJobs } from '@/contexts/SummaryJobsContext';

async function resolveSummaryLanguage(
  meetingId: string,
  transcriptTexts: string[],
  translate: (key: string, options?: Record<string, unknown>) => string,
): Promise<string | null> {
  try {
    const perMeeting = await readMeetingSummaryLanguage(meetingId);
    if (perMeeting.language) return perMeeting.language;
  } catch (err) {
    console.warn('Failed to load meeting summary language:', err);
    toast.warning(translate('summary:languageLoadFailed'), {
      description: translate('summary:languageAutoFallback'),
    });
  }

  try {
    const cachedDetected = await readCachedDetectedSummaryLanguage(meetingId);
    if (cachedDetected) return cachedDetected;
  } catch (err) {
    console.warn('Failed to load cached detected summary language:', err);
  }

  try {
    const detection = await detectAndCacheSummaryLanguage(meetingId, transcriptTexts);
    if (detection.reason === 'tie') {
      toast.warning(translate('summary:bilingualDetected'), {
        description: translate('summary:bilingualHint'),
      });
    }
    return detection.language;
  } catch (err) {
    console.warn('Failed to detect transcript summary language:', err);
    return null;
  }
}

type SummaryStatus = 'idle' | 'processing' | 'summarizing' | 'regenerating' | 'completed' | 'error';

interface UseSummaryGenerationProps {
  meeting: any;
  transcripts: Transcript[];
  modelConfig: ModelConfig;
  isModelConfigLoading: boolean;
  selectedTemplate: string;
  onMeetingUpdated?: () => Promise<void>;
  updateMeetingTitle: (title: string) => void;
  setAiSummary: (summary: Summary | null) => void;
  onOpenModelSettings?: () => void;
  speakerParticipants?: SpeakerParticipant[];
}

export function useSummaryGeneration({
  meeting,
  transcripts,
  modelConfig,
  isModelConfigLoading,
  selectedTemplate,
  onMeetingUpdated,
  updateMeetingTitle,
  setAiSummary,
  onOpenModelSettings,
  speakerParticipants = [],
}: UseSummaryGenerationProps) {
  const { t } = useTranslation(['summary', 'common']);
  const { getJob, refreshJob, trackJob, cancelJob, acknowledgeJob } = useSummaryJobs();
  const job = getJob(meeting.id);
  const summaryStatus: SummaryStatus = job?.status === 'pending' || job?.status === 'processing'
    ? 'processing'
    : job?.status === 'completed'
      ? 'completed'
      : job?.status === 'failed' || job?.status === 'error' || job?.status === 'interrupted'
        ? 'error'
        : job?.status === 'cancelled' && job.data
          ? 'completed'
          : 'idle';
  const summaryError = summaryStatus === 'error' ? job?.error || t('summary:generationFailed') : null;
  const streamingSummary = job?.streamingSummary || '';
  const streamingThinking = job?.streamingThinking ?? null;
  const streamingThinkingComplete = job?.streamingThinkingComplete || false;
  const summaryPhase = job?.phase ?? null;
  const summaryCurrentStep = job?.currentStep ?? null;
  const summaryTotalSteps = job?.totalSteps ?? null;
  const summaryStartedAt = job?.startedAt ?? null;
  const handledCompletionRef = useRef(job?.status === 'completed');

  useEffect(() => {
    void refreshJob(meeting.id).catch((error) => {
      console.error('Failed to restore summary job:', error);
    });
  }, [meeting.id, refreshJob]);

  useEffect(() => {
    if (job?.data) setAiSummary(job.data);
    if (job?.unread) acknowledgeJob(meeting.id);
  }, [acknowledgeJob, job?.data, job?.unread, meeting.id, setAiSummary]);

  useEffect(() => {
    if (job?.status === 'pending' || job?.status === 'processing') {
      handledCompletionRef.current = false;
      return;
    }
    if (job?.status !== 'completed' || handledCompletionRef.current) return;
    handledCompletionRef.current = true;
    if (job.meetingName) updateMeetingTitle(job.meetingName);
    void onMeetingUpdated?.();
  }, [job?.meetingName, job?.status, onMeetingUpdated, updateMeetingTitle]);

  // Helper to get status message
  const getSummaryStatusMessage = useCallback((status: SummaryStatus) => {
    switch (status) {
      case 'processing':
        return t('summary:statusProcessing');
      case 'summarizing':
        return t('summary:statusGenerating');
      case 'regenerating':
        return t('summary:statusRegenerating');
      case 'completed':
        return t('summary:statusCompleted');
      case 'error':
        return t('summary:statusError');
      default:
        return '';
    }
  }, [t]);

  // Unified summary processing logic
  const processSummary = useCallback(async ({
    transcriptText,
    transcriptTexts,
    customPrompt = '',
    isRegeneration = false,
  }: {
    transcriptText: string;
    transcriptTexts?: string[];
    customPrompt?: string;
    isRegeneration?: boolean;
  }) => {
    try {
      if (!transcriptText.trim()) {
        throw new Error(t('summary:noTranscriptText'));
      }

      console.log('Processing transcript with template:', selectedTemplate);

      // Show toast notification for generation start
      toast.info(t(isRegeneration ? 'summary:regenerating' : 'summary:generating'), {
        description: t('summary:usingModel', { provider: modelConfig.provider, model: modelConfig.model }),
        duration: 3000,
      });

      // Resolve explicit metadata override first; Auto detects the transcript language.
      const summaryLanguage = await resolveSummaryLanguage(
        meeting.id,
        transcriptTexts?.length ? transcriptTexts : [transcriptText],
        t,
      );

      await invokeTauri('api_process_transcript', {
        text: transcriptText,
        model: modelConfig.provider,
        modelName: modelConfig.model,
        meetingId: meeting.id,
        chunkSize: 40000,
        overlap: 1000,
        customPrompt: customPrompt,
        templateId: selectedTemplate,
        summaryLanguage,
      });
      trackJob(meeting.id);
    } catch (error) {
      console.error(`Failed to ${isRegeneration ? 'regenerate' : 'generate'} summary:`, error);
      const errorMessage = error instanceof Error ? error.message : t('common:unknown');
      toast.error(t(isRegeneration ? 'summary:regenerationFailed' : 'summary:generationFailed'), {
        description: errorMessage,
      });
      void refreshJob(meeting.id);
      const isModelRequiredError = errorMessage.includes('model is required') ||
        errorMessage.includes('"model":"required"') ||
        (errorMessage.toLowerCase().includes('model') && errorMessage.toLowerCase().includes('required'));
      if (isModelRequiredError) onOpenModelSettings?.();
    }
  }, [
    meeting.id,
    modelConfig,
    selectedTemplate,
    trackJob,
    refreshJob,
    onOpenModelSettings,
    t,
  ]);

  // Helper function to fetch ALL transcripts for summary generation
  const fetchAllTranscripts = useCallback(async (meetingId: string): Promise<Transcript[]> => {
    try {
      console.log('📊 Fetching all transcripts for meeting:', meetingId);

      const allData = await invokeTauri('api_get_meeting_transcripts', {
        meetingId,
      }) as { transcripts: Transcript[]; total_count: number; has_more: boolean };

      console.log(`✅ Fetched ${allData.transcripts.length} transcripts from database`);
      return allData.transcripts;
    } catch (error) {
      console.error('❌ Error fetching all transcripts:', error);
      toast.error(t('summary:fetchTranscriptsFailed'));
      return [];
    }
  }, [t]);

  const buildSummaryTranscriptPayload = useCallback((allTranscripts: Transcript[]) => {
    const formatTime = (seconds: number | undefined, fallbackTimestamp: string): string => {
      if (seconds === undefined) {
        return fallbackTimestamp;
      }
      const totalSecs = Math.floor(seconds);
      const mins = Math.floor(totalSecs / 60);
      const secs = totalSecs % 60;
      return `[${mins.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}]`;
    };

    return {
      transcriptText: allTranscripts
        .map(segment => `${formatTime(segment.audio_start_time, segment.timestamp)} ${prefixResolvedSpeaker(segment.text, segment.speaker, speakerParticipants, t)}`)
        .join('\n'),
      transcriptTexts: allTranscripts.map(segment => prefixResolvedSpeaker(segment.text, segment.speaker, speakerParticipants, t)),
    };
  }, [speakerParticipants, t]);

  // Public API: Generate summary from transcripts
  const handleGenerateSummary = useCallback(async (customPrompt: string = '') => {
    // Check if model config is still loading
    if (isModelConfigLoading) {
      console.log('⏳ Model configuration is still loading, please wait...');
      toast.info(t('summary:modelConfigLoading'));
      return;
    }

    // CHANGE: Fetch ALL transcripts from database, not from pagination state
    console.log('📊 Fetching all transcripts for summary generation...');
    const allTranscripts = await fetchAllTranscripts(meeting.id);

    if (!allTranscripts.length) {
      const error_msg = t('summary:noTranscripts');
      console.log(error_msg);
      toast.error(error_msg);
      return;
    }

    console.log(`✅ Proceeding with ${allTranscripts.length} transcripts`);

    console.log('🚀 Starting summary generation with config:', {
      provider: modelConfig.provider,
      model: modelConfig.model,
      template: selectedTemplate
    });

    // Check if Ollama provider has models available
    if (modelConfig.provider === 'ollama') {
      try {
        const endpoint = modelConfig.ollamaEndpoint || null;
        const models = await invokeTauri('get_ollama_models', { endpoint }) as any[];

        if (!models || models.length === 0) {
          toast.error(
            t('summary:noOllamaModels'),
            { duration: 5000 }
          );
          return;
        }
      } catch (error) {
        console.error('Error checking Ollama models:', error);
        const errorMessage = error instanceof Error ? error.message : String(error);

        if (isOllamaNotInstalledError(errorMessage)) {
          // Ollama is not installed - show specific message with download link
          toast.error(
            t('summary:ollamaNotInstalled'),
            {
              description: t('summary:ollamaInstallHint'),
              duration: 7000,
              action: {
                label: t('common:download'),
                onClick: () => invokeTauri('open_external_url', { url: 'https://ollama.com/download' })
              }
            }
          );
        } else {
          // Other error - generic message
          toast.error(
            t('summary:ollamaCheckFailed'),
            { duration: 5000 }
          );
        }
        return;
      }
    }

    // Check if built-in AI provider has models available
    if (modelConfig.provider === 'builtin-ai') {
      try {
        const selectedModel = modelConfig.model;

        if (!selectedModel) {
          toast.error(t('summary:noBuiltInModel'), {
            description: t('summary:selectModelHint'),
            duration: 5000,
          });
          if (onOpenModelSettings) {
            onOpenModelSettings();
          }
          return;
        }

        // Check model readiness with filesystem refresh
        const isReady = await invokeTauri<boolean>('builtin_ai_is_model_ready', {
          modelName: selectedModel,
          refresh: true,
        });

        if (!isReady) {
          // Get detailed model status
          const modelInfo = await invokeTauri<BuiltInModelInfo | null>('builtin_ai_get_model_info', {
            modelName: selectedModel,
          });

          if (modelInfo) {
            const status = modelInfo.status;

            if (status.type === 'downloading') {
              toast.info(t('summary:modelDownloading'), {
                description: t('summary:modelDownloadingHint', { model: selectedModel, progress: status.progress }),
                duration: 5000,
              });
              return;
            }

            if (status.type === 'not_downloaded') {
              toast.error(t('summary:modelNotDownloaded'), {
                description: t('summary:modelNotDownloadedHint', { model: selectedModel }),
                duration: 7000,
              });
              if (onOpenModelSettings) {
                onOpenModelSettings();
              }
              return;
            }

            if (status.type === 'corrupted' || status.type === 'error') {
              const errorDesc = status.type === 'error'
                ? status.Error || t('summary:modelFileError')
                : t('summary:modelFileCorrupted');
              toast.error(t('summary:modelUnavailable'), {
                description: t('summary:modelUnavailableHint', { error: errorDesc }),
                duration: 7000,
              });
              if (onOpenModelSettings) {
                onOpenModelSettings();
              }
              return;
            }
          }

          // Fallback if we couldn't get model info
          toast.error(t('summary:modelNotReady'), {
            description: t('summary:modelNotReadyHint'),
            duration: 5000,
          });
          if (onOpenModelSettings) {
            onOpenModelSettings();
          }
          return;
        }

        // Model is ready, continue to backend call
      } catch (error) {
        console.error('Error validating built-in AI model:', error);
        toast.error(t('summary:modelValidationFailed'), {
          description: error instanceof Error ? error.message : String(error),
          duration: 5000,
        });
        return;
      }
    }

    const summaryPayload = buildSummaryTranscriptPayload(allTranscripts);

    await processSummary({
      ...summaryPayload,
      customPrompt,
    });
  }, [meeting.id, fetchAllTranscripts, buildSummaryTranscriptPayload, processSummary, modelConfig, isModelConfigLoading, selectedTemplate, t]);

  // Public API: Regenerate summary from the current saved transcript
  const handleRegenerateSummary = useCallback(async () => {
    const allTranscripts = await fetchAllTranscripts(meeting.id);

    if (!allTranscripts.length) {
      console.error('No transcripts available for regeneration');
      toast.error(t('summary:noTranscriptsForRegeneration'));
      return;
    }

    await processSummary({
      ...buildSummaryTranscriptPayload(allTranscripts),
      isRegeneration: true
    });
  }, [meeting.id, fetchAllTranscripts, buildSummaryTranscriptPayload, processSummary, t]);

  // Public API: Stop ongoing summary generation
  const handleStopGeneration = useCallback(async () => {
    console.log('Stopping summary generation for meeting:', meeting.id);

    try {
      await cancelJob(meeting.id);
      console.log('✓ Backend cancellation request sent for meeting:', meeting.id);
    } catch (error) {
      console.error('Failed to cancel summary generation:', error);
      // Continue with frontend cleanup even if backend call fails
    }

    // Show toast notification
    toast.info(t('summary:stopped'), {
      description: t('summary:stoppedHint'),
      duration: 3000,
    });
  }, [cancelJob, meeting.id, t]);

  return {
    summaryStatus,
    summaryError,
    streamingSummary,
    streamingThinking,
    streamingThinkingComplete,
    summaryPhase,
    summaryCurrentStep,
    summaryTotalSteps,
    summaryStartedAt,
    handleGenerateSummary,
    handleRegenerateSummary,
    handleStopGeneration,
    getSummaryStatusMessage,
  };
}
