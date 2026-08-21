import React, { useEffect, useState } from 'react';
import { Check, Gauge, Scale, Sparkles } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { OnboardingContainer } from '../OnboardingContainer';
import { useOnboarding } from '@/contexts/OnboardingContext';
import { useTranslation } from 'react-i18next';
import type { RecommendedPipelinePreset } from '@/lib/pipeline-recommendations';

export function SetupOverviewStep() {
  const { t } = useTranslation('onboarding');
  const { goNext, selectedPipelinePreset, setSelectedPipelinePreset } = useOnboarding();
  const [isMac, setIsMac] = useState(false);

  useEffect(() => {
    const checkPlatform = async () => {
      try {
        const { platform } = await import('@tauri-apps/plugin-os');
        setIsMac(platform() === 'macos');
      } catch (e) {
        setIsMac(navigator.userAgent.includes('Mac'));
      }
    };
    checkPlatform();
  }, []);

  const presets: Array<{ id: RecommendedPipelinePreset; icon: typeof Gauge }> = [
    { id: 'fast', icon: Gauge },
    { id: 'balanced', icon: Scale },
    { id: 'quality', icon: Sparkles },
  ];

  const handleContinue = () => {
    goNext();
  };

  return (
    <OnboardingContainer
      title={t('setupTitle')}
      description={t('setupDescription')}
      step={2}
      totalSteps={isMac ? 4 : 3}
    >
      <div className="flex flex-col items-center space-y-8">
        <div className="grid w-full max-w-3xl gap-3 md:grid-cols-3">
          {presets.map(({ id, icon: Icon }) => {
            const selected = selectedPipelinePreset === id;
            return <button
              key={id}
              type="button"
              aria-pressed={selected}
              onClick={() => setSelectedPipelinePreset(id)}
              className={`rounded-xl border p-5 text-left transition ${selected ? 'border-sky-500 bg-sky-50 ring-1 ring-sky-500' : 'border-gray-200 bg-white hover:border-sky-300'}`}
            >
              <div className="flex items-center justify-between"><Icon className="h-5 w-5 text-sky-700" />{selected && <Check className="h-4 w-4 text-sky-700" />}</div>
              <h3 className="mt-3 font-semibold text-gray-900">{t(`pipelinePresets.${id}.name`)}</h3>
              <p className="mt-2 text-xs leading-5 text-gray-600">{t(`pipelinePresets.${id}.description`)}</p>
              <p className="mt-3 text-xs font-medium text-sky-800">{t(`pipelinePresets.${id}.models`)}</p>
            </button>;
          })}
        </div>
        <p className="max-w-xl text-center text-xs leading-5 text-gray-500">{t('pipelineCustomLater')}</p>


        {/* CTA Section */}
        <div className="w-full max-w-xs space-y-4">
          <Button
            onClick={handleContinue}
            className="w-full h-11 bg-gray-900 hover:bg-gray-800 text-white"
          >
            {t('letsGo')}
          </Button>
          <div className="text-center">
            <a
              href="https://github.com/Coco422/mingtily"
              target="_blank"
              rel="noopener noreferrer"
              className="text-xs text-gray-600 hover:underline"
            >
              {t('reportIssue')}
            </a>
          </div>
        </div>
      </div>
    </OnboardingContainer>
  );
}
