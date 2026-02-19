import { beforeEach, describe, expect, it, vi } from 'vitest';

import i18n, { LANGUAGE_OPTIONS, __test__ } from '../config';

beforeEach(() => {
  window.localStorage.clear();
  vi.restoreAllMocks();
});

describe('i18n locale normalization', () => {
  it('normalizes stored language codes (supported only)', () => {
    expect(__test__.normalizeStoredLanguage('en-US')).toBe('en-US');
    expect(__test__.normalizeStoredLanguage('zh-TW')).toBe('zh-TW');

    expect(__test__.normalizeStoredLanguage('en')).toBeUndefined();
    expect(__test__.normalizeStoredLanguage('zh')).toBeUndefined();

    expect(__test__.normalizeStoredLanguage('does-not-exist')).toBeUndefined();
  });

  it('keeps LANGUAGE_OPTIONS aligned with resources', () => {
    for (const option of LANGUAGE_OPTIONS) {
      expect(__test__.normalizeStoredLanguage(option.code)).toBe(option.code);
    }
  });

  it('normalizes browser language tags', () => {
    expect(__test__.normalizeBrowserLanguage('en-GB')).toBe('en-US');
    expect(__test__.normalizeBrowserLanguage('pt-PT')).toBe('pt-BR');

    expect(__test__.normalizeBrowserLanguage('zh')).toBe('zh-CN');
    expect(__test__.normalizeBrowserLanguage('zh-Hans')).toBe('zh-CN');
    expect(__test__.normalizeBrowserLanguage('zh-Hans-CN')).toBe('zh-CN');
    expect(__test__.normalizeBrowserLanguage('zh-Hant')).toBe('zh-TW');
    expect(__test__.normalizeBrowserLanguage('zh-Hant-HK')).toBe('zh-TW');
    expect(__test__.normalizeBrowserLanguage('zh-hk')).toBe('zh-TW');
    expect(__test__.normalizeBrowserLanguage('zh-MO')).toBe('zh-TW');
    expect(__test__.normalizeBrowserLanguage('zh_TW')).toBe('zh-TW');

    expect(__test__.normalizeBrowserLanguage('unknown')).toBe('en-US');
  });

  it('normalizes base language browser tags correctly', () => {
    expect(__test__.normalizeBrowserLanguage('en')).toBe('en-US');
    expect(__test__.normalizeBrowserLanguage('ja')).toBe('ja-JP');
    expect(__test__.normalizeBrowserLanguage('ko')).toBe('ko-KR');
    expect(__test__.normalizeBrowserLanguage('fr')).toBe('fr-FR');
    expect(__test__.normalizeBrowserLanguage('es')).toBe('es-ES');
    expect(__test__.normalizeBrowserLanguage('pt')).toBe('pt-BR');
    expect(__test__.normalizeBrowserLanguage('de')).toBe('de-DE');
    expect(__test__.normalizeBrowserLanguage('it')).toBe('it-IT');
    expect(__test__.normalizeBrowserLanguage('ru')).toBe('ru-RU');
    expect(__test__.normalizeBrowserLanguage('uk')).toBe('uk-UA');
    expect(__test__.normalizeBrowserLanguage('ar')).toBe('ar-SA');
    expect(__test__.normalizeBrowserLanguage('hi')).toBe('hi-IN');
    expect(__test__.normalizeBrowserLanguage('tr')).toBe('tr-TR');
  });

  it('handles underscore separators in browser language tags', () => {
    expect(__test__.normalizeBrowserLanguage('zh_CN')).toBe('zh-CN');
    expect(__test__.normalizeBrowserLanguage('zh_TW')).toBe('zh-TW');
    expect(__test__.normalizeBrowserLanguage('en_US')).toBe('en-US');
  });

  it('normalizes mixed-case browser language tags', () => {
    expect(__test__.normalizeBrowserLanguage('EN_gb')).toBe('en-US');
    expect(__test__.normalizeBrowserLanguage('PT_br')).toBe('pt-BR');
    expect(__test__.normalizeBrowserLanguage('ZH_hant_hk')).toBe('zh-TW');
  });

  it('detects initial language from localStorage first (supported only)', () => {
    window.localStorage.setItem('cardinal.language', 'fr-FR');
    expect(__test__.detectInitialLanguage()).toBe('fr-FR');

    window.localStorage.setItem('cardinal.language', 'fr');
    const navigatorLanguage = vi
      .spyOn(window.navigator, 'language', 'get')
      .mockReturnValue('ja-JP');

    expect(__test__.detectInitialLanguage()).toBe('ja-JP');
    navigatorLanguage.mockRestore();
  });

  it('falls back to browser language when localStorage is invalid', () => {
    window.localStorage.setItem('cardinal.language', 'not-a-language');

    const navigatorLanguage = vi
      .spyOn(window.navigator, 'language', 'get')
      .mockReturnValue('zh-Hant');

    expect(__test__.detectInitialLanguage()).toBe('zh-TW');
    navigatorLanguage.mockRestore();
  });

  it('falls back to browser language when stored language casing is invalid', () => {
    window.localStorage.setItem('cardinal.language', 'fr-fr');

    const navigatorLanguage = vi
      .spyOn(window.navigator, 'language', 'get')
      .mockReturnValue('de-DE');

    expect(__test__.detectInitialLanguage()).toBe('de-DE');
    navigatorLanguage.mockRestore();
  });

  it('falls back to browser language when localStorage throws', () => {
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => {});

    const localStorageGetItem = vi.spyOn(Storage.prototype, 'getItem').mockImplementation(() => {
      throw new Error('blocked');
    });

    const navigatorLanguage = vi
      .spyOn(window.navigator, 'language', 'get')
      .mockReturnValue('ja-JP');

    expect(__test__.detectInitialLanguage()).toBe('ja-JP');

    navigatorLanguage.mockRestore();
    localStorageGetItem.mockRestore();
    warn.mockRestore();
  });

  it('initializes i18n with a supported language', () => {
    expect(LANGUAGE_OPTIONS.map((option) => option.code)).toContain(i18n.language);
  });

  it('handles empty localStorage gracefully', () => {
    window.localStorage.setItem('cardinal.language', '');

    const navigatorLanguage = vi
      .spyOn(window.navigator, 'language', 'get')
      .mockReturnValue('fr-FR');

    expect(__test__.detectInitialLanguage()).toBe('fr-FR');
    navigatorLanguage.mockRestore();
  });

  it('detects initial language in SSR environment (window undefined)', () => {
    const originalWindow = global.window;

    // @ts-expect-error - testing SSR scenario
    delete global.window;

    expect(__test__.detectInitialLanguage()).toBe('en-US');

    global.window = originalWindow;
  });
});

describe('i18n language change events', () => {
  it('persists language to localStorage on change', () => {
    i18n.changeLanguage('ja-JP');
    expect(window.localStorage.getItem('cardinal.language')).toBe('ja-JP');
  });

  it('updates document.documentElement.lang on change', () => {
    i18n.changeLanguage('ko-KR');
    expect(document.documentElement.lang).toBe('ko-KR');
  });

  it('handles localStorage write errors gracefully', () => {
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => {});
    const setItemSpy = vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
      throw new Error('quota exceeded');
    });

    expect(() => i18n.changeLanguage('de-DE')).not.toThrow();

    setItemSpy.mockRestore();
    warn.mockRestore();
  });
});
