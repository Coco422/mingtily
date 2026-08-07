import { useState, useEffect, useCallback, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useTranscripts } from '@/contexts/TranscriptContext';
import { useSidebar } from '@/components/Sidebar/SidebarProvider';
import { useConfig } from '@/contexts/ConfigContext';
import { useRecordingState, RecordingStatus } from '@/contexts/RecordingStateContext';
import { recordingService } from '@/services/recordingService';
import { showRecordingNotification } from '@/lib/recordingNotification';
import { toast } from 'sonner';
import { useTranslation } from 'react-i18next';

interface UseRecordingStartReturn {
  handleRecordingStart: () => Promise<void>;
  isAutoStarting: boolean;
}

type RecordingStartSource = 'manual' | 'navigation' | 'sidebar';

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function isAlreadyRecordingError(error: unknown): boolean {
  return errorMessage(error).toLowerCase().includes('recording already in progress');
}

/**
 * Custom hook for managing recording start lifecycle.
 * Handles both manual start (button click) and auto-start (from sidebar navigation).
 *
 * Features:
 * - Meeting title generation (format: Meeting DD_MM_YY_HH_MM_SS)
 * - Transcript clearing on start
 * - Recording notification display
 * - Auto-start from sidebar via sessionStorage flag
 */
export function useRecordingStart(
  isRecording: boolean,
  setIsRecording: (value: boolean) => void,
  showModal?: (name: 'modelSelector', message?: string) => void
): UseRecordingStartReturn {
  const { t } = useTranslation(['recording', 'meeting']);
  const [isAutoStarting, setIsAutoStarting] = useState(false);
  const startInFlightRef = useRef(false);

  const { clearTranscripts, setMeetingTitle } = useTranscripts();
  const { setIsMeetingActive } = useSidebar();
  const { selectedDevices } = useConfig();
  const { setStatus } = useRecordingState();

  // Generate meeting title with timestamp
  const generateMeetingTitle = useCallback(() => {
    const now = new Date();
    const day = String(now.getDate()).padStart(2, '0');
    const month = String(now.getMonth() + 1).padStart(2, '0');
    const year = String(now.getFullYear()).slice(-2);
    const hours = String(now.getHours()).padStart(2, '0');
    const minutes = String(now.getMinutes()).padStart(2, '0');
    const seconds = String(now.getSeconds()).padStart(2, '0');
    return t('meeting:defaultTitle', { date: `${day}_${month}_${year}_${hours}_${minutes}_${seconds}` });
  }, [t]);

  // Validate the provider and model currently selected in Services.
  const checkTranscriptionReady = useCallback(async (): Promise<boolean> => {
    try {
      await invoke('validate_transcription_model_ready');
      return true;
    } catch (error) {
      console.error('Failed to validate transcription model:', error);
      return false;
    }
  }, []);

  // Check the active provider for an in-progress model download.
  const checkIfModelDownloading = useCallback(async (): Promise<boolean> => {
    try {
      const config = await invoke<{ provider: string } | null>('api_get_transcript_config');
      if (config?.provider === 'sherpa-onnx') return false;
      const command = config?.provider === 'localWhisper'
        ? 'whisper_get_available_models'
        : 'parakeet_get_available_models';
      const models = await invoke<any[]>(command);
      const isDownloading = models.some(m =>
        m.status && (
          typeof m.status === 'object'
            ? 'Downloading' in m.status
            : m.status === 'Downloading'
        )
      );
      return isDownloading;
    } catch (error) {
      console.error('Failed to check model download status:', error);
      return false; // Default to not downloading (will show error + modal)
    }
  }, []);

  const ensureTranscriptionReady = useCallback(async (): Promise<boolean> => {
    const transcriptionReady = await checkTranscriptionReady();
    if (transcriptionReady) return true;

    const isDownloading = await checkIfModelDownloading();
    if (isDownloading) {
      toast.info(t('recording:modelDownloading'), {
        description: t('recording:modelDownloadingHint'),
        duration: 5000,
      });
    } else {
      toast.error(t('recording:modelNotReady'), {
        description: t('recording:modelNotReadyHint'),
        duration: 5000,
      });
      showModal?.('modelSelector', t('recording:modelSetupRequired'));
    }
    setStatus(RecordingStatus.IDLE);
    return false;
  }, [checkIfModelDownloading, checkTranscriptionReady, setStatus, showModal, t]);

  const startConfiguredRecording = useCallback(async (
    source: RecordingStartSource,
    rethrowError: boolean
  ) => {
    if (isRecording || startInFlightRef.current) {
      console.log(`Ignoring ${source} recording start because a session is active or starting`);
      return;
    }

    // State updates are asynchronous, so use a ref as the synchronous lock shared by
    // the button, navigation auto-start, and sidebar event paths.
    startInFlightRef.current = true;
    if (source !== 'manual') setIsAutoStarting(true);

    try {
      console.log(`${source} recording start - checking configured transcription model`);
      if (!(await ensureTranscriptionReady())) return;

      const generatedMeetingTitle = generateMeetingTitle();

      // Set STARTING status before initiating backend recording
      setStatus(RecordingStatus.STARTING, t('recording:initializing'));

      let startedNewSession = true;
      try {
        console.log('Starting backend recording with meeting:', generatedMeetingTitle);
        await recordingService.startRecordingWithDevices(
          selectedDevices?.micDevice || null,
          selectedDevices?.systemDevice || null,
          generatedMeetingTitle
        );
        console.log('Backend recording started successfully');
      } catch (error) {
        // A stale UI event can arrive just after another path successfully starts.
        // Treat the backend's safe duplicate-start rejection as idempotent success.
        const backendIsRecording = isAlreadyRecordingError(error)
          ? await recordingService.isRecording().catch(() => false)
          : false;
        if (!backendIsRecording) throw error;

        startedNewSession = false;
        console.warn('Recording was already active; synchronizing the UI instead of showing an error');
      }

      const activeMeetingTitle = startedNewSession
        ? generatedMeetingTitle
        : await recordingService.getRecordingMeetingName().catch(() => null);
      if (activeMeetingTitle) setMeetingTitle(activeMeetingTitle);
      setIsRecording(true);
      setIsMeetingActive(true);
      setStatus(RecordingStatus.RECORDING);

      if (startedNewSession) {
        clearTranscripts();
        await showRecordingNotification();
      }
    } catch (error) {
      console.error('Failed to start recording:', error);
      setStatus(RecordingStatus.ERROR, errorMessage(error) || t('recording:startFailed'));
      setIsRecording(false);
      if (rethrowError) throw error;
      alert(t('recording:startFailedHint'));
    } finally {
      startInFlightRef.current = false;
      if (source !== 'manual') setIsAutoStarting(false);
    }
  }, [
    clearTranscripts,
    ensureTranscriptionReady,
    generateMeetingTitle,
    isRecording,
    selectedDevices,
    setIsMeetingActive,
    setIsRecording,
    setMeetingTitle,
    setStatus,
    t,
  ]);

  // Handle manual recording start (from button click)
  const handleRecordingStart = useCallback(async () => {
    await startConfiguredRecording('manual', true);
  }, [startConfiguredRecording]);

  // Check for autoStartRecording flag and start recording automatically
  useEffect(() => {
    const checkAutoStartRecording = async () => {
      if (typeof window !== 'undefined') {
        const shouldAutoStart = sessionStorage.getItem('autoStartRecording');
        if (shouldAutoStart === 'true' && !isRecording && !isAutoStarting) {
          console.log('Auto-starting recording from navigation...');
          sessionStorage.removeItem('autoStartRecording'); // Clear the flag
          await startConfiguredRecording('navigation', false);
        }
      }
    };

    checkAutoStartRecording();
  }, [
    isRecording,
    isAutoStarting,
    startConfiguredRecording,
  ]);

  // Listen for direct recording trigger from sidebar when already on home page
  useEffect(() => {
    const handleDirectStart = async () => {
      if (isRecording || isAutoStarting || startInFlightRef.current) {
        console.log('Recording already in progress, ignoring direct start event');
        return;
      }

      await startConfiguredRecording('sidebar', false);
    };

    window.addEventListener('start-recording-from-sidebar', handleDirectStart);

    return () => {
      window.removeEventListener('start-recording-from-sidebar', handleDirectStart);
    };
  }, [
    isRecording,
    isAutoStarting,
    startConfiguredRecording,
  ]);

  return {
    handleRecordingStart,
    isAutoStarting,
  };
}
