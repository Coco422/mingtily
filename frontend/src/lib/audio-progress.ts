import type { TFunction } from 'i18next';

const LOCALIZED_STAGES = new Set([
  'copying',
  'decoding',
  'resampling',
  'vad',
  'diarization',
  'transcribing',
  'saving',
  'complete',
]);

export function localizeAudioProgress(
  t: TFunction,
  stage: string,
  fallbackMessage: string,
) {
  if (!LOCALIZED_STAGES.has(stage)) {
    return { stage, message: fallbackMessage };
  }

  return {
    stage: t(`audioProgress.${stage}.stage`),
    message: t(`audioProgress.${stage}.message`),
  };
}
