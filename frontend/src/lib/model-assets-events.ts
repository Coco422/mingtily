export const MODEL_ASSETS_CHANGED_EVENT = 'mingtily:model-assets-changed';

export type ModelAssetProvider =
  | 'localWhisper'
  | 'parakeet'
  | 'sherpa-onnx'
  | 'speaker-diarization'
  | 'punctuation';

export function notifyModelAssetsChanged(provider: ModelAssetProvider) {
  if (typeof window === 'undefined') return;
  window.dispatchEvent(
    new CustomEvent<{ provider: ModelAssetProvider }>(MODEL_ASSETS_CHANGED_EVENT, {
      detail: { provider },
    })
  );
}
