"use client";

import React, { useEffect, useState } from "react";
import { getVersion } from '@tauri-apps/api/app';
import Image from 'next/image';
import { useTranslation } from 'react-i18next';

const FORK_NOTICE = 'Mingtily is an independent community fork of Meetily. It is not affiliated with or endorsed by the Meetily project.';

export function About() {
  const { t } = useTranslation(['common', 'meeting']);
  const [currentVersion, setCurrentVersion] = useState('0.6.1');

  useEffect(() => {
    getVersion().then(setCurrentVersion).catch(console.error);
  }, []);

  return (
    <div className="p-6 space-y-6 max-h-[80vh] overflow-y-auto">
      <div className="text-center space-y-3">
        <Image
          src="/icon_128x128.png"
          alt={t('common:about', { defaultValue: 'Mingtily' }) + ' Mingtily'}
          width={72}
          height={72}
          className="mx-auto"
        />
        <div>
          <h1 className="text-xl font-semibold text-gray-900">Mingtily</h1>
          <p className="text-sm text-gray-500">{t('common:version', { version: currentVersion })}</p>
        </div>
        <p className="text-sm text-gray-700 leading-relaxed">
          {t('meeting:localFirstDescription')}
        </p>
      </div>

      <div className="rounded-lg border border-sky-100 bg-sky-50 p-4 space-y-2">
        <h2 className="text-sm font-semibold text-sky-950">{t('meeting:externalAiTitle')}</h2>
        <p className="text-sm text-sky-900 leading-relaxed">
          {t('meeting:externalAiDescription')}
        </p>
      </div>

      <div className="space-y-2 text-sm text-gray-600">
        <p>{FORK_NOTICE}</p>
        <p>
          {t('meeting:licenseNotice')}
        </p>
      </div>
    </div>
  );
}
