import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';
import en from './resources/en.json';
import zh from './resources/zh.json';
import ja from './resources/ja.json';
import ko from './resources/ko.json';
import fr from './resources/fr.json';
import es from './resources/es.json';
import pt from './resources/pt.json';
import de from './resources/de.json';
import it from './resources/it.json';
import ru from './resources/ru.json';
import uk from './resources/uk.json';
import ar from './resources/ar.json';
import hi from './resources/hi.json';
import tr from './resources/tr.json';

export type SupportedLanguage =
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

type LanguageOption = {
  code: SupportedLanguage;
  label: string;
};

export const LANGUAGE_OPTIONS: LanguageOption[] = [
  { code: 'en', label: 'English' },
  { code: 'zh', label: '中文' },
  { code: 'ja', label: '日本語' },
  { code: 'ko', label: '한국어' },
  { code: 'fr', label: 'Français' },
  { code: 'es', label: 'Español' },
  { code: 'pt', label: 'Português' },
  { code: 'de', label: 'Deutsch' },
  { code: 'it', label: 'Italiano' },
  { code: 'ru', label: 'Русский' },
  { code: 'uk', label: 'Українська' },
  { code: 'ar', label: 'العربية' },
  { code: 'hi', label: 'हिन्दी' },
  { code: 'tr', label: 'Türkçe' },
];

const STORAGE_KEY = 'cardinal.language';
const DEFAULT_LANGUAGE: SupportedLanguage = 'en';

const resources = {
  en: { translation: en },
  zh: { translation: zh },
  ja: { translation: ja },
  ko: { translation: ko },
  fr: { translation: fr },
  es: { translation: es },
  pt: { translation: pt },
  de: { translation: de },
  it: { translation: it },
  ru: { translation: ru },
  uk: { translation: uk },
  ar: { translation: ar },
  hi: { translation: hi },
  tr: { translation: tr },
} as const;

const detectInitialLanguage = (): SupportedLanguage => {
  if (typeof window === 'undefined') {
    return DEFAULT_LANGUAGE;
  }

  try {
    const stored = window.localStorage.getItem(STORAGE_KEY) as SupportedLanguage | null;
    if (stored && resources[stored]) {
      return stored;
    }
  } catch (error) {
    console.warn('Unable to read saved language preference', error);
  }

  const browserLang = window.navigator.language?.split('-')?.[0] as SupportedLanguage | undefined;
  if (browserLang && resources[browserLang]) {
    return browserLang;
  }

  return DEFAULT_LANGUAGE;
};

void i18n.use(initReactI18next).init({
  resources,
  lng: detectInitialLanguage(),
  fallbackLng: DEFAULT_LANGUAGE,
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
