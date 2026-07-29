import { useState, useCallback, useEffect, useRef } from 'react';
import { Transcript, Summary } from '@/types';
import { ModelConfig } from '@/components/ModelSettingsModal';
import { CurrentMeeting, useSidebar } from '@/components/Sidebar/SidebarProvider';
import { invoke as invokeTauri } from '@tauri-apps/api/core';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { toast } from 'sonner';
import { isOllamaNotInstalledError } from '@/lib/utils';
import { BuiltInModelInfo } from '@/lib/builtin-ai';
import { prefixSpeaker } from '@/lib/speaker-label';
import { useTranslation } from 'react-i18next';
import {
  detectAndCacheSummaryLanguage,
  readMeetingSummaryLanguage,
  readCachedDetectedSummaryLanguage,
} from '@/lib/summary-language-preferences';

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

interface SummaryGenerationStreamPayload {
  meeting_id: string;
  markdown: string;
  thinking: string | null;
  thinking_complete: boolean;
  phase: 'final';
}

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
}: UseSummaryGenerationProps) {
  const { t } = useTranslation(['summary', 'common']);
  const [summaryStatus, setSummaryStatus] = useState<SummaryStatus>('idle');
  const [summaryError, setSummaryError] = useState<string | null>(null);
  const [streamingSummary, setStreamingSummary] = useState('');
  const [streamingThinking, setStreamingThinking] = useState<string | null>(null);
  const [streamingThinkingComplete, setStreamingThinkingComplete] = useState(false);
  const generationActiveRef = useRef(false);
  const pendingStreamUpdateRef = useRef<SummaryGenerationStreamPayload | null>(null);
  const streamingFrameRef = useRef<number | null>(null);

  const { startSummaryPolling, stopSummaryPolling } = useSidebar();

  const clearStreamingSummary = useCallback(() => {
    pendingStreamUpdateRef.current = null;
    if (streamingFrameRef.current !== null) {
      cancelAnimationFrame(streamingFrameRef.current);
      streamingFrameRef.current = null;
    }
    setStreamingSummary('');
    setStreamingThinking(null);
    setStreamingThinkingComplete(false);
  }, []);

  useEffect(() => {
    let disposed = false;
    let unlisten: UnlistenFn | undefined;

    clearStreamingSummary();
    generationActiveRef.current = false;

    void listen<SummaryGenerationStreamPayload>('summary-generation-stream', (event) => {
      if (
        generationActiveRef.current &&
        event.payload.meeting_id === meeting.id &&
        event.payload.phase === 'final'
      ) {
        pendingStreamUpdateRef.current = event.payload;
        if (streamingFrameRef.current === null) {
          streamingFrameRef.current = requestAnimationFrame(() => {
            streamingFrameRef.current = null;
            const pending = pendingStreamUpdateRef.current;
            if (pending) {
              setStreamingSummary(pending.markdown);
              setStreamingThinking(pending.thinking);
              setStreamingThinkingComplete(pending.thinking_complete);
            }
          });
        }
      }
    }).then((stopListening) => {
      if (disposed) {
        stopListening();
      } else {
        unlisten = stopListening;
      }
    }).catch((error) => {
      console.error('Failed to listen for summary stream:', error);
    });

    return () => {
      disposed = true;
      if (streamingFrameRef.current !== null) {
        cancelAnimationFrame(streamingFrameRef.current);
        streamingFrameRef.current = null;
      }
      unlisten?.();
    };
  }, [meeting.id, clearStreamingSummary]);

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
    generationActiveRef.current = true;
    clearStreamingSummary();
    setSummaryStatus(isRegeneration ? 'regenerating' : 'processing');
    setSummaryError(null);

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

      // Process transcript and get process_id
      const result = await invokeTauri('api_process_transcript', {
        text: transcriptText,
        model: modelConfig.provider,
        modelName: modelConfig.model,
        meetingId: meeting.id,
        chunkSize: 40000,
        overlap: 1000,
        customPrompt: customPrompt,
        templateId: selectedTemplate,
        summaryLanguage,
      }) as any;

      const process_id = result.process_id;
      console.log('Process ID:', process_id);

      // Start global polling via context
      startSummaryPolling(meeting.id, process_id, async (pollingResult) => {
        console.log('Summary status:', pollingResult);

        // Handle cancellation
        if (pollingResult.status === 'cancelled') {
          generationActiveRef.current = false;
          clearStreamingSummary();
          console.log('Summary generation was cancelled');

          // Reload summary from database (backend has already restored from backup)
          try {
            const existingSummary = await invokeTauri('api_get_summary', {
              meetingId: meeting.id
            }) as any;

            if (existingSummary?.data) {
              console.log('Restored previous summary after cancellation');
              setAiSummary(existingSummary.data);
              setSummaryStatus('completed');
            } else {
              setSummaryStatus('idle');
            }
          } catch (error) {
            console.error('Failed to reload summary after cancellation:', error);
            setSummaryStatus('idle');
          }

          setSummaryError(null);
          return;
        }

        // Handle errors
        if (pollingResult.status === 'error' || pollingResult.status === 'failed') {
          generationActiveRef.current = false;
          clearStreamingSummary();
          console.error('Backend returned error:', pollingResult.error);
          const errorMessage = pollingResult.error || t(isRegeneration ? 'summary:regenerationFailed' : 'summary:generationFailed');

          // If this was a regeneration, try to restore previous summary from database
          if (isRegeneration) {
            try {
              const existingSummary = await invokeTauri('api_get_summary', {
                meetingId: meeting.id
              }) as any;

              if (existingSummary?.data) {
                console.log('Restored previous summary after regeneration failure');
                setAiSummary(existingSummary.data);
                setSummaryStatus('completed');
                setSummaryError(null);

                // Show error toast with restoration message
                toast.error(t('summary:regenerationFailed'), {
                  description: t('summary:previousRestored', { error: errorMessage }),
                });

                return;
              }
            } catch (error) {
              console.error('Failed to reload summary after error:', error);
            }
          }

          // Continue with normal error handling if not regeneration or reload failed
          setSummaryError(errorMessage);
          setSummaryStatus('error');

          // Check if this is a "model is required" error
          const isModelRequiredError = errorMessage.includes('model is required') ||
            errorMessage.includes('"model":"required"') ||
            errorMessage.toLowerCase().includes('model') && errorMessage.toLowerCase().includes('required');

          // Show error toast
          toast.error(t(isRegeneration ? 'summary:regenerationFailed' : 'summary:generationFailed'), {
            description: errorMessage.includes('Connection refused')
              ? t('summary:connectionRefused')
              : errorMessage,
          });

          // Auto-open model settings modal if model is missing
          if (isModelRequiredError && onOpenModelSettings) {
            console.log('🔧 Model required error detected, opening model settings...');
            onOpenModelSettings();
          }

          return;
        }

        // Handle successful completion
        if (pollingResult.status === 'completed' && pollingResult.data) {
          generationActiveRef.current = false;
          console.log('Summary generation completed:', pollingResult.data);

          // Update meeting title if available
          const meetingName = pollingResult.data.MeetingName || pollingResult.meetingName;
          if (meetingName) {
            updateMeetingTitle(meetingName);
          }

          // Check if backend returned markdown format (new flow)
          if (pollingResult.data.markdown) {
            console.log('Received markdown format from backend');
            setAiSummary({ markdown: pollingResult.data.markdown } as any);
            clearStreamingSummary();
            setSummaryStatus('completed');

            // Show success toast
            toast.success(t('summary:success'), {
              description: t('summary:ready'),
              duration: 4000,
            });

            if (meetingName && onMeetingUpdated) {
              await onMeetingUpdated();
            }

            return;
          }

          // Legacy format handling
          const summarySections = Object.entries(pollingResult.data).filter(([key]) => key !== 'MeetingName');
          const allEmpty = summarySections.every(([, section]) => !(section as any).blocks || (section as any).blocks.length === 0);

          if (allEmpty) {
            console.error('Summary completed but all sections empty');
            clearStreamingSummary();
            setSummaryError(t('summary:emptyResult'));
            setSummaryStatus('error');

            return;
          }

          // Remove MeetingName from data before formatting
          const { MeetingName, ...summaryData } = pollingResult.data;

          // Format legacy summary data
          const formattedSummary: Summary = {};
          const sectionKeys = pollingResult.data._section_order || Object.keys(summaryData);

          for (const key of sectionKeys) {
            try {
              const section = summaryData[key];
              if (section && typeof section === 'object' && 'title' in section && 'blocks' in section) {
                const typedSection = section as { title?: string; blocks?: any[] };

                if (Array.isArray(typedSection.blocks)) {
                  formattedSummary[key] = {
                    title: typedSection.title || key,
                    blocks: typedSection.blocks.map((block: any) => ({
                      ...block,
                      color: 'default',
                      content: block?.content?.trim() || ''
                    }))
                  };
                } else {
                  formattedSummary[key] = {
                    title: typedSection.title || key,
                    blocks: []
                  };
                }
              }
            } catch (error) {
              console.warn(`Error processing section ${key}:`, error);
            }
          }

          setAiSummary(formattedSummary);
          clearStreamingSummary();
          setSummaryStatus('completed');

          // Show success toast
          toast.success(t('summary:success'), {
            description: t('summary:ready'),
            duration: 4000,
          });

          if (meetingName && onMeetingUpdated) {
            await onMeetingUpdated();
          }
        }
      });
    } catch (error) {
      generationActiveRef.current = false;
      clearStreamingSummary();
      console.error(`Failed to ${isRegeneration ? 'regenerate' : 'generate'} summary:`, error);
      const errorMessage = error instanceof Error ? error.message : t('common:unknown');
      setSummaryError(errorMessage);
      setSummaryStatus('error');
      // Note: We don't clear the summary here because the backend has already restored from backup

      toast.error(t(isRegeneration ? 'summary:regenerationFailed' : 'summary:generationFailed'), {
        description: errorMessage,
      });

    }
  }, [
    meeting.id,
    meeting.created_at,
    modelConfig,
    selectedTemplate,
    startSummaryPolling,
    setAiSummary,
    updateMeetingTitle,
    onMeetingUpdated,
    clearStreamingSummary,
    t,
  ]);

  // Helper function to fetch ALL transcripts for summary generation
  const fetchAllTranscripts = useCallback(async (meetingId: string): Promise<Transcript[]> => {
    try {
      console.log('📊 Fetching all transcripts for meeting:', meetingId);

      // First, get total count by fetching first page
      const firstPage = await invokeTauri('api_get_meeting_transcripts', {
        meetingId,
        limit: 1,
        offset: 0,
      }) as { transcripts: Transcript[]; total_count: number; has_more: boolean };

      const totalCount = firstPage.total_count;
      console.log(`📊 Total transcripts in database: ${totalCount}`);

      if (totalCount === 0) {
        return [];
      }

      // Fetch all transcripts in one call
      const allData = await invokeTauri('api_get_meeting_transcripts', {
        meetingId,
        limit: totalCount,
        offset: 0,
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
        .map(segment => `${formatTime(segment.audio_start_time, segment.timestamp)} ${prefixSpeaker(segment.text, segment.speaker, (key, options) => t(`common:${key}`, options))}`)
        .join('\n'),
      transcriptTexts: allTranscripts.map(segment => prefixSpeaker(segment.text, segment.speaker, (key, options) => t(`common:${key}`, options))),
    };
  }, [t]);

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
      // Call backend to cancel the summary generation
      await invokeTauri('api_cancel_summary', {
        meetingId: meeting.id
      });
      console.log('✓ Backend cancellation request sent for meeting:', meeting.id);
    } catch (error) {
      console.error('Failed to cancel summary generation:', error);
      // Continue with frontend cleanup even if backend call fails
    }

    // Stop polling
    stopSummaryPolling(meeting.id);

    // Reset status to idle
    generationActiveRef.current = false;
    clearStreamingSummary();
    setSummaryStatus('idle');
    setSummaryError(null);

    // Show toast notification
    toast.info(t('summary:stopped'), {
      description: t('summary:stoppedHint'),
      duration: 3000,
    });
  }, [meeting.id, stopSummaryPolling, clearStreamingSummary, t]);

  return {
    summaryStatus,
    summaryError,
    streamingSummary,
    streamingThinking,
    streamingThinkingComplete,
    handleGenerateSummary,
    handleRegenerateSummary,
    handleStopGeneration,
    getSummaryStatusMessage,
  };
}
