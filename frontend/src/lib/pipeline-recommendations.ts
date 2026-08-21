import { invoke } from '@tauri-apps/api/core';
import { notifyModelAssetsChanged } from '@/lib/model-assets-events';
import { PunctuationAPI } from '@/lib/punctuation';
import {
  FUNASR_NANO_MODEL_ID,
  PARAFORMER_SMALL_MODEL_ID,
  SENSEVOICE_MODEL_ID,
  SherpaAsrAPI,
  type SherpaAsrModelStatus,
} from '@/lib/sherpa-asr';
import type { PipelineConfig, PipelinePreset } from '@/types/pipeline';

export type RecommendedPipelinePreset = Exclude<PipelinePreset, 'custom'>;
export type PipelineAssetKind = 'asr' | 'speaker' | 'punctuation';

export interface PipelineAssetRequirement {
  key: string;
  kind: PipelineAssetKind;
  modelId?: string;
  name: string;
  downloadSizeMiB: number;
  installed: boolean;
  corrupt: boolean;
}

interface SpeakerStatus {
  status: 'available' | 'missing' | 'corrupt';
  size_mb: number;
}

export const PIPELINE_RECOMMENDATIONS: Record<RecommendedPipelinePreset, {
  asrModels: string[];
  speaker: boolean;
  punctuation: boolean;
  experimental: boolean;
}> = {
  fast: {
    asrModels: [PARAFORMER_SMALL_MODEL_ID],
    speaker: false,
    punctuation: false,
    experimental: false,
  },
  balanced: {
    asrModels: [SENSEVOICE_MODEL_ID],
    speaker: true,
    punctuation: true,
    experimental: false,
  },
  quality: {
    asrModels: [SENSEVOICE_MODEL_ID, FUNASR_NANO_MODEL_ID],
    speaker: true,
    punctuation: true,
    experimental: true,
  },
};

export function applyRecommendedPipeline(
  current: PipelineConfig,
  preset: PipelinePreset
): PipelineConfig {
  if (preset === 'custom') return { ...current, preset };

  const finalizedModel = preset === 'fast'
    ? PARAFORMER_SMALL_MODEL_ID
    : SENSEVOICE_MODEL_ID;
  const quality = preset === 'quality';

  return {
    ...current,
    preset,
    live: { mode: 'vad-segmented', streamingProvider: null, streamingModel: null },
    finalized: {
      provider: 'sherpa-onnx',
      model: finalizedModel,
      language: preset === 'fast' ? 'auto' : 'zh',
    },
    postMeetingAsr: quality
      ? { policy: 'auto', provider: 'sherpa-onnx', model: FUNASR_NANO_MODEL_ID }
      : { policy: 'off', provider: null, model: null },
    speaker: {
      ...current.speaker,
      liveEnabled: preset !== 'fast',
      refinement: preset === 'fast' ? 'off' : 'background-auto',
    },
    enhancements: {
      punctuation: preset === 'fast' ? 'off' : 'auto',
      terminology: preset === 'fast' ? 'off' : 'auto',
    },
    resources: {
      ...current.resources,
      mode: preset === 'fast' ? 'eco' : quality ? 'fast' : 'balanced',
      memoryLimitMiB: null,
    },
  };
}

export async function loadPipelineAssetRequirements(
  preset: RecommendedPipelinePreset
): Promise<PipelineAssetRequirement[]> {
  const recommendation = PIPELINE_RECOMMENDATIONS[preset];
  const [sherpaModels, speakerStatus, punctuationStatus] = await Promise.all([
    SherpaAsrAPI.listModels(),
    recommendation.speaker
      ? invoke<SpeakerStatus>('speaker_diarization_get_status')
      : Promise.resolve(null),
    recommendation.punctuation ? PunctuationAPI.getStatus() : Promise.resolve(null),
  ]);

  const sherpaById = new Map<string, SherpaAsrModelStatus>(
    sherpaModels.map((model) => [model.id, model])
  );
  const assets: PipelineAssetRequirement[] = recommendation.asrModels.map((modelId) => {
    const model = sherpaById.get(modelId);
    return {
      key: `asr:${modelId}`,
      kind: 'asr',
      modelId,
      name: model?.name ?? modelId,
      downloadSizeMiB: Math.round((model?.download_size ?? 0) / 1024 / 1024),
      installed: model?.status === 'available',
      corrupt: model?.status === 'corrupt',
    };
  });

  if (speakerStatus) {
    assets.push({
      key: 'speaker',
      kind: 'speaker',
      name: 'Speaker diarization',
      downloadSizeMiB: Math.round(speakerStatus.size_mb),
      installed: speakerStatus.status === 'available',
      corrupt: speakerStatus.status === 'corrupt',
    });
  }
  if (punctuationStatus) {
    assets.push({
      key: 'punctuation',
      kind: 'punctuation',
      name: punctuationStatus.name,
      downloadSizeMiB: Math.round(punctuationStatus.download_size / 1024 / 1024),
      installed: punctuationStatus.status === 'available',
      corrupt: punctuationStatus.status === 'corrupt',
    });
  }
  return assets;
}

export async function downloadPipelineAsset(asset: PipelineAssetRequirement): Promise<void> {
  if (asset.kind === 'asr' && asset.modelId) {
    await SherpaAsrAPI.downloadModel(asset.modelId);
  } else if (asset.kind === 'speaker') {
    await invoke('speaker_diarization_download_model');
    notifyModelAssetsChanged('speaker-diarization');
  } else if (asset.kind === 'punctuation') {
    await PunctuationAPI.downloadModel();
    notifyModelAssetsChanged('punctuation');
  }
}
