import type { AppLocale } from './resources';

export function resolveLocale(
  storedLocale: string | null | undefined,
  systemLanguage: string | null | undefined
): AppLocale {
  if (storedLocale === 'zh-CN' || storedLocale === 'en-US') return storedLocale;
  return systemLanguage?.toLowerCase().startsWith('zh') ? 'zh-CN' : 'en-US';
}
