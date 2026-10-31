import { beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
  emit: vi.fn(),
  listen: vi.fn(),
  handlers: new Map<string, () => void>(),
  changeLanguage: vi.fn(),
}));

vi.mock('@tauri-apps/api/event', () => ({
  emit: (...args: unknown[]) => mocks.emit(...args),
  listen: (event: string, handler: () => void) => {
    mocks.listen(event, handler);
    mocks.handlers.set(event, handler);
    return Promise.resolve(() => mocks.handlers.delete(event));
  },
}));

import {
  UI_PREFERENCES_CHANGED_EVENT,
  applyStoredUiPreferences,
  broadcastUiPreferencesChanged,
  subscribeUiPreferencesChanged,
} from '../uiPreferences';

describe('ui preferences sync', () => {
  beforeEach(() => {
    window.localStorage.clear();
    document.documentElement.removeAttribute('data-theme');
    document.documentElement.style.removeProperty('--app-font-size');
    mocks.emit.mockClear();
    mocks.listen.mockClear();
    mocks.handlers.clear();
    mocks.changeLanguage.mockClear();
  });

  it('applies the stored theme to the current document', () => {
    window.localStorage.setItem('cardinal.theme', 'dark');
    applyStoredUiPreferences();
    expect(document.documentElement.getAttribute('data-theme')).toBe('dark');
  });

  it('clears the attribute when the preference follows the system', () => {
    document.documentElement.setAttribute('data-theme', 'dark');
    window.localStorage.removeItem('cardinal.theme');
    applyStoredUiPreferences();
    expect(document.documentElement.hasAttribute('data-theme')).toBe(false);
  });

  it('applies the stored font size so the bookmarks window matches the main one', () => {
    // The bookmarks window sizes everything in `rem`; without this it stayed at the default.
    window.localStorage.setItem('cardinal.fontSize.v1', '18');
    applyStoredUiPreferences();
    expect(document.documentElement.style.getPropertyValue('--app-font-size')).toBe('18px');
  });

  it('falls back to the default font size for a corrupt value', () => {
    window.localStorage.setItem('cardinal.fontSize.v1', 'not-a-number');
    applyStoredUiPreferences();
    expect(document.documentElement.style.getPropertyValue('--app-font-size')).toBe('13px');
  });

  it('broadcasts a change so the other window repaints', () => {
    broadcastUiPreferencesChanged();
    expect(mocks.emit).toHaveBeenCalledWith(UI_PREFERENCES_CHANGED_EVENT);
  });

  it('re-applies the stored values when the other window broadcasts', async () => {
    const unsubscribe = subscribeUiPreferencesChanged();
    await Promise.resolve();

    window.localStorage.setItem('cardinal.theme', 'dark');
    window.localStorage.setItem('cardinal.fontSize.v1', '20');
    mocks.handlers.get(UI_PREFERENCES_CHANGED_EVENT)?.();

    expect(document.documentElement.getAttribute('data-theme')).toBe('dark');
    expect(document.documentElement.style.getPropertyValue('--app-font-size')).toBe('20px');

    unsubscribe();
  });

  it('runs the language hook after re-applying', async () => {
    // main.tsx passes a callback that switches i18n; this module stays i18n-free on purpose.
    const onAfterApply = vi.fn();
    subscribeUiPreferencesChanged({ onAfterApply });
    await Promise.resolve();

    mocks.handlers.get(UI_PREFERENCES_CHANGED_EVENT)?.();
    expect(onAfterApply).toHaveBeenCalledTimes(1);
  });
});
