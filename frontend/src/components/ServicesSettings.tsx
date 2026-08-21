'use client';

import { useEffect, useState } from 'react';
import { AlertTriangle, Bot, BookOpen, Plus, Trash2 } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';
import { Button } from '@/components/ui/button';
import { Switch } from '@/components/ui/switch';
import { Textarea } from '@/components/ui/textarea';
import { Input } from '@/components/ui/input';
import { PipelineSettings } from '@/components/PipelineSettings';
import { SummaryModelSettings } from '@/components/SummaryModelSettings';
import { useConfig } from '@/contexts/ConfigContext';
import {
  DEFAULT_SHERPA_ASR_ENHANCEMENT_CONFIG,
  SherpaAsrAPI,
  SherpaAsrEnhancementConfig,
  HomophoneReplacerStatus,
  supportsDynamicHotwords,
  DEFAULT_TERMINOLOGY_CONFIG,
  TerminologyConfig,
} from '@/lib/sherpa-asr';

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
        <div className="rounded-md bg-sky-50 p-2 text-sky-700"><Icon className="h-5 w-5" /></div>
        <div>
          <h2 className="text-lg font-semibold text-gray-900">{title}</h2>
          <p className="mt-1 text-sm text-gray-600">{description}</p>
        </div>
      </div>
      {children}
    </section>
  );
}

function parseHotwords(value: string): string[] {
  return value
    .split(/[\n,，]+/)
    .map((term) => term.trim())
    .filter(Boolean);
}

export function ServicesSettings({ onOpenModels }: ServicesSettingsProps) {
  const { t } = useTranslation(['settings', 'models']);
  const { transcriptModelConfig, modelConfig } = useConfig();
  const [enhancementConfig, setEnhancementConfig] = useState<SherpaAsrEnhancementConfig>(
    DEFAULT_SHERPA_ASR_ENHANCEMENT_CONFIG
  );
  const [hotwordsText, setHotwordsText] = useState('');
  const [terminologyConfig, setTerminologyConfig] = useState<TerminologyConfig>(DEFAULT_TERMINOLOGY_CONFIG);
  const [savingTerminology, setSavingTerminology] = useState(false);
  const [homophoneStatus, setHomophoneStatus] = useState<HomophoneReplacerStatus | null>(null);

  useEffect(() => {
    let active = true;
    void Promise.allSettled([
      SherpaAsrAPI.getTerminologyConfig(),
      SherpaAsrAPI.getHomophoneStatus(),
    ]).then(([terminologyResult, homophoneResult]) => {
      if (!active) return;
      if (terminologyResult.status === 'fulfilled') {
        setTerminologyConfig(terminologyResult.value);
        setEnhancementConfig({
          hotwords: terminologyResult.value.terms,
          homophoneReplacerEnabled: terminologyResult.value.homophoneReplacerEnabled,
          homophoneRuleFsts: terminologyResult.value.homophoneRuleFsts,
        });
        setHotwordsText(terminologyResult.value.terms.join('\n'));
      }
      if (homophoneResult.status === 'fulfilled') {
        setHomophoneStatus(homophoneResult.value);
      }
    });
    return () => {
      active = false;
    };
  }, []);

  const dynamicHotwordsSupported = supportsDynamicHotwords(
    transcriptModelConfig.provider,
    transcriptModelConfig.model
  );
  const homophoneResourcesReady =
    homophoneStatus?.status === 'available' &&
    (homophoneStatus?.rules.length ?? 0) > 0;
  const availableHomophoneRuleIds = new Set(
    homophoneStatus?.rules.map((rule) => rule.id) ?? []
  );
  const homophoneSelectionReady =
    !enhancementConfig.homophoneReplacerEnabled ||
    (homophoneResourcesReady &&
      enhancementConfig.homophoneRuleFsts.length === 1 &&
      enhancementConfig.homophoneRuleFsts.every((ruleId) =>
        availableHomophoneRuleIds.has(ruleId)
      ));
  const parsedTerminologyTerms = [...new Set(parseHotwords(hotwordsText))];
  const terminologyTermsError = parsedTerminologyTerms.length > 200
    ? t('settings:services.transcription.terminology.tooManyTerms')
    : parsedTerminologyTerms.some((term) => [...term].length > 100)
      ? t('settings:services.transcription.terminology.termTooLong')
      : parsedTerminologyTerms.reduce((sum, term) => sum + [...term].length, 0) > 4000
        ? t('settings:services.transcription.terminology.termsTooLong')
        : null;

  const saveTerminology = async () => {
    setSavingTerminology(true);
    try {
      const saved = await SherpaAsrAPI.saveTerminologyConfig({
        ...terminologyConfig,
        terms: parseHotwords(hotwordsText),
        homophoneReplacerEnabled: enhancementConfig.homophoneReplacerEnabled,
        homophoneRuleFsts: enhancementConfig.homophoneRuleFsts,
      });
      setTerminologyConfig(saved);
      setEnhancementConfig({
        hotwords: saved.terms,
        homophoneReplacerEnabled: saved.homophoneReplacerEnabled,
        homophoneRuleFsts: saved.homophoneRuleFsts,
      });
      setHotwordsText(saved.terms.join('\n'));
      toast.success(t('settings:services.transcription.terminology.saved'));
    } catch (error) {
      toast.error(t('settings:services.transcription.terminology.saveFailed'), { description: String(error) });
    } finally {
      setSavingTerminology(false);
    }
  };

  const isRemoteSummary = ['claude', 'groq', 'openai', 'openrouter', 'custom-openai'].includes(
    modelConfig.provider
  );

  return (
    <div className="space-y-6">
      <PipelineSettings onOpenModels={onOpenModels} />
      <ServiceCard
        icon={BookOpen}
        title={t('settings:services.transcription.terminology.customTitle')}
        description={t('settings:services.transcription.terminology.customDescription')}
      >
        <div className="space-y-2">
          <label className="text-sm font-medium">{t('settings:services.transcription.terminology.terms')}</label>
          <Textarea
            value={hotwordsText}
            onChange={(event) => setHotwordsText(event.target.value)}
            rows={5}
            placeholder={t('settings:services.transcription.terminology.hotwordsPlaceholder')}
          />
          <div className="flex justify-between text-xs text-gray-500">
            <span>{t('settings:services.transcription.terminology.termCount', {
              count: parsedTerminologyTerms.length,
              characters: parsedTerminologyTerms.reduce((sum, term) => sum + [...term].length, 0),
            })}</span>
            <span>{t('settings:services.transcription.terminology.nextSession')}</span>
          </div>
          {terminologyTermsError && <p className="text-xs text-red-600">{terminologyTermsError}</p>}
          <p className="text-xs leading-5 text-sky-700">
            {transcriptModelConfig.provider === 'localWhisper'
              ? t('settings:services.transcription.terminology.whisperBehavior')
              : dynamicHotwordsSupported
                ? t('settings:services.transcription.terminology.promptBehavior')
                : t('settings:services.transcription.terminology.correctionOnlyBehavior')}
          </p>
        </div>

        <div className="mt-5 border-t pt-4">
          <div className="flex items-center justify-between">
            <div>
              <h3 className="text-sm font-medium text-gray-900">{t('settings:services.transcription.terminology.replacements')}</h3>
              <p className="mt-1 text-xs text-gray-500">{t('settings:services.transcription.terminology.replacementsHint')}</p>
            </div>
            <Button
              size="sm"
              variant="outline"
              onClick={() => setTerminologyConfig((current) => ({
                ...current,
                replacements: [...current.replacements, { source: '', target: '' }],
              }))}
            >
              <Plus size={16} /> {t('settings:services.transcription.terminology.addReplacement')}
            </Button>
          </div>
          <div className="mt-3 space-y-2">
            {terminologyConfig.replacements.map((replacement, index) => (
              <div key={index} className="grid grid-cols-[1fr_auto_1fr_auto] items-center gap-2">
                <Input
                  aria-invalid={!replacement.source.trim()}
                  value={replacement.source}
                  onChange={(event) => setTerminologyConfig((current) => ({
                    ...current,
                    replacements: current.replacements.map((item, itemIndex) => itemIndex === index
                      ? { ...item, source: event.target.value }
                      : item),
                  }))}
                  placeholder={t('settings:services.transcription.terminology.sourcePlaceholder')}
                />
                <span className="text-gray-400">→</span>
                <Input
                  value={replacement.target}
                  onChange={(event) => setTerminologyConfig((current) => ({
                    ...current,
                    replacements: current.replacements.map((item, itemIndex) => itemIndex === index
                      ? { ...item, target: event.target.value }
                      : item),
                  }))}
                  placeholder={t('settings:services.transcription.terminology.targetPlaceholder')}
                />
                <Button
                  size="icon"
                  variant="ghost"
                  aria-label={t('settings:services.transcription.terminology.removeReplacement')}
                  onClick={() => setTerminologyConfig((current) => ({
                    ...current,
                    replacements: current.replacements.filter((_, itemIndex) => itemIndex !== index),
                  }))}
                ><Trash2 size={16} /></Button>
              </div>
            ))}
          </div>
        </div>

        <details className="mt-5 rounded-md border border-gray-200 p-3">
          <summary className="cursor-pointer text-sm font-medium text-gray-800">
            {t('settings:services.transcription.terminology.advancedCompatibility')}
          </summary>
          <p className="mt-2 text-xs leading-5 text-gray-500">
            {t('settings:services.transcription.terminology.advancedCompatibilityHint')}
          </p>
          <div className="mt-3 flex items-center justify-between gap-4">
            <span className="text-sm">{t('settings:services.transcription.terminology.homophone')}</span>
            <Switch
              checked={enhancementConfig.homophoneReplacerEnabled}
              disabled={!enhancementConfig.homophoneReplacerEnabled && !homophoneResourcesReady}
              onCheckedChange={(homophoneReplacerEnabled) => setEnhancementConfig((current) => ({
                ...current,
                homophoneReplacerEnabled,
              }))}
            />
          </div>
          {(homophoneStatus?.rules.length ?? 0) > 0 && enhancementConfig.homophoneReplacerEnabled && (
            <div className="mt-3 space-y-2">
              {homophoneStatus?.rules.map((rule) => (
                <label key={rule.id} className="flex items-center gap-2 text-sm">
                  <input
                    type="radio"
                    name="terminology-homophone-rule"
                    checked={enhancementConfig.homophoneRuleFsts.includes(rule.id)}
                    onChange={() => setEnhancementConfig((current) => ({ ...current, homophoneRuleFsts: [rule.id] }))}
                  />
                  {rule.name}
                </label>
              ))}
            </div>
          )}
          {!homophoneResourcesReady && (
            <Button className="mt-3" variant="outline" size="sm" onClick={onOpenModels}>
              {t('settings:services.transcription.terminology.manageAdvancedResources')}
            </Button>
          )}
        </details>

        <div className="mt-5 flex justify-end">
          <Button
            onClick={saveTerminology}
            disabled={
              savingTerminology ||
              !!terminologyTermsError ||
              terminologyConfig.replacements.some((replacement) => !replacement.source.trim()) ||
              !homophoneSelectionReady
            }
          >
            {savingTerminology ? t('settings:actions.saving') : t('settings:actions.save')}
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
