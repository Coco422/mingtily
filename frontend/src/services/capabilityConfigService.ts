import { invoke } from '@tauri-apps/api/core';
import {
  DEFAULT_SPEAKER_DIARIZATION_CONFIG,
  SpeakerDiarizationConfig,
  TranscriptModelConfig,
} from '@/types/capabilities';
import { configService, ModelConfig } from '@/services/configService';

export const SPEAKER_DIARIZATION_CONFIG_CHANGED_EVENT =
  'mingtily:speaker-diarization-config-changed';

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
