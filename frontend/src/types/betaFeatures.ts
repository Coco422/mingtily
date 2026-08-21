/**
 * Beta Features Type System
 *
 * This file defines the scalable architecture for managing beta features.
 *
 * ## Adding a New Beta Feature
 * 1. Add property to BetaFeatures interface
 * 2. Add default value in DEFAULT_BETA_FEATURES
 * 3. Add the feature to the settings UI
 * 4. Add UI strings in BETA_FEATURE_NAMES and BETA_FEATURE_DESCRIPTIONS
 * 5. Use in components: `betaFeatures.yourFeatureName`
 *
 * ## Graduating a Feature to Stable
 * 1. Remove property from BetaFeatures interface
 * 2. TypeScript will error at all usage sites
 * 3. Remove conditional checks - feature is now always-on
 */

export interface BetaFeatures {
  /**
   * Import audio files and retranscribe existing meetings with different language settings
   * @since v0.3.0
   */
  importAndRetranscribe: boolean;
  customTranscriptionPipelines: boolean;
  experimentalAsrModels: boolean;
}

export const DEFAULT_BETA_FEATURES: BetaFeatures = {
  importAndRetranscribe: true, // Default: enabled
  customTranscriptionPipelines: true,
  experimentalAsrModels: false,
};

/** Safe pre-load state: gated UI stays unreachable until Rust returns its store value. */
export const DISABLED_BETA_FEATURES: BetaFeatures = {
  importAndRetranscribe: false,
  customTranscriptionPipelines: true,
  experimentalAsrModels: false,
};


/**
 * Human-readable feature names for UI display
 */
export const BETA_FEATURE_NAMES: Record<keyof BetaFeatures, string> = {
  importAndRetranscribe: 'Import Audio & Retranscribe',
  customTranscriptionPipelines: 'Custom transcription pipelines',
  experimentalAsrModels: 'Experimental ASR models',
};

/**
 * Feature descriptions for UI tooltips/help text
 */
export const BETA_FEATURE_DESCRIPTIONS: Record<keyof BetaFeatures, string> = {
  importAndRetranscribe: 'Import audio files to transcribe or retranscribe existing meetings with different language settings.',
  customTranscriptionPipelines: 'Choose live, finalized, and post-meeting processing paths.',
  experimentalAsrModels: 'Show experimental streaming and high-resource speech recognition models.',
};

/**
 * Type-safe feature key union
 * This ensures only valid feature keys can be used
 */
export type BetaFeatureKey = keyof BetaFeatures;

export const BETA_FEATURES_CHANGED_EVENT = 'mingtily-beta-features-changed';

export function readLegacyImportAndRetranscribe(): boolean | null {
  if (typeof window === 'undefined') return null;
  try {
    const saved = localStorage.getItem('betaFeatures');
    if (!saved) return null;
    const parsed = JSON.parse(saved) as { importAndRetranscribe?: unknown };
    return typeof parsed.importAndRetranscribe === 'boolean'
      ? parsed.importAndRetranscribe
      : null;
  } catch {
    return null;
  }
}
