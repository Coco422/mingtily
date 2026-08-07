import { invoke } from '@tauri-apps/api/core';
import {
  DEFAULT_SPEAKER_DIARIZATION_CONFIG,
  SpeakerDiarizationConfig,
  TranscriptModelConfig,
} from '@/types/capabilities';
import { configService, ModelConfig } from '@/services/configService';
import {
  SherpaAsrAPI,
  SherpaAsrEnhancementConfig,
  StreamingTranscriptionConfig,
} from '@/lib/sherpa-asr';

export const SPEAKER_DIARIZATION_CONFIG_CHANGED_EVENT =
  'mingtily:speaker-diarization-config-changed';
export const STREAMING_TRANSCRIPTION_CONFIG_CHANGED_EVENT =
  'mingtily:streaming-transcription-config-changed';
export const SHERPA_ASR_ENHANCEMENT_CONFIG_CHANGED_EVENT =
  'mingtily:sherpa-asr-enhancement-config-changed';

class CapabilityConfigService {
  getTranscription(): Promise<TranscriptModelConfig> {
    return configService.getTranscriptConfig();
  }

  saveTranscription(config: TranscriptModelConfig): Promise<void> {
    return invoke('api_save_transcript_config', {
      provider: config.provider,
      model: config.model,
      apiKey: config.apiKey ?? null,
    });
  }

  getStreamingTranscription(): Promise<StreamingTranscriptionConfig> {
    return SherpaAsrAPI.getStreamingConfig();
  }

  async saveStreamingTranscription(config: StreamingTranscriptionConfig): Promise<void> {
    await SherpaAsrAPI.saveStreamingConfig(config);
    if (typeof window !== 'undefined') {
      window.dispatchEvent(
        new CustomEvent<StreamingTranscriptionConfig>(
          STREAMING_TRANSCRIPTION_CONFIG_CHANGED_EVENT,
          { detail: config }
        )
      );
    }
  }

  getSherpaAsrEnhancements(): Promise<SherpaAsrEnhancementConfig> {
    return SherpaAsrAPI.getEnhancementConfig();
  }

  async saveSherpaAsrEnhancements(
    config: SherpaAsrEnhancementConfig
  ): Promise<SherpaAsrEnhancementConfig> {
    const saved = await SherpaAsrAPI.saveEnhancementConfig(config);
    if (typeof window !== 'undefined') {
      window.dispatchEvent(
        new CustomEvent<SherpaAsrEnhancementConfig>(
          SHERPA_ASR_ENHANCEMENT_CONFIG_CHANGED_EVENT,
          { detail: saved }
        )
      );
    }
    return saved;
  }

  async getSpeakerDiarization(): Promise<SpeakerDiarizationConfig> {
    try {
      const config = await invoke<Partial<SpeakerDiarizationConfig>>(
        'speaker_diarization_get_config'
      );
      return { ...DEFAULT_SPEAKER_DIARIZATION_CONFIG, ...config };
    } catch (error) {
      console.warn(
        '[CapabilityConfig] Falling back to the compatible speaker default:',
        error
      );
      return { ...DEFAULT_SPEAKER_DIARIZATION_CONFIG };
    }
  }

  async saveSpeakerDiarization(config: SpeakerDiarizationConfig): Promise<void> {
    await invoke('speaker_diarization_save_config', { config });
    if (typeof window !== 'undefined') {
      window.dispatchEvent(
        new CustomEvent<SpeakerDiarizationConfig>(
          SPEAKER_DIARIZATION_CONFIG_CHANGED_EVENT,
          { detail: config }
        )
      );
    }
  }

  getSummary(): Promise<ModelConfig> {
    return configService.getModelConfig();
  }
}

export const capabilityConfigService = new CapabilityConfigService();
