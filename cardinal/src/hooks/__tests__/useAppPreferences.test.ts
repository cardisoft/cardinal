import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { OPEN_PREFERENCES_EVENT } from '../../constants/appEvents';
import { applyThemePreference, persistThemePreference } from '../../theme';
import { setTrayEnabled } from '../../tray';
import { getStoredTrayIconEnabled, persistTrayIconEnabled } from '../../trayIconPreference';
import { setWatchConfig } from '../../utils/watchConfig';
import { getBrowserLanguage } from '../../i18n/config';
import { useIgnorePaths } from '../useIgnorePaths';
import { useWatchRoot } from '../useWatchRoot';
import { useAppPreferences } from '../useAppPreferences';
import { invoke } from '@tauri-apps/api/core';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

vi.mock('../useWatchRoot', () => ({
  useWatchRoot: vi.fn(),
}));

vi.mock('../useIgnorePaths', () => ({
  useIgnorePaths: vi.fn(),
}));

vi.mock('../../trayIconPreference', () => ({
  getStoredTrayIconEnabled: vi.fn(),
  persistTrayIconEnabled: vi.fn(),
}));

vi.mock('../../tray', () => ({
  setTrayEnabled: vi.fn(),
}));

vi.mock('../../theme', () => ({
  applyThemePreference: vi.fn(),
  persistThemePreference: vi.fn(),
}));

vi.mock('../../utils/watchConfig', () => ({
  setWatchConfig: vi.fn(),
}));

vi.mock('../../i18n/config', () => ({
  getBrowserLanguage: vi.fn(),
}));

const mockedInvoke = vi.mocked(invoke);
const mockedUseWatchRoot = vi.mocked(useWatchRoot);
const mockedUseIgnorePaths = vi.mocked(useIgnorePaths);
const mockedGetStoredTrayIconEnabled = vi.mocked(getStoredTrayIconEnabled);
const mockedPersistTrayIconEnabled = vi.mocked(persistTrayIconEnabled);
const mockedSetTrayEnabled = vi.mocked(setTrayEnabled);
const mockedPersistThemePreference = vi.mocked(persistThemePreference);
const mockedApplyThemePreference = vi.mocked(applyThemePreference);
const mockedSetWatchConfig = vi.mocked(setWatchConfig);
const mockedGetBrowserLanguage = vi.mocked(getBrowserLanguage);

describe('useAppPreferences', () => {
  const setWatchRoot = vi.fn();
  const setIgnorePaths = vi.fn();
  const changeLanguage = vi.fn().mockResolvedValue(undefined);
  const refreshSearchResults = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();

    mockedUseWatchRoot.mockReturnValue({
      watchRoot: '/workspace',
      setWatchRoot,
      defaultWatchRoot: '/',
    });
    mockedUseIgnorePaths.mockReturnValue({
      ignorePaths: ['/Volumes'],
      setIgnorePaths,
      defaultIgnorePaths: ['/Volumes'],
    });
    mockedGetStoredTrayIconEnabled.mockReturnValue(true);
    mockedSetTrayEnabled.mockResolvedValue(undefined);
    mockedSetWatchConfig.mockResolvedValue(undefined);
    mockedInvoke.mockResolvedValue(undefined);
    mockedGetBrowserLanguage.mockReturnValue('fr-FR');
  });

  it('starts logic once when permission is granted', async () => {
    const { rerender } = renderHook((props) => useAppPreferences(props), {
      initialProps: {
        fullDiskAccessStatus: 'granted' as const,
        isCheckingFullDiskAccess: false,
        refreshSearchResults,
        i18n: { changeLanguage },
      },
    });

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith('start_logic', {
        watchRoot: '/workspace',
        ignorePaths: ['/Volumes'],
      });
    });

    rerender({
      fullDiskAccessStatus: 'granted' as const,
      isCheckingFullDiskAccess: false,
      refreshSearchResults,
      i18n: { changeLanguage },
    });

    expect(mockedInvoke).toHaveBeenCalledTimes(1);
  });

  it('updates watch config and refreshes search when preferences change', async () => {
    const { result } = renderHook(() =>
      useAppPreferences({
        fullDiskAccessStatus: 'granted',
        isCheckingFullDiskAccess: false,
        refreshSearchResults,
        i18n: { changeLanguage },
      }),
    );

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith('start_logic', {
        watchRoot: '/workspace',
        ignorePaths: ['/Volumes'],
      });
    });

    mockedSetWatchConfig.mockClear();
    refreshSearchResults.mockClear();

    act(() => {
      result.current.handleWatchConfigChange({
        watchRoot: '/tmp',
        ignorePaths: ['/tmp/ignore'],
      });
    });

    expect(setWatchRoot).toHaveBeenCalledWith('/tmp');
    expect(setIgnorePaths).toHaveBeenCalledWith(['/tmp/ignore']);
    expect(mockedSetWatchConfig).toHaveBeenCalledWith({
      watchRoot: '/tmp',
      ignorePaths: ['/tmp/ignore'],
    });
    expect(refreshSearchResults).toHaveBeenCalledTimes(1);
  });

  it('skips setWatchConfig when watch config is unchanged', async () => {
    const { result } = renderHook(() =>
      useAppPreferences({
        fullDiskAccessStatus: 'granted',
        isCheckingFullDiskAccess: false,
        refreshSearchResults,
        i18n: { changeLanguage },
      }),
    );

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith('start_logic', {
        watchRoot: '/workspace',
        ignorePaths: ['/Volumes'],
      });
    });

    mockedSetWatchConfig.mockClear();
    refreshSearchResults.mockClear();
    setWatchRoot.mockClear();
    setIgnorePaths.mockClear();

    act(() => {
      result.current.handleWatchConfigChange({
        watchRoot: '/workspace',
        ignorePaths: ['/Volumes'],
      });
    });

    expect(setWatchRoot).not.toHaveBeenCalled();
    expect(setIgnorePaths).not.toHaveBeenCalled();
    expect(mockedSetWatchConfig).not.toHaveBeenCalled();
    expect(refreshSearchResults).not.toHaveBeenCalled();
  });

  it('opens and closes preferences, and resets user preferences', async () => {
    const { result } = renderHook(() =>
      useAppPreferences({
        fullDiskAccessStatus: 'denied',
        isCheckingFullDiskAccess: false,
        refreshSearchResults,
        i18n: { changeLanguage },
      }),
    );

    await waitFor(() => {
      expect(mockedSetTrayEnabled).toHaveBeenCalledWith(true);
    });

    mockedSetTrayEnabled.mockClear();
    mockedPersistTrayIconEnabled.mockClear();

    act(() => {
      window.dispatchEvent(new Event(OPEN_PREFERENCES_EVENT));
    });
    expect(result.current.isPreferencesOpen).toBe(true);

    act(() => {
      result.current.closePreferences();
    });
    expect(result.current.isPreferencesOpen).toBe(false);

    const initialToken = result.current.preferencesResetToken;
    act(() => {
      result.current.handleResetPreferences();
    });

    await waitFor(() => {
      expect(mockedSetTrayEnabled).toHaveBeenCalledWith(false);
    });
    expect(mockedPersistTrayIconEnabled).toHaveBeenCalledWith(false);
    expect(mockedPersistThemePreference).toHaveBeenCalledWith('system');
    expect(mockedApplyThemePreference).toHaveBeenCalledWith('system');
    expect(changeLanguage).toHaveBeenCalledWith('fr-FR');
    expect(result.current.preferencesResetToken).toBe(initialToken + 1);
  });
});
