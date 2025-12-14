import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';
import enUS from './resources/en-US.json';
import zhCN from './resources/zh-CN.json';
import zhTW from './resources/zh-TW.json';
import jaJP from './resources/ja-JP.json';
import koKR from './resources/ko-KR.json';
import frFR from './resources/fr-FR.json';
import esES from './resources/es-ES.json';
import ptBR from './resources/pt-BR.json';
import deDE from './resources/de-DE.json';
import itIT from './resources/it-IT.json';
import ruRU from './resources/ru-RU.json';
import ukUA from './resources/uk-UA.json';
import arSA from './resources/ar-SA.json';
import hiIN from './resources/hi-IN.json';
import trTR from './resources/tr-TR.json';

export type SupportedLanguage =
  | 'en-US'
  | 'zh-CN'
  | 'zh-TW'
  | 'ja-JP'
  | 'ko-KR'
  | 'fr-FR'
  | 'es-ES'
  | 'pt-BR'
  | 'de-DE'
  | 'it-IT'
  | 'ru-RU'
  | 'uk-UA'
  | 'ar-SA'
  | 'hi-IN'
  | 'tr-TR';

type LanguageOption = {
  code: SupportedLanguage;
  label: string;
};

export const LANGUAGE_OPTIONS: LanguageOption[] = [
  { code: 'en-US', label: 'English' },
  { code: 'zh-CN', label: '简体中文' },
  { code: 'zh-TW', label: '繁體中文' },
  { code: 'ja-JP', label: '日本語' },
  { code: 'ko-KR', label: '한국어' },
  { code: 'fr-FR', label: 'Français' },
  { code: 'es-ES', label: 'Español' },
  { code: 'pt-BR', label: 'Português (Brasil)' },
  { code: 'de-DE', label: 'Deutsch' },
  { code: 'it-IT', label: 'Italiano' },
  { code: 'ru-RU', label: 'Русский' },
  { code: 'uk-UA', label: 'Українська' },
  { code: 'ar-SA', label: 'العربية' },
  { code: 'hi-IN', label: 'हिन्दी' },
  { code: 'tr-TR', label: 'Türkçe' },
];

const STORAGE_KEY = 'cardinal.language';
const DEFAULT_LANGUAGE: SupportedLanguage = 'en-US';

const resources = {
  'en-US': { translation: enUS },
  'zh-CN': { translation: zhCN },
  'zh-TW': { translation: zhTW },
  'ja-JP': { translation: jaJP },
  'ko-KR': { translation: koKR },
  'fr-FR': { translation: frFR },
  'es-ES': { translation: esES },
  'pt-BR': { translation: ptBR },
  'de-DE': { translation: deDE },
  'it-IT': { translation: itIT },
  'ru-RU': { translation: ruRU },
  'uk-UA': { translation: ukUA },
  'ar-SA': { translation: arSA },
  'hi-IN': { translation: hiIN },
  'tr-TR': { translation: trTR },
} as const;

type LegacyLanguage =
  | 'en'
  | 'zh'
  | 'ja'
  | 'ko'
  | 'fr'
  | 'es'
  | 'pt'
  | 'de'
  | 'it'
  | 'ru'
  | 'uk'
  | 'ar'
  | 'hi'
  | 'tr';

const LEGACY_LANGUAGE_MAP: Record<LegacyLanguage, SupportedLanguage> = {
  en: 'en-US',
  zh: 'zh-CN',
  ja: 'ja-JP',
  ko: 'ko-KR',
  fr: 'fr-FR',
  es: 'es-ES',
  pt: 'pt-BR',
  de: 'de-DE',
  it: 'it-IT',
  ru: 'ru-RU',
  uk: 'uk-UA',
  ar: 'ar-SA',
  hi: 'hi-IN',
  tr: 'tr-TR',
};

const normalizeStoredLanguage = (stored: string): SupportedLanguage | undefined => {
  if (stored in resources) {
    return stored as SupportedLanguage;
  }
  if (stored in LEGACY_LANGUAGE_MAP) {
    return LEGACY_LANGUAGE_MAP[stored as LegacyLanguage];
  }
  return undefined;
};

const normalizeBrowserLanguage = (lng: string): SupportedLanguage => {
  if (lng in resources) {
    return lng as SupportedLanguage;
  }

  const [base, regionOrScript] = lng.split('-', 2);

  if (base === 'zh') {
    const tag = regionOrScript?.toUpperCase();
    if (tag === 'HANT' || tag === 'TW' || tag === 'HK' || tag === 'MO') {
      return 'zh-TW';
    }
    return 'zh-CN';
  }

  switch (base) {
    case 'en':
      return 'en-US';
    case 'ja':
      return 'ja-JP';
    case 'ko':
      return 'ko-KR';
    case 'fr':
      return 'fr-FR';
    case 'es':
      return 'es-ES';
    case 'pt':
      return 'pt-BR';
    case 'de':
      return 'de-DE';
    case 'it':
      return 'it-IT';
    case 'ru':
      return 'ru-RU';
    case 'uk':
      return 'uk-UA';
    case 'ar':
      return 'ar-SA';
    case 'hi':
      return 'hi-IN';
    case 'tr':
      return 'tr-TR';
    default:
      return DEFAULT_LANGUAGE;
  }
};

const detectInitialLanguage = (): SupportedLanguage => {
  if (typeof window === 'undefined') {
    return DEFAULT_LANGUAGE;
  }

  try {
    const stored = window.localStorage.getItem(STORAGE_KEY);
    if (stored) {
      const normalized = normalizeStoredLanguage(stored);
      if (normalized) {
        return normalized;
      }
    }
  } catch (error) {
    console.warn('Unable to read saved language preference', error);
  }

  const browserLang = window.navigator.language;
  if (browserLang) {
    return normalizeBrowserLanguage(browserLang);
  }

  return DEFAULT_LANGUAGE;
};

export const __test__ = {
  detectInitialLanguage,
  normalizeBrowserLanguage,
  normalizeStoredLanguage,
} as const;

void i18n.use(initReactI18next).init({
  resources,
  lng: detectInitialLanguage(),
  fallbackLng: DEFAULT_LANGUAGE,
  supportedLngs: Object.keys(resources),
  interpolation: {
    escapeValue: false,
  },
  returnNull: false,
});

if (typeof document !== 'undefined') {
  document.documentElement.lang = i18n.language;
}

i18n.on('languageChanged', (lng) => {
  if (typeof window !== 'undefined') {
    try {
      window.localStorage.setItem(STORAGE_KEY, lng);
    } catch (error) {
      console.warn('Unable to persist language preference', error);
    }
  }
  if (typeof document !== 'undefined') {
    document.documentElement.lang = lng;
  }
});

export { i18n as default, STORAGE_KEY as LANGUAGE_STORAGE_KEY };
