import { beforeEach, describe, expect, it, vi } from 'vitest';

import i18n, { LANGUAGE_OPTIONS, __test__ } from '../config';

beforeEach(() => {
  window.localStorage.clear();
  vi.restoreAllMocks();
});

describe('i18n locale normalization', () => {
  it('normalizes stored language codes (supported + legacy)', () => {
    expect(__test__.normalizeStoredLanguage('en-US')).toBe('en-US');
    expect(__test__.normalizeStoredLanguage('zh-TW')).toBe('zh-TW');

    expect(__test__.normalizeStoredLanguage('en')).toBe('en-US');
    expect(__test__.normalizeStoredLanguage('zh')).toBe('zh-CN');

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
    expect(__test__.normalizeBrowserLanguage('zh-Hant')).toBe('zh-TW');
    expect(__test__.normalizeBrowserLanguage('zh-hk')).toBe('zh-TW');
    expect(__test__.normalizeBrowserLanguage('zh-MO')).toBe('zh-TW');

    expect(__test__.normalizeBrowserLanguage('unknown')).toBe('en-US');
  });

  it('detects initial language from localStorage first (supported + legacy)', () => {
    window.localStorage.setItem('cardinal.language', 'fr-FR');
    expect(__test__.detectInitialLanguage()).toBe('fr-FR');

    window.localStorage.setItem('cardinal.language', 'fr');
    expect(__test__.detectInitialLanguage()).toBe('fr-FR');
  });

  it('falls back to browser language when localStorage is invalid', () => {
    window.localStorage.setItem('cardinal.language', 'not-a-language');

    const navigatorLanguage = vi
      .spyOn(window.navigator, 'language', 'get')
      .mockReturnValue('zh-Hant');

    expect(__test__.detectInitialLanguage()).toBe('zh-TW');
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
});

