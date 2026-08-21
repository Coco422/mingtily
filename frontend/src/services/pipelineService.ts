import { invoke } from '@tauri-apps/api/core';
import type { PipelineConfig, ResolvedPipeline } from '@/types/pipeline';
import type { BetaFeatures } from '@/types/betaFeatures';

export const pipelineService = {
  getConfig: () => invoke<PipelineConfig>('pipeline_get_config'),
  resolve: (config: PipelineConfig) =>
    invoke<ResolvedPipeline>('pipeline_resolve_config', { config }),
  save: (config: PipelineConfig) =>
    invoke<ResolvedPipeline>('pipeline_save_config', { config }),
  getBetaFeatures: () => invoke<BetaFeatures>('pipeline_get_beta_features'),
  migrateLegacyBetaFeatures: (legacyImportAndRetranscribe: boolean | null) =>
    invoke<BetaFeatures>('pipeline_migrate_legacy_beta_features', {
      legacyImportAndRetranscribe,
    }),
  saveBetaFeatures: (features: BetaFeatures) =>
    invoke<void>('pipeline_save_beta_features', {
      features: {
        importAndRetranscribe: features.importAndRetranscribe,
        customTranscriptionPipelines: features.customTranscriptionPipelines,
        experimentalAsrModels: features.experimentalAsrModels,
      },
    }),
};
