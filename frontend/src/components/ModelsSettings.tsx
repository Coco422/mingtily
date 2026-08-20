'use client';

import { Component, useEffect, useState, type ErrorInfo, type ReactNode } from 'react';
import { AlertTriangle, AudioLines, Bot, ChevronDown, FileCog, MessageSquareText, Users } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { ModelManager } from '@/components/WhisperModelManager';
import { ParakeetModelManager } from '@/components/ParakeetModelManager';
import { SherpaAsrModelManager } from '@/components/SherpaAsrModelManager';
import { PunctuationModelManager } from '@/components/PunctuationModelManager';
import { HomophoneReplacerManager } from '@/components/HomophoneReplacerManager';
import { SpeakerDiarizationModelManager } from '@/components/SpeakerDiarizationModelManager';
import { BuiltInModelManager } from '@/components/BuiltInModelManager';
import { OllamaModelManager } from '@/components/OllamaModelManager';
import { useConfig } from '@/contexts/ConfigContext';
import {
  capabilityConfigService,
  SPEAKER_DIARIZATION_CONFIG_CHANGED_EVENT,
  STREAMING_TRANSCRIPTION_CONFIG_CHANGED_EVENT,
} from '@/services/capabilityConfigService';
import { DEFAULT_SPEAKER_DIARIZATION_CONFIG } from '@/types/capabilities';
import { Button } from '@/components/ui/button';
import type { StreamingTranscriptionConfig } from '@/lib/sherpa-asr';

interface ModelsSettingsProps {
  onOpenServices: () => void;
}

interface ModelsSettingsErrorBoundaryProps {
  children: ReactNode;
  title: string;
  description: string;
  retryLabel: string;
}

interface ModelsSettingsErrorBoundaryState {
  error: Error | null;
}

class ModelsSettingsErrorBoundary extends Component<
  ModelsSettingsErrorBoundaryProps,
  ModelsSettingsErrorBoundaryState
> {
  state: ModelsSettingsErrorBoundaryState = { error: null };

  static getDerivedStateFromError(error: Error): ModelsSettingsErrorBoundaryState {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error('[ModelsSettings] Rendering failed', error, info);
  }

  render() {
    if (!this.state.error) return this.props.children;

    return (
      <section className="rounded-lg border border-red-200 bg-red-50/60 p-5">
        <div className="flex items-start gap-3">
          <AlertTriangle className="mt-0.5 h-5 w-5 shrink-0 text-red-600" />
          <div className="min-w-0 flex-1">
            <h2 className="text-sm font-semibold text-red-900">{this.props.title}</h2>
            <p className="mt-1 text-xs leading-5 text-red-800">{this.props.description}</p>
            <pre className="mt-3 overflow-x-auto rounded-md border border-red-200 bg-white/80 p-3 text-xs text-red-900">
              {this.state.error.message}
            </pre>
            <Button
              variant="outline"
              size="sm"
              className="mt-3 border-red-200 bg-white text-red-800 hover:bg-red-100"
              onClick={() => this.setState({ error: null })}
            >
              {this.props.retryLabel}
            </Button>
          </div>
        </div>
      </section>
    );
  }
}

function ModelSection({
  icon: Icon,
  title,
  description,
  defaultOpen = false,
  children,
}: {
  icon: typeof Bot;
  title: string;
  description: string;
  defaultOpen?: boolean;
  children: React.ReactNode;
}) {
  const [open, setOpen] = useState(defaultOpen);

  return (
    <section className="overflow-hidden rounded-lg border border-black/[0.08] bg-white">
      <button
        type="button"
        aria-expanded={open}
        onClick={() => setOpen((current) => !current)}
        className="flex w-full items-start gap-3 bg-gray-50/60 px-5 py-4 text-left transition-colors hover:bg-gray-100/70 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-sky-500 focus-visible:ring-inset"
      >
        <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-md border border-sky-100 bg-sky-50 text-sky-700">
          <Icon className="h-4 w-4" />
        </div>
        <div className="min-w-0 flex-1">
          <h2 className="text-base font-semibold leading-6 text-gray-900">{title}</h2>
          <p className="mt-0.5 text-xs leading-5 text-gray-600">{description}</p>
        </div>
        <ChevronDown
          className={`mt-1 h-4 w-4 shrink-0 text-gray-500 transition-transform duration-150 ${open ? 'rotate-180' : ''}`}
        />
      </button>
      {open && <div className="border-t border-black/[0.08] p-4">{children}</div>}
    </section>
  );
}

function ModelProviderGroup({
  title,
  description,
  defaultOpen = false,
  children,
}: {
  title: string;
  description: string;
  defaultOpen?: boolean;
  children: React.ReactNode;
}) {
  const [open, setOpen] = useState(defaultOpen);

  return (
    <div className="rounded-md border border-black/[0.08] bg-white">
      <button
        type="button"
        aria-expanded={open}
        onClick={() => setOpen((current) => !current)}
        className="flex w-full items-center gap-3 px-3 py-3 text-left transition-colors hover:bg-gray-50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-sky-500 focus-visible:ring-inset"
      >
        <div className="min-w-0 flex-1">
          <div className="text-sm font-semibold text-gray-900">{title}</div>
          <div className="mt-0.5 text-xs leading-5 text-gray-500">{description}</div>
        </div>
        <ChevronDown
          className={`h-4 w-4 shrink-0 text-gray-500 transition-transform duration-150 ${open ? 'rotate-180' : ''}`}
        />
      </button>
      {open && <div className="border-t border-black/[0.08] p-3">{children}</div>}
    </div>
  );
}

function ModelsSettingsContent({ onOpenServices }: ModelsSettingsProps) {
  const { t } = useTranslation('models');
  const { transcriptModelConfig, modelConfig } = useConfig();
  const [speakerEnabled, setSpeakerEnabled] = useState(
    DEFAULT_SPEAKER_DIARIZATION_CONFIG.enabled
  );
  const [streamingConfig, setStreamingConfig] = useState<StreamingTranscriptionConfig | null>(
    null
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

  useEffect(() => {
    void capabilityConfigService
      .getStreamingTranscription()
      .then(setStreamingConfig)
      .catch((error) =>
        console.warn('[ModelsSettings] Unable to load streaming transcription config', error)
      );

    const handleStreamingConfigChanged = (event: Event) => {
      setStreamingConfig(
        (event as CustomEvent<StreamingTranscriptionConfig>).detail
      );
    };
    window.addEventListener(
      STREAMING_TRANSCRIPTION_CONFIG_CHANGED_EVENT,
      handleStreamingConfigChanged
    );
    return () => {
      window.removeEventListener(
        STREAMING_TRANSCRIPTION_CONFIG_CHANGED_EVENT,
        handleStreamingConfigChanged
      );
    };
  }, []);

  return (
    <div className="space-y-4">
      <ModelSection
        icon={AudioLines}
        title={t('sections.transcription')}
        description={t('sections.transcriptionDescription')}
        defaultOpen
      >
        <div className="space-y-3">
          <ModelProviderGroup
            title={t('sections.providerGroups.sherpa')}
            description={t('sections.providerGroups.sherpaDescription')}
            defaultOpen={transcriptModelConfig.provider === 'sherpa-onnx'}
          >
            <SherpaAsrModelManager
              selectedModel={
                transcriptModelConfig.provider === 'sherpa-onnx'
                  ? transcriptModelConfig.model
                  : undefined
              }
              additionalSelectedModels={
                streamingConfig?.enabled ? [streamingConfig.model] : []
              }
              onOpenServices={onOpenServices}
            />
          </ModelProviderGroup>
          <ModelProviderGroup
            title={t('sections.providerGroups.whisper')}
            description={t('sections.providerGroups.whisperDescription')}
          >
            <ModelManager
              mode="manage"
              selectedModel={transcriptModelConfig.provider === 'localWhisper' ? transcriptModelConfig.model : undefined}
            />
          </ModelProviderGroup>
          <ModelProviderGroup
            title={t('sections.providerGroups.parakeet')}
            description={t('sections.providerGroups.parakeetDescription')}
            defaultOpen={transcriptModelConfig.provider === 'parakeet'}
          >
            <ParakeetModelManager
              mode="manage"
              selectedModel={transcriptModelConfig.provider === 'parakeet' ? transcriptModelConfig.model : undefined}
            />
          </ModelProviderGroup>
        </div>
      </ModelSection>

      <ModelSection
        icon={MessageSquareText}
        title={t('sections.punctuation')}
        description={t('sections.punctuationDescription')}
      >
        <PunctuationModelManager />
      </ModelSection>

      <ModelSection
        icon={FileCog}
        title={t('sections.terminology')}
        description={t('sections.terminologyDescription')}
      >
        <HomophoneReplacerManager />
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

export function ModelsSettings(props: ModelsSettingsProps) {
  const { t } = useTranslation('models');

  return (
    <ModelsSettingsErrorBoundary
      title={t('errors.pageFailed')}
      description={t('errors.pageFailedDescription')}
      retryLabel={t('actions.retry')}
    >
      <ModelsSettingsContent {...props} />
    </ModelsSettingsErrorBoundary>
  );
}
