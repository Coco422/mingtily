import { useState, useCallback, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { TranscriptModelConfig } from '@/types/capabilities';
import { SherpaAsrAPI } from '@/lib/sherpa-asr';

export interface RawModelInfo {
  name: string;
  size_mb: number;
  status: 'Available' | 'Missing' | { Downloading: { progress: number } } | { Error: string };
}

export interface ModelOption {
  provider: 'whisper' | 'parakeet' | 'sherpa-onnx';
  name: string;
  displayName: string;
  size_mb: number;
  inputMode: 'continuous' | 'vad-segmented' | 'whole-file';
}

/**
 * Fetch installed transcription models from every local provider.
 *
 * This hook centralizes the model fetching logic that was previously duplicated
 * in ImportAudioDialog and RetranscribeDialog components.
 *
 * @param transcriptModelConfig - User's saved model configuration from context
 * @returns Object containing available models, selected model key, loading state, and fetch function
 */
export function useTranscriptionModels(
  transcriptModelConfig: Partial<TranscriptModelConfig> | undefined,
  includeExperimental = false,
  includeContinuous = false
) {
  const [availableModels, setAvailableModels] = useState<ModelOption[]>([]);
  const [selectedModelKey, setSelectedModelKey] = useState<string>('');
  const [loadingModels, setLoadingModels] = useState(false);
  // Track whether the user has manually changed the model selection
  const userSelectedRef = useRef(false);

  // Wrap setSelectedModelKey to track user-initiated changes
  const setSelectedModelKeyWithTracking = useCallback((key: string) => {
    userSelectedRef.current = true;
    setSelectedModelKey(key);
  }, []);

  const fetchModels = useCallback(async () => {
    setLoadingModels(true);
    const allModels: ModelOption[] = [];

    // Fetch Whisper models
    try {
      const whisperModels = await invoke<RawModelInfo[]>('whisper_get_available_models');
      const availableWhisper = whisperModels
        .filter((m) => m.status === 'Available')
        .map((m) => ({
          provider: 'whisper' as const,
          name: m.name,
          displayName: `🏠 Whisper: ${m.name}`,
          size_mb: m.size_mb,
          inputMode: 'vad-segmented' as const,
        }));
      allModels.push(...availableWhisper);
    } catch (err) {
      console.error('Failed to fetch Whisper models:', err);
    }

    // Fetch Parakeet models
    try {
      await invoke('parakeet_init');
      const parakeetModels = await invoke<RawModelInfo[]>('parakeet_get_available_models');
      const availableParakeet = parakeetModels
        .filter((m) => m.status === 'Available')
        .map((m) => ({
          provider: 'parakeet' as const,
          name: m.name,
          displayName: `⚡ Parakeet: ${m.name}`,
          size_mb: m.size_mb,
          inputMode: 'vad-segmented' as const,
        }));
      allModels.push(...availableParakeet);
    } catch (err) {
      console.error('Failed to fetch Parakeet models:', err);
    }

    try {
      const sherpaModels = await SherpaAsrAPI.listModels();
      const availableSherpa = sherpaModels
        .filter((model) => model.status === 'available' && (includeExperimental || !model.beta))
        .map((model) => ({
          provider: 'sherpa-onnx' as const,
          name: model.id,
          displayName: `🀄 Sherpa ONNX: ${model.name}`,
          size_mb: model.installed_size / 1024 / 1024,
          inputMode: model.streaming_mode === 'continuous'
            ? 'continuous' as const
            : 'vad-segmented' as const,
        }));
      allModels.push(...availableSherpa);
    } catch (err) {
      console.error('Failed to fetch Sherpa ONNX models:', err);
    }

    const selectableModels = includeContinuous
      ? allModels
      : allModels.filter((model) => model.inputMode !== 'continuous');
    setAvailableModels(selectableModels);

    // Set default model based on user's saved configuration
    const configuredProvider = transcriptModelConfig?.provider || '';
    const configuredModel = transcriptModelConfig?.model || '';

    // Try to match the configured model
    // Note: 'localWhisper' in config maps to 'whisper' provider in model list
    const configuredMatch = selectableModels.find(
      (m) =>
        (configuredProvider === 'localWhisper' && m.provider === 'whisper' && m.name === configuredModel) ||
        (configuredProvider === 'parakeet' && m.provider === 'parakeet' && m.name === configuredModel) ||
        (configuredProvider === 'sherpa-onnx' && m.provider === 'sherpa-onnx' && m.name === configuredModel)
    );

    // Only set default model if user hasn't manually selected one
    if (!userSelectedRef.current) {
      if (configuredMatch) {
        // Use the configured model if available
        setSelectedModelKey(`${configuredMatch.provider}:${configuredMatch.name}`);
      } else if (selectableModels.length > 0) {
        // Fall back to first available model
        setSelectedModelKey(`${selectableModels[0].provider}:${selectableModels[0].name}`);
      }
    }

    setLoadingModels(false);
  }, [includeContinuous, includeExperimental, transcriptModelConfig]);

  // Reset user selection tracking (call when dialog opens fresh)
  const resetSelection = useCallback(() => {
    userSelectedRef.current = false;
  }, []);

  return {
    availableModels,
    selectedModelKey,
    setSelectedModelKey: setSelectedModelKeyWithTracking,
    loadingModels,
    fetchModels,
    resetSelection,
  };
}
