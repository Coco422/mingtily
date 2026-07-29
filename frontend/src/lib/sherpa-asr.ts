import { invoke } from '@tauri-apps/api/core';

export type SherpaAsrModelState = 'available' | 'missing' | 'corrupt';

export interface SherpaAsrModelStatus {
  id: string;
  name: string;
  backend: string;
  status: SherpaAsrModelState;
  download_size: number;
  installed_size: number;
  languages: string[];
  language_hint: 'auto-only' | 'auto-or-fixed';
  streaming_mode: 'vad-segmented' | 'continuous';
  license: string;
  recommended: boolean;
  beta: boolean;
  path: string;
  error?: string | null;
}

export interface SherpaAsrDownloadProgress {
  model_id: string;
  progress: number;
  downloaded_bytes: number;
  total_bytes: number;
  downloaded_mb: number;
  total_mb: number;
  status: string;
}

export const SHERPA_ASR_PROVIDER_ID = 'sherpa-onnx' as const;
export const SENSEVOICE_MODEL_ID = 'sensevoice-small-int8';
export const PARAFORMER_SMALL_MODEL_ID = 'paraformer-zh-small-int8';
export const PARAFORMER_ONLINE_MODEL_ID = 'paraformer-online-zh-en-int8';
export const QWEN3_ASR_MODEL_ID = 'qwen3-asr-0.6b-int8';

export const SherpaAsrAPI = {
  listModels: () => invoke<SherpaAsrModelStatus[]>('sherpa_asr_list_models'),
  downloadModel: (modelId: string) =>
    invoke<void>('sherpa_asr_download_model', { modelId }),
  deleteModel: (modelId: string) =>
    invoke<void>('sherpa_asr_delete_model', { modelId }),
};

export function supportedLanguageCodes(
  provider: string | undefined,
  model: string | undefined
): string[] | null {
  if (provider === 'localWhisper' || provider === 'whisper' || !provider) {
    return null;
  }
  if (provider === 'parakeet') return ['auto'];
  if (provider !== SHERPA_ASR_PROVIDER_ID) return null;

  switch (model) {
    case SENSEVOICE_MODEL_ID:
      return ['auto', 'zh', 'yue', 'en', 'ja', 'ko'];
    case QWEN3_ASR_MODEL_ID:
      return ['auto', 'zh', 'yue', 'en', 'ja', 'ko', 'de', 'fr', 'es', 'pt', 'ru'];
    case PARAFORMER_SMALL_MODEL_ID:
    case PARAFORMER_ONLINE_MODEL_ID:
    default:
      return ['auto'];
  }
}

export function normalizeLanguageForModel(
  provider: string | undefined,
  model: string | undefined,
  language: string
): string {
  const supported = supportedLanguageCodes(provider, model);
  return !supported || supported.includes(language) ? language : 'auto';
}

export function isAutomaticLanguageOnly(
  provider: string | undefined,
  model: string | undefined
): boolean {
  const supported = supportedLanguageCodes(provider, model);
  return supported?.length === 1 && supported[0] === 'auto';
}

export function isStreamingRecognitionModel(
  provider: string | undefined,
  model: string | undefined
): boolean {
  return provider === SHERPA_ASR_PROVIDER_ID && model === PARAFORMER_ONLINE_MODEL_ID;
}
