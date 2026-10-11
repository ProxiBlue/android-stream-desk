import { createI18n } from 'vue-i18n';

export const SUPPORTED_LOCALES = ['en', 'vi'] as const;
export type SupportedLocale = (typeof SUPPORTED_LOCALES)[number];

const STORAGE_KEY = 'locale';

function detectInitialLocale(): SupportedLocale {
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved && (SUPPORTED_LOCALES as readonly string[]).includes(saved)) {
      return saved as SupportedLocale;
    }
  } catch (_) {
    // localStorage unavailable (e.g. private mode) — fall through to default.
  }
  return 'en';
}

// No global `messages` here on purpose: every component owns its own strings
// via `useI18n({ useScope: 'local', messages: { en: {...}, vi: {...} } })`,
// right next to the template that uses them. This global instance only
// carries the active locale — every local scope inherits it automatically.
export const i18n = createI18n({
  legacy: false,
  locale: detectInitialLocale(),
  fallbackLocale: 'en',
  messages: {},
});

export function setLocale(locale: SupportedLocale): void {
  i18n.global.locale.value = locale;
  try {
    localStorage.setItem(STORAGE_KEY, locale);
  } catch (_) {
    // ignore — worst case the choice doesn't persist across reloads.
  }
}

export function currentLocale(): SupportedLocale {
  return i18n.global.locale.value as SupportedLocale;
}
