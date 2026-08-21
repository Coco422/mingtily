'use client';

import { useEffect, useMemo, useState } from 'react';
import { AlertTriangle, Check, Download, Gauge, Loader2, Scale, SlidersHorizontal, Sparkles } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Switch } from '@/components/ui/switch';
import { useConfig } from '@/contexts/ConfigContext';
import { useTranscriptionModels, type ModelOption } from '@/hooks/useTranscriptionModels';
import { pipelineService } from '@/services/pipelineService';
import {
  applyRecommendedPipeline,
  downloadPipelineAsset,
  loadPipelineAssetRequirements,
  type PipelineAssetRequirement,
} from '@/lib/pipeline-recommendations';
import type { LiveMode, PipelineConfig, PipelinePreset, PostMeetingPolicy, ResolvedPipeline, ResourceMode, SpeakerRefinementPolicy } from '@/types/pipeline';

interface PipelineSettingsProps { onOpenModels: () => void }
const PRESETS: Array<{ id: PipelinePreset; icon: typeof Gauge }> = [
  { id: 'fast', icon: Gauge }, { id: 'balanced', icon: Scale },
  { id: 'quality', icon: Sparkles }, { id: 'custom', icon: SlidersHorizontal },
];
const providerForPipeline = (provider: ModelOption['provider']) => provider === 'whisper' ? 'localWhisper' : provider;
const modelKey = (provider: string, model: string) => `${provider === 'localWhisper' ? 'whisper' : provider}:${model}`;
function parseModelKey(key: string) {
  const separator = key.indexOf(':');
  return { provider: providerForPipeline(key.slice(0, separator) as ModelOption['provider']), model: key.slice(separator + 1) };
}

function InstalledModelSelect({ value, models, loading, onChange }: { value: string; models: ModelOption[]; loading: boolean; onChange: (value: string) => void }) {
  const { t } = useTranslation('settings');
  return <Select value={value} onValueChange={onChange} disabled={loading || models.length === 0}>
    <SelectTrigger><SelectValue placeholder={t('services.selectInstalledModel')} /></SelectTrigger>
    <SelectContent>{models.map((model) => <SelectItem key={`${model.provider}:${model.name}`} value={`${model.provider}:${model.name}`}>{model.displayName}</SelectItem>)}</SelectContent>
  </Select>;
}

export function PipelineSettings({ onOpenModels }: PipelineSettingsProps) {
  const { t } = useTranslation('settings');
  const { betaFeatures, toggleBetaFeature, setTranscriptModelConfig, setSelectedLanguage } = useConfig();
  const [config, setConfig] = useState<PipelineConfig | null>(null);
  const [resolved, setResolved] = useState<ResolvedPipeline | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loadingConfig, setLoadingConfig] = useState(true);
  const [saving, setSaving] = useState(false);
  const [recommendedAssets, setRecommendedAssets] = useState<PipelineAssetRequirement[]>([]);
  const [loadingAssets, setLoadingAssets] = useState(false);
  const [downloadingAsset, setDownloadingAsset] = useState<string | null>(null);
  const { availableModels, loadingModels, fetchModels } = useTranscriptionModels(undefined, betaFeatures.experimentalAsrModels, true);

  useEffect(() => { void fetchModels(); }, [fetchModels]);
  useEffect(() => {
    let active = true;
    void pipelineService.getConfig().then((value) => {
      if (active) setConfig(applyRecommendedPipeline(value, value.preset));
    })
      .catch((reason) => { if (active) setError(String(reason)); })
      .finally(() => { if (active) setLoadingConfig(false); });
    return () => { active = false; };
  }, []);
  useEffect(() => {
    if (!config) return;
    const timer = window.setTimeout(() => void pipelineService.resolve(config)
      .then((value) => { setResolved(value); setError(null); })
      .catch((reason) => { setResolved(null); setError(String(reason)); }), 150);
    return () => window.clearTimeout(timer);
  }, [config]);
  useEffect(() => {
    if (!config || config.preset === 'custom') {
      setRecommendedAssets([]);
      return;
    }
    let active = true;
    setLoadingAssets(true);
    void loadPipelineAssetRequirements(config.preset)
      .then((assets) => { if (active) setRecommendedAssets(assets); })
      .catch((reason) => { if (active) setError(String(reason)); })
      .finally(() => { if (active) setLoadingAssets(false); });
    return () => { active = false; };
  }, [config?.preset]);

  const finalizedModels = useMemo(() => availableModels.filter((model) => model.inputMode !== 'continuous'), [availableModels]);
  const streamingModels = useMemo(() => availableModels.filter((model) => model.inputMode === 'continuous'), [availableModels]);
  if (loadingConfig) return <section className="rounded-lg border border-sky-200 bg-sky-50/50 p-6 text-sm text-slate-600">{t('pipeline.loading')}</section>;
  if (!config) return <section className="rounded-lg border border-red-200 bg-red-50 p-6 text-sm text-red-700"><div className="flex items-center gap-2 font-medium"><AlertTriangle className="h-4 w-4" />{t('pipeline.loadFailed')}</div>{error && <p className="mt-2 break-words">{error}</p>}</section>;

  const setFinalizedModel = (key: string) => setConfig({ ...config, finalized: { ...config.finalized, ...parseModelKey(key) } });
  const setStreamingModel = (key: string) => {
    if (key === 'disabled') return setConfig({ ...config, live: { mode: 'vad-segmented', streamingProvider: null, streamingModel: null } });
    const selected = parseModelKey(key);
    setConfig({ ...config, live: { mode: 'continuous-preview', streamingProvider: selected.provider, streamingModel: selected.model } });
  };
  const setPostModel = (key: string) => setConfig({ ...config, postMeetingAsr: { ...config.postMeetingAsr, ...parseModelKey(key) } });
  const selectPreset = async (preset: PipelinePreset) => {
    if (preset === 'quality' && !betaFeatures.experimentalAsrModels) {
      try {
        await toggleBetaFeature('experimentalAsrModels', true);
      } catch (reason) {
        toast.error(t('pipeline.experimentalEnableFailed'), { description: String(reason) });
        return;
      }
    }
    setConfig(applyRecommendedPipeline(config, preset));
  };
  const downloadRecommended = async () => {
    try {
      for (const asset of recommendedAssets.filter((item) => !item.installed)) {
        setDownloadingAsset(asset.key);
        await downloadPipelineAsset(asset);
      }
      if (config.preset !== 'custom') {
        setRecommendedAssets(await loadPipelineAssetRequirements(config.preset));
      }
      await fetchModels();
      toast.success(t('pipeline.recommendedReady'));
    } catch (reason) {
      toast.error(t('pipeline.recommendedDownloadFailed'), { description: String(reason) });
    } finally {
      setDownloadingAsset(null);
    }
  };
  const save = async () => {
    setSaving(true);
    try {
      const value = await pipelineService.save(config);
      setConfig(value.config); setResolved(value);
      setTranscriptModelConfig({ provider: value.config.finalized.provider as 'localWhisper' | 'parakeet' | 'sherpa-onnx', model: value.config.finalized.model, apiKey: null });
      setSelectedLanguage(value.config.finalized.language);
      toast.success(t('pipeline.saved'));
    } catch (reason) { toast.error(t('pipeline.saveFailed'), { description: String(reason) }); }
    finally { setSaving(false); }
  };
  const showQualityPaths = config.preset === 'quality' || config.preset === 'custom';
  const custom = config.preset === 'custom';
  const effective = resolved?.effectiveConfig ?? resolved?.config;
  const languageOptions = resolved?.finalizedCapabilities.languages.length
    ? ['auto', ...resolved.finalizedCapabilities.languages]
    : ['auto', 'zh', 'yue', 'en', 'ja', 'ko', 'de', 'fr', 'es', 'pt', 'ru'];

  return <section className="space-y-5 rounded-lg border border-sky-200 bg-white p-6 shadow-sm">
    <div><h2 className="text-lg font-semibold text-slate-900">{t('pipeline.title')}</h2><p className="mt-1 text-sm text-slate-600">{t('pipeline.description')}</p></div>
    <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">{PRESETS.map(({ id, icon: Icon }) => {
      const selected = config.preset === id;
      return <button key={id} type="button" aria-pressed={selected} onClick={() => void selectPreset(id)} className={`rounded-lg border p-4 text-left transition ${selected ? 'border-sky-500 bg-sky-50 ring-1 ring-sky-500' : 'border-slate-200 hover:border-sky-300'}`}>
        <div className="flex items-center justify-between"><Icon className={`h-5 w-5 ${selected ? 'text-sky-700' : 'text-slate-500'}`} />{selected && <Check className="h-4 w-4 text-sky-700" />}</div>
        <div className="mt-3 font-semibold text-slate-900">{t(`pipeline.presets.${id}`)}</div><p className="mt-1 text-xs leading-5 text-slate-600">{t(`pipeline.presetDescriptions.${id}`)}</p>
      </button>;
    })}</div>
    {!custom && <div className="space-y-3 rounded-lg border border-sky-200 bg-sky-50/50 p-4">
      <div><div className="font-medium text-slate-900">{t('pipeline.recommendedSetup')}</div><p className="mt-1 text-xs leading-5 text-slate-600">{t(`pipeline.recommendationDescriptions.${config.preset}`)}</p></div>
      <div className="grid gap-2 md:grid-cols-2">
        {recommendedAssets.map((asset) => <div key={asset.key} className="flex items-center justify-between rounded-md border border-slate-200 bg-white px-3 py-2 text-sm">
          <div><div className="font-medium text-slate-800">{t(`pipeline.assetKinds.${asset.kind}`)} · {asset.name}</div><div className="text-xs text-slate-500">{asset.downloadSizeMiB} MiB</div></div>
          <span className={asset.installed ? 'text-emerald-700' : asset.corrupt ? 'text-red-600' : 'text-amber-700'}>{asset.installed ? t('pipeline.assetReady') : asset.corrupt ? t('pipeline.assetRepair') : t('pipeline.assetMissing')}</span>
        </div>)}
      </div>
      {!loadingAssets && recommendedAssets.some((asset) => !asset.installed) && <div className="flex justify-end"><Button type="button" onClick={() => void downloadRecommended()} disabled={downloadingAsset !== null}>{downloadingAsset ? <><Loader2 className="mr-2 h-4 w-4 animate-spin" />{t('pipeline.downloadingRecommended')}</> : <><Download className="mr-2 h-4 w-4" />{t('pipeline.downloadRecommended')}</>}</Button></div>}
      {!loadingAssets && recommendedAssets.length > 0 && recommendedAssets.every((asset) => asset.installed) && <div className="flex items-center gap-2 text-sm font-medium text-emerald-700"><Check className="h-4 w-4" />{t('pipeline.recommendedReady')}</div>}
    </div>}
    {custom && <div className="grid gap-4 border-t border-slate-200 pt-5 md:grid-cols-2">
      <div className="space-y-2"><label className="text-sm font-medium">{t('pipeline.finalizedModel')}</label><InstalledModelSelect value={modelKey(config.finalized.provider, config.finalized.model)} models={finalizedModels} loading={loadingModels} onChange={setFinalizedModel} /></div>
      <div className="space-y-2"><label className="text-sm font-medium">{t('pipeline.language')}</label><Select value={config.finalized.language} onValueChange={(language) => setConfig({ ...config, finalized: { ...config.finalized, language } })}><SelectTrigger><SelectValue /></SelectTrigger><SelectContent>{languageOptions.map((language) => <SelectItem key={language} value={language}>{language === 'auto' ? t('services.transcription.autoDetect') : language}</SelectItem>)}</SelectContent></Select></div>
      {showQualityPaths && <div className="space-y-2"><label className="text-sm font-medium">{t('pipeline.streamingModel')}</label><Select value={config.live.mode === 'continuous-preview' && config.live.streamingModel ? modelKey(config.live.streamingProvider ?? 'sherpa-onnx', config.live.streamingModel) : 'disabled'} onValueChange={setStreamingModel}><SelectTrigger><SelectValue /></SelectTrigger><SelectContent><SelectItem value="disabled">{t('pipeline.noContinuousPreview')}</SelectItem>{streamingModels.map((model) => <SelectItem key={`${model.provider}:${model.name}`} value={`${model.provider}:${model.name}`}>{model.displayName}</SelectItem>)}</SelectContent></Select></div>}
    </div>}
    {custom && finalizedModels.length === 0 && !loadingModels && <div className="flex items-center justify-between gap-3 rounded-md border border-amber-200 bg-amber-50 p-3 text-sm text-amber-800"><span>{t('services.noInstalledModel')}</span><Button variant="outline" size="sm" onClick={onOpenModels}>{t('tabs.models')}</Button></div>}
    {custom && <div className="grid gap-4 rounded-lg border border-slate-200 bg-slate-50/60 p-4 md:grid-cols-2">
      <div className="space-y-2"><label className="text-sm font-medium">{t('pipeline.liveMode')}</label><Select value={config.live.mode} onValueChange={(mode) => setConfig({ ...config, live: { ...config.live, mode: mode as LiveMode } })}><SelectTrigger><SelectValue /></SelectTrigger><SelectContent>{(['off', 'vad-segmented', 'continuous-preview'] as LiveMode[]).map((mode) => <SelectItem key={mode} value={mode}>{t(`pipeline.liveModes.${mode}`)}</SelectItem>)}</SelectContent></Select></div>
      <div className="space-y-2"><label className="text-sm font-medium">{t('pipeline.postMeetingAsr')}</label><Select value={config.postMeetingAsr.policy} onValueChange={(policy) => setConfig({ ...config, postMeetingAsr: { ...config.postMeetingAsr, policy: policy as PostMeetingPolicy } })}><SelectTrigger><SelectValue /></SelectTrigger><SelectContent>{(['off', 'manual', 'auto'] as PostMeetingPolicy[]).map((policy) => <SelectItem key={policy} value={policy}>{t(`pipeline.policies.${policy}`)}</SelectItem>)}</SelectContent></Select></div>
      {config.postMeetingAsr.policy !== 'off' && <div className="space-y-2"><label className="text-sm font-medium">{t('pipeline.postModel')}</label><InstalledModelSelect value={modelKey(config.postMeetingAsr.provider ?? config.finalized.provider, config.postMeetingAsr.model ?? config.finalized.model)} models={finalizedModels} loading={loadingModels} onChange={setPostModel} /></div>}
      <div className="space-y-2"><label className="text-sm font-medium">{t('pipeline.speakerRefinement')}</label><Select value={config.speaker.refinement} onValueChange={(refinement) => setConfig({ ...config, speaker: { ...config.speaker, refinement: refinement as SpeakerRefinementPolicy } })}><SelectTrigger><SelectValue /></SelectTrigger><SelectContent>{(['off', 'manual', 'background-auto'] as SpeakerRefinementPolicy[]).map((policy) => <SelectItem key={policy} value={policy}>{t(`pipeline.speakerPolicies.${policy}`)}</SelectItem>)}</SelectContent></Select></div>
      <div className="space-y-2"><label className="text-sm font-medium">{t('pipeline.resourceMode')}</label><Select value={config.resources.mode} onValueChange={(mode) => setConfig({ ...config, resources: { ...config.resources, mode: mode as ResourceMode } })}><SelectTrigger><SelectValue /></SelectTrigger><SelectContent>{(['eco', 'balanced', 'fast'] as ResourceMode[]).map((mode) => <SelectItem key={mode} value={mode}>{t(`pipeline.resources.${mode}`)}</SelectItem>)}</SelectContent></Select><p className="text-xs leading-5 text-slate-500">{t(`pipeline.resourceDescriptions.${config.resources.mode}`)}</p></div>
      <div className="space-y-2"><label className="text-sm font-medium">{t('pipeline.memoryLimit')}</label><Input type="number" min={512} value={config.resources.memoryLimitMiB ?? ''} onChange={(event) => setConfig({ ...config, resources: { ...config.resources, memoryLimitMiB: event.target.value ? Number(event.target.value) : null } })} /></div>
      <label className="flex items-center justify-between gap-3 text-sm"><span>{t('pipeline.liveSpeakers')}</span><Switch checked={config.speaker.liveEnabled} onCheckedChange={(checked) => setConfig({ ...config, speaker: { ...config.speaker, liveEnabled: checked } })} /></label>
      <label className="flex items-center justify-between gap-3 text-sm"><span>{t('pipeline.punctuation')}</span><Switch checked={config.enhancements.punctuation === 'auto'} onCheckedChange={(checked) => setConfig({ ...config, enhancements: { ...config.enhancements, punctuation: checked ? 'auto' : 'off' } })} /></label>
      <label className="flex items-center justify-between gap-3 text-sm"><span>{t('pipeline.terminology')}</span><Switch checked={config.enhancements.terminology === 'auto'} onCheckedChange={(checked) => setConfig({ ...config, enhancements: { ...config.enhancements, terminology: checked ? 'auto' : 'off' } })} /></label>
      <label className="flex items-center justify-between gap-3 text-sm"><span>{t('pipeline.runOnBattery')}</span><Switch checked={config.resources.runAutomaticJobsOnBattery} onCheckedChange={(checked) => setConfig({ ...config, resources: { ...config.resources, runAutomaticJobsOnBattery: checked } })} /></label>
    </div>}
    {resolved && effective && <div className="space-y-1 rounded-lg border border-slate-200 bg-slate-50 p-4 text-sm text-slate-700"><div className="font-medium text-slate-900">{t('pipeline.actualPipeline')}</div><div>{t('pipeline.resolvedLive', { mode: effective.live.mode })}</div><div>{t('pipeline.resolvedFinalized', { provider: resolved.finalizedCapabilities.provider, model: resolved.finalizedCapabilities.model })}</div>{resolved.postMeetingCapabilities && <div>{t('pipeline.resolvedPost', { provider: resolved.postMeetingCapabilities.provider, model: resolved.postMeetingCapabilities.model })}</div>}<div>{t('pipeline.estimate', { memory: resolved.estimatedMemoryMiB, workers: resolved.workerCount, threads: resolved.threadCount })}</div><div>{resolved.speakerRefinementEnabled ? t('pipeline.speakerWillRun') : t('pipeline.speakerWillNotRun')}</div>{resolved.decisions.map((decision) => <div key={decision} className="text-xs text-amber-700">{t(`pipelineDecisions.${decision}`)}</div>)}</div>}
    {!custom && recommendedAssets.some((asset) => !asset.installed) && <p className="text-sm text-amber-700">{t('pipeline.downloadToActivate')}</p>}
    {error && (custom || (!loadingAssets && (recommendedAssets.length === 0 || recommendedAssets.every((asset) => asset.installed)))) && <p className="break-words text-sm text-red-600">{error}</p>}
    <div className="flex justify-end"><Button onClick={() => void save()} disabled={saving || !resolved}>{saving ? t('actions.saving') : t('actions.save')}</Button></div>
  </section>;
}
