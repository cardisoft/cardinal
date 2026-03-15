import { beforeEach, describe, expect, it } from 'vitest';
import {
  DEFAULT_SHORTCUTS,
  SHORTCUTS_STORAGE_KEY,
  getStoredShortcutAccelerators,
  getStoredShortcuts,
  persistShortcuts,
} from '../shortcuts';

describe('shortcuts storage', () => {
  beforeEach(() => {
    window.localStorage.clear();
  });

  it('hydrates defaults when storage is empty', () => {
    expect(getStoredShortcuts()).toEqual(DEFAULT_SHORTCUTS);
  });

  it('persists and restores local shortcut overrides', () => {
    persistShortcuts({
      ...DEFAULT_SHORTCUTS,
      openResult: 'Command+P',
      searchHistoryUp: 'Control+K',
    });

    expect(getStoredShortcuts()).toMatchObject({
      ...DEFAULT_SHORTCUTS,
      openResult: 'Command+P',
      searchHistoryUp: 'Control+K',
    });
  });

  it('writes unified shortcuts to localStorage', () => {
    persistShortcuts(DEFAULT_SHORTCUTS);

    expect(window.localStorage.getItem(SHORTCUTS_STORAGE_KEY)).toBeTruthy();
  });

  it('falls back to defaults when storage payload is invalid', () => {
    window.localStorage.setItem(SHORTCUTS_STORAGE_KEY, '{bad json');

    expect(getStoredShortcuts()).toEqual(DEFAULT_SHORTCUTS);
  });

  it('builds app menu/tray accelerator values from stored shortcuts', () => {
    persistShortcuts({
      ...DEFAULT_SHORTCUTS,
      openPreferences: 'Command+Comma',
      hideWindow: 'Esc',
      quickLaunch: 'Command+Shift+Space',
    });

    expect(getStoredShortcutAccelerators()).toEqual({
      quickLaunch: 'Command+Shift+Space',
      openPreferences: 'Cmd+,',
      hideWindow: 'Esc',
    });
  });
});
