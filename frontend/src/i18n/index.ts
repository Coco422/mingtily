'use client';

import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { resources, type AppLocale } from './resources';
import { resolveLocale } from './locale';

export const UI_LOCALE_STORAGE_KEY = 'mingtily.uiLocale';
export const FALLBACK_LOCALE: AppLocale = 'en-US';

export function resolveInitialLocale(): AppLocale {
  if (typeof window === 'undefined') return FALLBACK_LOCALE;
  const stored = window.localStorage.getItem(UI_LOCALE_STORAGE_KEY);
  return resolveLocale(stored, navigator.language);
}

if (!i18n.isInitialized) {
  i18n.use(initReactI18next).init({
    resources,
    lng: FALLBACK_LOCALE,
    fallbackLng: FALLBACK_LOCALE,
    defaultNS: 'common',
    interpolation: { escapeValue: false },
    react: { useSuspense: false },
  });
}

export async function setUiLocale(locale: AppLocale) {
  window.localStorage.setItem(UI_LOCALE_STORAGE_KEY, locale);
  await i18n.changeLanguage(locale);
  try {
    await invoke('set_ui_locale', { locale });
  } catch (error) {
    console.warn('[i18n] Failed to synchronize the native UI locale:', error);
  }
}

export default i18n;
