import { invoke } from '@tauri-apps/api/core';
import { notifyModelAssetsChanged } from '@/lib/model-assets-events';

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

export interface ImportedSherpaModel {
  model_id: string;
  path: string;
}

export interface StreamingTranscriptionConfig {
  enabled: boolean;
  provider: typeof SHERPA_ASR_PROVIDER_ID;
  model: string;
}

export interface SherpaAsrEnhancementConfig {
  hotwords: string[];
  homophoneReplacerEnabled: boolean;
  homophoneRuleFsts: string[];
}

export interface TerminologyReplacement {
  source: string;
  target: string;
}

export interface TerminologyConfig {
  terms: string[];
  replacements: TerminologyReplacement[];
  homophoneReplacerEnabled: boolean;
  homophoneRuleFsts: string[];
}

export interface HomophoneRuleStatus {
  id: string;
  name: string;
  size: number;
}

export interface HomophoneReplacerStatus {
  id: string;
  name: string;
  status: SherpaAsrModelState;
  download_size: number;
  installed_size: number;
  license: string;
  path: string;
  error?: string | null;
  rules: HomophoneRuleStatus[];
}

export const SHERPA_ASR_PROVIDER_ID = 'sherpa-onnx' as const;
export const SENSEVOICE_MODEL_ID = 'sensevoice-small-int8';
export const PARAFORMER_SMALL_MODEL_ID = 'paraformer-zh-small-int8';
export const PARAFORMER_ONLINE_MODEL_ID = 'paraformer-online-zh-en-int8';
export const QWEN3_ASR_MODEL_ID = 'qwen3-asr-0.6b-int8';
export const FUNASR_NANO_MODEL_ID = 'funasr-nano-int8';

export const DEFAULT_SHERPA_ASR_ENHANCEMENT_CONFIG: SherpaAsrEnhancementConfig = {
  hotwords: [],
  homophoneReplacerEnabled: false,
  homophoneRuleFsts: [],
};

export const DEFAULT_TERMINOLOGY_CONFIG: TerminologyConfig = {
  terms: [],
  replacements: [],
  homophoneReplacerEnabled: false,
  homophoneRuleFsts: [],
};

export const SherpaAsrAPI = {
  listModels: () => invoke<SherpaAsrModelStatus[]>('sherpa_asr_list_models'),
  getStreamingConfig: () =>
    invoke<StreamingTranscriptionConfig>('sherpa_asr_get_streaming_config'),
  saveStreamingConfig: (config: StreamingTranscriptionConfig) =>
    invoke<void>('sherpa_asr_save_streaming_config', { config }),
  getEnhancementConfig: () =>
    invoke<SherpaAsrEnhancementConfig>('sherpa_asr_get_enhancement_config'),
  saveEnhancementConfig: (config: SherpaAsrEnhancementConfig) =>
    invoke<SherpaAsrEnhancementConfig>('sherpa_asr_save_enhancement_config', { config }),
  getTerminologyConfig: () => invoke<TerminologyConfig>('terminology_get_config'),
  saveTerminologyConfig: (config: TerminologyConfig) =>
    invoke<TerminologyConfig>('terminology_save_config', { config }),
  getHomophoneStatus: () =>
    invoke<HomophoneReplacerStatus>('sherpa_asr_get_homophone_status'),
  downloadHomophoneLexicon: () =>
    invoke<void>('sherpa_asr_download_homophone_lexicon'),
  deleteHomophoneLexicon: () =>
    invoke<void>('sherpa_asr_delete_homophone_lexicon'),
  importHomophoneRules: () =>
    invoke<HomophoneRuleStatus[]>('sherpa_asr_import_homophone_rules'),
  deleteHomophoneRule: (ruleId: string) =>
    invoke<HomophoneRuleStatus[]>('sherpa_asr_delete_homophone_rule', { ruleId }),
  downloadModel: async (modelId: string) => {
    await invoke<void>('sherpa_asr_download_model', { modelId });
    notifyModelAssetsChanged('sherpa-onnx');
  },
  importModelArchive: async () => {
    const result = await invoke<ImportedSherpaModel | null>('sherpa_asr_import_model_archive');
    if (result) notifyModelAssetsChanged('sherpa-onnx');
    return result;
  },
  importModelDirectory: async () => {
    const result = await invoke<ImportedSherpaModel | null>('sherpa_asr_import_model_directory');
    if (result) notifyModelAssetsChanged('sherpa-onnx');
    return result;
  },
  deleteModel: async (modelId: string) => {
    await invoke<void>('sherpa_asr_delete_model', { modelId });
    notifyModelAssetsChanged('sherpa-onnx');
  },
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
    case FUNASR_NANO_MODEL_ID:
    case PARAFORMER_SMALL_MODEL_ID:
    case PARAFORMER_ONLINE_MODEL_ID:
    default:
      return ['auto'];
  }
}

export function supportsDynamicHotwords(
  provider: string | undefined,
  model: string | undefined
): boolean {
  return provider === SHERPA_ASR_PROVIDER_ID &&
    (model === QWEN3_ASR_MODEL_ID || model === FUNASR_NANO_MODEL_ID);
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
