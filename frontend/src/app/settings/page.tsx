'use client';

import React, { useMemo, useState, useLayoutEffect, useRef } from 'react';
import { ArrowLeft, Settings2, Mic, Database as DatabaseIcon, SlidersHorizontal, FlaskConical } from 'lucide-react';
import { useRouter } from 'next/navigation';
import { motion } from 'framer-motion';
import { useTranslation } from 'react-i18next';
import { RecordingSettings } from '@/components/RecordingSettings';
import { PreferenceSettings } from '@/components/PreferenceSettings';
import { ModelsSettings } from '@/components/ModelsSettings';
import { ServicesSettings } from '@/components/ServicesSettings';
import { BetaSettings } from '@/components/BetaSettings';
import { Tabs, TabsList, TabsTrigger, TabsContent } from '@/components/ui/tabs';

export default function SettingsPage() {
  const router = useRouter();
  const { t, i18n } = useTranslation('settings');
  const tabs = useMemo(() => [
    { value: 'general', label: t('tabs.general'), icon: Settings2 },
    { value: 'recording', label: t('tabs.recordings'), icon: Mic },
    { value: 'models', label: t('tabs.models'), icon: DatabaseIcon },
    { value: 'services', label: t('tabs.services'), icon: SlidersHorizontal },
    { value: 'beta', label: t('tabs.beta'), icon: FlaskConical },
  ] as const, [t, i18n.resolvedLanguage]);

  // Animation state for tabs
  const [activeTab, setActiveTab] = useState('general');
  const [mountedTabs, setMountedTabs] = useState<Set<string>>(() => new Set());
  const tabRefs = useRef<(HTMLButtonElement | null)[]>([]);
  const [underlineStyle, setUnderlineStyle] = useState({ left: 0, width: 0 });

  const activateTab = (value: string) => {
    if (value === 'models' || value === 'services') {
      setMountedTabs((current) => {
        if (current.has(value)) return current;
        const next = new Set(current);
        next.add(value);
        return next;
      });
    }
    setActiveTab(value);
  };

  // Update underline position when active tab changes
  useLayoutEffect(() => {
    const activeIndex = tabs.findIndex(tab => tab.value === activeTab);
    const activeTabElement = tabRefs.current[activeIndex];

    if (activeTabElement) {
      const { offsetLeft, offsetWidth } = activeTabElement;
      setUnderlineStyle({ left: offsetLeft, width: offsetWidth });
    }
  }, [activeTab, tabs]);

  return (
    <div className="h-screen bg-gray-50 flex flex-col">
      {/* Fixed Header */}
      <div className="sticky top-0 z-10 bg-gray-50 border-b border-gray-200">
        <div className="max-w-6xl mx-auto px-8 py-6">
          <div className="flex items-center gap-4">
            <button
              onClick={() => router.back()}
              className="flex items-center gap-2 text-gray-600 hover:text-gray-900 transition-colors"
            >
              <ArrowLeft className="w-5 h-5" />
              <span>{t('back')}</span>
            </button>
            <h1 className="text-3xl font-bold">{t('title')}</h1>
          </div>
        </div>
      </div>

      {/* Scrollable Content */}
      <div className="flex-1 overflow-y-auto">
        <div className="max-w-6xl mx-auto p-8 pt-6">
          {/* Tabs */}
          <Tabs value={activeTab} onValueChange={activateTab}>
            <TabsList className="sticky top-0 z-30 h-auto w-full justify-start overflow-x-auto overflow-y-hidden rounded-none border-b border-gray-200 bg-gray-50/95 p-0 shadow-[0_1px_0_rgba(15,23,42,0.04)] backdrop-blur supports-[backdrop-filter]:bg-gray-50/85">
              {tabs.map((tab, index) => {
                const Icon = tab.icon;
                return (
                  <TabsTrigger
                    key={tab.value}
                    value={tab.value}
                    ref={el => { tabRefs.current[index] = el }}
                    className="flex shrink-0 items-center gap-2 whitespace-nowrap px-4 py-4 bg-transparent rounded-none border-0 data-[state=active]:bg-transparent data-[state=active]:text-blue-600 data-[state=active]:shadow-none text-gray-600 hover:text-gray-900 relative z-10"
                  >
                    <Icon className="w-4 h-4" />
                    {tab.label}
                  </TabsTrigger>
                );
              })}

              <motion.div
                className="absolute bottom-0 z-20 h-0.5 bg-blue-600"
                layoutId="underline"
                style={{ left: underlineStyle.left, width: underlineStyle.width }}
                transition={{ type: 'spring', stiffness: 400, damping: 40 }}
              />
            </TabsList>

            <TabsContent value="general">
              <PreferenceSettings />
            </TabsContent>
            <TabsContent value="recording">
              <RecordingSettings />
            </TabsContent>
            {mountedTabs.has('models') && (
              <TabsContent value="models" forceMount>
                <ModelsSettings onOpenServices={() => activateTab('services')} />
              </TabsContent>
            )}
            {mountedTabs.has('services') && (
              <TabsContent value="services" forceMount>
                <ServicesSettings onOpenModels={() => activateTab('models')} />
              </TabsContent>
            )}
            <TabsContent value="beta" className="mt-6">
              <BetaSettings />
            </TabsContent>
          </Tabs>
        </div>
      </div>
    </div>
  );
};
