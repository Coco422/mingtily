'use client';

import { useCallback, useEffect, useMemo, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { AlertTriangle, Bot, MessageSquareText, Users } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';
import { Button } from '@/components/ui/button';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Switch } from '@/components/ui/switch';
import { LanguageSelection } from '@/components/LanguageSelection';
import { SummaryModelSettings } from '@/components/SummaryModelSettings';
import { useConfig } from '@/contexts/ConfigContext';
import { WhisperAPI } from '@/lib/whisper';
import { ParakeetAPI } from '@/lib/parakeet';
import { SherpaAsrAPI, SherpaAsrModelStatus } from '@/lib/sherpa-asr';
import { capabilityConfigService } from '@/services/capabilityConfigService';
import {
  DEFAULT_SPEAKER_DIARIZATION_CONFIG,
  SpeakerDiarizationConfig,
  TranscriptProviderId,
} from '@/types/capabilities';

interface ServicesSettingsProps {
  onOpenModels: () => void;
}

interface ServiceCardProps {
  icon: typeof Bot;
  title: string;
  description: string;
  children: React.ReactNode;
}

function ServiceCard({ icon: Icon, title, description, children }: ServiceCardProps) {
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

function readProviderModelMap(): Record<string, string> {
  try {
    const parsed = JSON.parse(localStorage.getItem('providerModelMap') || '{}');
    return parsed && typeof parsed === 'object' && !Array.isArray(parsed) ? parsed : {};
  } catch {
    return {};
  }
}

export function ServicesSettings({ onOpenModels }: ServicesSettingsProps) {
  const { t } = useTranslation(['settings', 'models']);
  const {
    transcriptModelConfig,
    setTranscriptModelConfig,
    selectedLanguage,
    setSelectedLanguage,
    modelConfig,
  } = useConfig();
  const [transcriptProvider, setTranscriptProvider] = useState<TranscriptProviderId>(
    transcriptModelConfig.provider
  );
  const [transcriptModel, setTranscriptModel] = useState(transcriptModelConfig.model);
  const [whisperModels, setWhisperModels] = useState<string[]>([]);
  const [parakeetModels, setParakeetModels] = useState<string[]>([]);
  const [sherpaModels, setSherpaModels] = useState<SherpaAsrModelStatus[]>([]);
  const [loadingModels, setLoadingModels] = useState(true);
  const [savingTranscript, setSavingTranscript] = useState(false);
  const [speakerConfig, setSpeakerConfig] = useState<SpeakerDiarizationConfig>(
    DEFAULT_SPEAKER_DIARIZATION_CONFIG
  );
  const [speakerInstalled, setSpeakerInstalled] = useState(false);
  const [savingSpeaker, setSavingSpeaker] = useState(false);

  const loadLocalModels = useCallback(async () => {
    setLoadingModels(true);
    try {
      const [whisperResult, parakeetResult, sherpaResult, speakerResult] = await Promise.allSettled([
        WhisperAPI.init().then(() => WhisperAPI.getAvailableModels()),
        ParakeetAPI.init().then(() => ParakeetAPI.getAvailableModels()),
        SherpaAsrAPI.listModels(),
        capabilityConfigService.getSpeakerDiarization(),
      ]);
      if (whisperResult.status === 'fulfilled') {
        setWhisperModels(
          whisperResult.value.filter((model) => model.status === 'Available').map((model) => model.name)
        );
      }
      if (parakeetResult.status === 'fulfilled') {
        setParakeetModels(
          parakeetResult.value.filter((model) => model.status === 'Available').map((model) => model.name)
        );
      }
      if (sherpaResult.status === 'fulfilled') {
        setSherpaModels(sherpaResult.value);
      }
      if (speakerResult.status === 'fulfilled') setSpeakerConfig(speakerResult.value);

      try {
        const status = await invoke<{ status: string }>('speaker_diarization_get_status');
        setSpeakerInstalled(status.status === 'available');
      } catch {
        setSpeakerInstalled(false);
      }
    } finally {
      setLoadingModels(false);
    }
  }, []);

  useEffect(() => {
    void loadLocalModels();
  }, [loadLocalModels]);

  useEffect(() => {
    setTranscriptProvider(transcriptModelConfig.provider);
    setTranscriptModel(transcriptModelConfig.model);
  }, [transcriptModelConfig]);

  const installedTranscriptModels = useMemo(() => {
    if (transcriptProvider === 'parakeet') return parakeetModels;
    if (transcriptProvider === 'sherpa-onnx') {
      return sherpaModels
        .filter((model) => model.status === 'available')
        .map((model) => model.id);
    }
    return whisperModels;
  }, [parakeetModels, sherpaModels, transcriptProvider, whisperModels]);

  const transcriptModelLabel = (modelId: string) =>
    sherpaModels.find((model) => model.id === modelId)?.name || modelId;

  const changeTranscriptProvider = (provider: TranscriptProviderId) => {
    setTranscriptProvider(provider);
    const available = provider === 'parakeet'
      ? parakeetModels
      : provider === 'sherpa-onnx'
        ? sherpaModels
            .filter((model) => model.status === 'available')
            .map((model) => model.id)
        : whisperModels;
    const remembered = readProviderModelMap()[provider];
    setTranscriptModel(available.includes(remembered) ? remembered : (available[0] || ''));
  };

  const saveTranscription = async () => {
    if (!transcriptModel || !installedTranscriptModels.includes(transcriptModel)) return;
    setSavingTranscript(true);
    const config = { provider: transcriptProvider, model: transcriptModel, apiKey: null };
    try {
      await capabilityConfigService.saveTranscription(config);
      setTranscriptModelConfig(config);
      const map = readProviderModelMap();
      map[transcriptProvider] = transcriptModel;
      localStorage.setItem('providerModelMap', JSON.stringify(map));
      toast.success(t('settings:services.transcription.saved'));
    } catch (error) {
      toast.error(t('settings:services.transcription.saveFailed'), { description: String(error) });
    } finally {
      setSavingTranscript(false);
    }
  };

  const saveSpeaker = async () => {
    setSavingSpeaker(true);
    try {
      await capabilityConfigService.saveSpeakerDiarization(speakerConfig);
      toast.success(t('settings:services.speaker.saved'));
    } catch (error) {
      toast.error(t('settings:services.speaker.saveFailed'), { description: String(error) });
    } finally {
      setSavingSpeaker(false);
    }
  };

  const isRemoteSummary = ['claude', 'groq', 'openai', 'openrouter', 'custom-openai'].includes(
    modelConfig.provider
  );

  return (
    <div className="space-y-6">
      <ServiceCard
        icon={MessageSquareText}
        title={t('settings:services.transcription.title')}
        description={t('settings:services.transcription.description')}
      >
        <div className="grid gap-4 md:grid-cols-2">
          <div className="space-y-2">
            <label className="text-sm font-medium">{t('settings:services.provider')}</label>
            <Select value={transcriptProvider} onValueChange={(value) => changeTranscriptProvider(value as TranscriptProviderId)}>
              <SelectTrigger><SelectValue /></SelectTrigger>
              <SelectContent>
                <SelectItem value="parakeet">Parakeet</SelectItem>
                <SelectItem value="localWhisper">Whisper</SelectItem>
                <SelectItem value="sherpa-onnx">Sherpa ONNX</SelectItem>
              </SelectContent>
            </Select>
          </div>
          <div className="space-y-2">
            <label className="text-sm font-medium">{t('settings:services.model')}</label>
            <Select value={transcriptModel} onValueChange={setTranscriptModel} disabled={loadingModels || installedTranscriptModels.length === 0}>
              <SelectTrigger><SelectValue placeholder={t('settings:services.selectInstalledModel')} /></SelectTrigger>
              <SelectContent>
                {installedTranscriptModels.map((model) => (
                  <SelectItem key={model} value={model}>
                    {transcriptModelLabel(model)}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
        </div>

        {installedTranscriptModels.length === 0 && !loadingModels && (
          <div className="mt-4 flex items-center justify-between gap-3 rounded-md border border-amber-200 bg-amber-50 p-3 text-sm text-amber-800">
            <span>{t('settings:services.noInstalledModel')}</span>
            <Button variant="outline" size="sm" onClick={onOpenModels}>{t('settings:tabs.models')}</Button>
          </div>
        )}

        <div className="mt-5 border-t pt-5">
          <LanguageSelection
            selectedLanguage={selectedLanguage}
            onLanguageChange={setSelectedLanguage}
            provider={transcriptProvider}
            model={transcriptModel}
          />
        </div>
        <div className="mt-5 flex justify-end">
          <Button onClick={saveTranscription} disabled={savingTranscript || !installedTranscriptModels.includes(transcriptModel)}>
            {savingTranscript ? t('settings:actions.saving') : t('settings:actions.save')}
          </Button>
        </div>
      </ServiceCard>

      <ServiceCard
        icon={Users}
        title={t('settings:services.speaker.title')}
        description={t('settings:services.speaker.description')}
      >
        <div className="flex items-center justify-between gap-4 rounded-md border p-4">
          <div>
            <div className="font-medium">Sherpa ONNX</div>
            <p className="mt-1 text-sm text-muted-foreground">sherpa-v1 · Pyannote + ERes2Net</p>
          </div>
          <Switch
            checked={speakerConfig.enabled}
            onCheckedChange={(enabled) => setSpeakerConfig((current) => ({ ...current, enabled }))}
          />
        </div>

        {speakerConfig.enabled && !speakerInstalled && (
          <div className="mt-4 flex items-center justify-between gap-3 rounded-md border border-amber-200 bg-amber-50 p-3 text-sm text-amber-800">
            <span>{t('settings:services.speaker.modelMissing')}</span>
            <Button variant="outline" size="sm" onClick={onOpenModels}>{t('settings:tabs.models')}</Button>
          </div>
        )}

        {!speakerConfig.enabled && (
          <div className="mt-4 flex gap-2 rounded-md bg-gray-50 p-3 text-sm text-gray-600">
            <AlertTriangle className="mt-0.5 h-4 w-4 shrink-0" />
            <span>{t('settings:services.speaker.disabledHint')}</span>
          </div>
        )}
        <div className="mt-5 flex justify-end">
          <Button onClick={saveSpeaker} disabled={savingSpeaker || (speakerConfig.enabled && !speakerInstalled)}>
            {savingSpeaker ? t('settings:actions.saving') : t('settings:actions.save')}
          </Button>
        </div>
      </ServiceCard>

      <ServiceCard
        icon={Bot}
        title={t('settings:services.summary.title')}
        description={t('settings:services.summary.description')}
      >
        {isRemoteSummary && (
          <div className="mb-4 flex gap-2 rounded-md border border-amber-200 bg-amber-50 p-3 text-sm text-amber-800">
            <AlertTriangle className="mt-0.5 h-4 w-4 shrink-0" />
            <span>{t('settings:services.summary.remoteNotice')}</span>
          </div>
        )}
        <SummaryModelSettings showAssetManagement={false} />
      </ServiceCard>
    </div>
  );
}
