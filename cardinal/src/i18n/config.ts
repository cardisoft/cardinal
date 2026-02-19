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

const LANGUAGE_DEFINITIONS = [
  { code: 'en-US', label: 'English', translation: enUS },
  { code: 'zh-CN', label: '简体中文', translation: zhCN },
  { code: 'zh-TW', label: '繁體中文', translation: zhTW },
  { code: 'ja-JP', label: '日本語', translation: jaJP },
  { code: 'ko-KR', label: '한국어', translation: koKR },
  { code: 'fr-FR', label: 'Français', translation: frFR },
  { code: 'es-ES', label: 'Español', translation: esES },
  { code: 'pt-BR', label: 'Português (Brasil)', translation: ptBR },
  { code: 'de-DE', label: 'Deutsch', translation: deDE },
  { code: 'it-IT', label: 'Italiano', translation: itIT },
  { code: 'ru-RU', label: 'Русский', translation: ruRU },
  { code: 'uk-UA', label: 'Українська', translation: ukUA },
  { code: 'ar-SA', label: 'العربية', translation: arSA },
  { code: 'hi-IN', label: 'हिन्दी', translation: hiIN },
  { code: 'tr-TR', label: 'Türkçe', translation: trTR },
] as const;

type LanguageDefinition = (typeof LANGUAGE_DEFINITIONS)[number];

export type SupportedLanguage = LanguageDefinition['code'];

type LanguageOption = {
  code: SupportedLanguage;
  label: string;
};

export const LANGUAGE_OPTIONS: LanguageOption[] = LANGUAGE_DEFINITIONS.map(({ code, label }) => ({
  code,
  label,
}));

const STORAGE_KEY = 'cardinal.language';
const DEFAULT_LANGUAGE: SupportedLanguage = 'en-US';

const resources = LANGUAGE_DEFINITIONS.reduce<
  Record<SupportedLanguage, { translation: LanguageDefinition['translation'] }>
>(
  (acc, { code, translation }) => {
    acc[code] = { translation };
    return acc;
  },
  {} as Record<SupportedLanguage, { translation: LanguageDefinition['translation'] }>,
);

const SUPPORTED_LANGUAGES = LANGUAGE_DEFINITIONS.map(({ code }) => code);
const SUPPORTED_LANGUAGE_SET = new Set<SupportedLanguage>(SUPPORTED_LANGUAGES);

const BASE_LANGUAGE_MAP = new Map<string, SupportedLanguage>();
for (const code of SUPPORTED_LANGUAGES) {
  const [base] = code.split('-');
  if (base && !BASE_LANGUAGE_MAP.has(base)) {
    BASE_LANGUAGE_MAP.set(base.toLowerCase(), code);
  }
}

const isSupportedLanguage = (value: string): value is SupportedLanguage =>
  SUPPORTED_LANGUAGE_SET.has(value as SupportedLanguage);

const normalizeStoredLanguage = (stored: string): SupportedLanguage | undefined =>
  isSupportedLanguage(stored) ? stored : undefined;

const normalizeBrowserLanguage = (lng: string): SupportedLanguage => {
  const normalizedInput = lng.replace(/_/g, '-');

  if (isSupportedLanguage(normalizedInput)) {
    return normalizedInput;
  }

  const [rawBase, ...subtags] = normalizedInput
    .split('-')
    .filter((part): part is string => part.length > 0);
  const base = rawBase?.toLowerCase();

  if (base === 'zh') {
    const upperSubtags = subtags.map((subtag) => subtag.toUpperCase());

    if (upperSubtags.includes('HANT')) {
      return 'zh-TW';
    }
    if (upperSubtags.includes('HANS')) {
      return 'zh-CN';
    }
    if (upperSubtags.some((subtag) => subtag === 'TW' || subtag === 'HK' || subtag === 'MO')) {
      return 'zh-TW';
    }
    return 'zh-CN';
  }

  if (base) {
    const mappedLanguage = BASE_LANGUAGE_MAP.get(base);
    if (mappedLanguage) {
      return mappedLanguage;
    }
  }

  return DEFAULT_LANGUAGE;
};

export const normalizeLanguageTag = (lng: string): SupportedLanguage =>
  normalizeBrowserLanguage(lng);

export const getBrowserLanguage = (): SupportedLanguage => {
  if (typeof window === 'undefined') {
    return DEFAULT_LANGUAGE;
  }
  const browserLang = window.navigator.language;
  return browserLang ? normalizeLanguageTag(browserLang) : DEFAULT_LANGUAGE;
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

  return getBrowserLanguage();
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
  supportedLngs: SUPPORTED_LANGUAGES,
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
