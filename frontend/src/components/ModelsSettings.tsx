'use client';

import { useEffect, useState } from 'react';
import { Bot, MessageSquareText, Users } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { ModelManager } from '@/components/WhisperModelManager';
import { ParakeetModelManager } from '@/components/ParakeetModelManager';
import { SpeakerDiarizationModelManager } from '@/components/SpeakerDiarizationModelManager';
import { BuiltInModelManager } from '@/components/BuiltInModelManager';
import { OllamaModelManager } from '@/components/OllamaModelManager';
import { useConfig } from '@/contexts/ConfigContext';
import {
  capabilityConfigService,
  SPEAKER_DIARIZATION_CONFIG_CHANGED_EVENT,
} from '@/services/capabilityConfigService';
import { DEFAULT_SPEAKER_DIARIZATION_CONFIG } from '@/types/capabilities';

interface ModelsSettingsProps {
  onOpenServices: () => void;
}

function ModelSection({
  icon: Icon,
  title,
  description,
  children,
}: {
  icon: typeof Bot;
  title: string;
  description: string;
  children: React.ReactNode;
}) {
  return (
    <section className="rounded-lg border border-gray-200 bg-white p-6 shadow-sm">
      <div className="mb-5 flex items-start gap-3">
        <div className="rounded-md bg-purple-50 p-2 text-purple-700"><Icon className="h-5 w-5" /></div>
        <div>
          <h2 className="text-lg font-semibold text-gray-900">{title}</h2>
          <p className="mt-1 text-sm text-gray-600">{description}</p>
        </div>
      </div>
      {children}
    </section>
  );
}

export function ModelsSettings({ onOpenServices }: ModelsSettingsProps) {
  const { t } = useTranslation('models');
  const { transcriptModelConfig, modelConfig } = useConfig();
  const [speakerEnabled, setSpeakerEnabled] = useState(
    DEFAULT_SPEAKER_DIARIZATION_CONFIG.enabled
  );

  useEffect(() => {
    const refreshSpeakerConfig = () => {
      void capabilityConfigService
        .getSpeakerDiarization()
        .then((config) => setSpeakerEnabled(config.enabled));
    };
    const handleSpeakerConfigChanged = (event: Event) => {
      const config = (event as CustomEvent<{ enabled: boolean }>).detail;
      setSpeakerEnabled(config.enabled);
    };

    refreshSpeakerConfig();
    window.addEventListener(
      SPEAKER_DIARIZATION_CONFIG_CHANGED_EVENT,
      handleSpeakerConfigChanged
    );

    return () => {
      window.removeEventListener(
        SPEAKER_DIARIZATION_CONFIG_CHANGED_EVENT,
        handleSpeakerConfigChanged
      );
    };
  }, []);

  return (
    <div className="space-y-6">
      <ModelSection
        icon={MessageSquareText}
        title={t('sections.transcription')}
        description={t('sections.transcriptionDescription')}
      >
        <div className="space-y-7">
          <div>
            <h3 className="mb-3 text-sm font-semibold">Whisper</h3>
            <ModelManager
              mode="manage"
              selectedModel={transcriptModelConfig.provider === 'localWhisper' ? transcriptModelConfig.model : undefined}
            />
          </div>
          <div className="border-t pt-6">
            <h3 className="mb-3 text-sm font-semibold">Parakeet</h3>
            <ParakeetModelManager
              mode="manage"
              selectedModel={transcriptModelConfig.provider === 'parakeet' ? transcriptModelConfig.model : undefined}
            />
          </div>
        </div>
      </ModelSection>

      <ModelSection
        icon={Users}
        title={t('sections.speaker')}
        description={t('sections.speakerDescription')}
      >
        <SpeakerDiarizationModelManager
          serviceEnabled={speakerEnabled}
          onOpenServices={onOpenServices}
        />
      </ModelSection>

      <ModelSection
        icon={Bot}
        title={t('sections.localSummary')}
        description={t('sections.localSummaryDescription')}
      >
        <BuiltInModelManager
          mode="manage"
          selectedModel={modelConfig.provider === 'builtin-ai' ? modelConfig.model : ''}
        />
      </ModelSection>

      <ModelSection
        icon={Bot}
        title={t('sections.ollama')}
        description={t('sections.ollamaDescription')}
      >
        <OllamaModelManager
          endpoint={modelConfig.ollamaEndpoint}
          selectedModel={modelConfig.provider === 'ollama' ? modelConfig.model : undefined}
        />
      </ModelSection>
    </div>
  );
}
