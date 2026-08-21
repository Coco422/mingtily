export type PipelinePreset = 'fast' | 'balanced' | 'quality' | 'custom';
export type LiveMode = 'off' | 'vad-segmented' | 'continuous-preview';
export type PostMeetingPolicy = 'off' | 'manual' | 'auto';
export type SpeakerRefinementPolicy = 'off' | 'manual' | 'background-auto';
export type ResourceMode = 'eco' | 'balanced' | 'fast';

export interface PipelineConfig {
  version: 1;
  preset: PipelinePreset;
  live: {
    mode: LiveMode;
    streamingProvider: string | null;
    streamingModel: string | null;
  };
  finalized: { provider: string; model: string; language: string };
  postMeetingAsr: {
    policy: PostMeetingPolicy;
    provider: string | null;
    model: string | null;
  };
  speaker: {
    liveEnabled: boolean;
    refinement: SpeakerRefinementPolicy;
    speakerCount: number | null;
  };
  enhancements: { punctuation: 'auto' | 'off'; terminology: 'auto' | 'off' };
  resources: {
    mode: ResourceMode;
    memoryLimitMiB: number | null;
    runAutomaticJobsOnBattery: boolean;
    pauseAutomaticJobsDuringRecording: true;
  };
}

export interface ModelCapabilities {
  provider: string;
  model: string;
  inputMode: 'continuous' | 'vad-segmented' | 'whole-file';
  outputs: string[];
  languages: string[];
  supportsHotwords: boolean;
  builtInPunctuation: boolean;
  recommendedThreads: number;
  fixedMemoryMiB: number;
  workerMemoryMiB: number;
  maxParallelism: number;
  maxAudioSeconds: number | null;
  supportedPlatforms: string[];
  betaGate: string | null;
}

export interface ResolvedPipeline {
  config: PipelineConfig;
  effectiveConfig?: PipelineConfig | null;
  liveCapabilities: ModelCapabilities | null;
  finalizedCapabilities: ModelCapabilities;
  postMeetingCapabilities: ModelCapabilities | null;
  speakerCapabilities?: ModelCapabilities | null;
  punctuationEnabled: boolean;
  speakerRefinementEnabled: boolean;
  estimatedMemoryMiB: number;
  workerCount: number;
  threadCount: number;
  decisions: string[];
}
