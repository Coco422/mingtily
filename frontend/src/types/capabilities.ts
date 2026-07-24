export type CapabilityId = 'transcription' | 'speakerDiarization' | 'summary';

export type ProviderBoundary = 'local' | 'loopback' | 'remote';

export type TranscriptProviderId =
  | 'localWhisper'
  | 'parakeet'
  | 'sherpa-onnx'
  | 'deepgram'
  | 'elevenLabs'
  | 'groq'
  | 'openai';

export type SummaryProviderId =
  | 'ollama'
  | 'groq'
  | 'claude'
  | 'openai'
  | 'openrouter'
  | 'builtin-ai'
  | 'custom-openai';

export type SpeakerProviderId = 'sherpa-onnx';

export interface ProviderDescriptor<TProvider extends string = string> {
  id: TProvider;
  capabilities: CapabilityId[];
  boundary: ProviderBoundary;
}

export interface TranscriptModelConfig {
  provider: TranscriptProviderId;
  model: string;
  apiKey?: string | null;
}

export interface SpeakerDiarizationConfig {
  enabled: boolean;
  provider: SpeakerProviderId;
  model: 'sherpa-v1';
}

export const DEFAULT_SPEAKER_DIARIZATION_CONFIG: SpeakerDiarizationConfig = {
  enabled: true,
  provider: 'sherpa-onnx',
  model: 'sherpa-v1',
};

export const PROVIDERS: ProviderDescriptor[] = [
  { id: 'localWhisper', capabilities: ['transcription'], boundary: 'local' },
  { id: 'parakeet', capabilities: ['transcription'], boundary: 'local' },
  { id: 'sherpa-onnx', capabilities: ['transcription', 'speakerDiarization'], boundary: 'local' },
  { id: 'builtin-ai', capabilities: ['summary'], boundary: 'local' },
  { id: 'ollama', capabilities: ['summary'], boundary: 'loopback' },
  { id: 'custom-openai', capabilities: ['summary'], boundary: 'remote' },
  { id: 'openai', capabilities: ['summary'], boundary: 'remote' },
  { id: 'claude', capabilities: ['summary'], boundary: 'remote' },
  { id: 'groq', capabilities: ['summary'], boundary: 'remote' },
  { id: 'openrouter', capabilities: ['summary'], boundary: 'remote' },
];
