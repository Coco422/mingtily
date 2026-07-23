'use client';

import { useEffect, type ReactNode } from 'react';
import { I18nextProvider } from 'react-i18next';
import i18n, { resolveInitialLocale, setUiLocale } from '.';

export function I18nProvider({ children }: { children: ReactNode }) {
  useEffect(() => {
    void setUiLocale(resolveInitialLocale());
  }, []);

  return <I18nextProvider i18n={i18n}>{children}</I18nextProvider>;
}
