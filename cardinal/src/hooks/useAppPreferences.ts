import { useCallback, useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { i18n as I18nInstance } from 'i18next';
import { OPEN_PREFERENCES_EVENT } from '../constants/appEvents';
import { getBrowserLanguage } from '../i18n/config';
import { applyThemePreference, persistThemePreference } from '../theme';
import { setTrayEnabled } from '../tray';
import { getStoredTrayIconEnabled, persistTrayIconEnabled } from '../trayIconPreference';
import { setWatchConfig } from '../utils/watchConfig';
import type { FullDiskAccessStatus } from './useFullDiskAccessPermission';
import { useIgnorePaths } from './useIgnorePaths';
import { useIncludePaths } from './useIncludePaths';
import { useServerConfig } from './useServerConfig';
import type { ServerConfig } from './useServerConfig';
import { useWatchRoot } from './useWatchRoot';

type WatchConfigChangePayload = {
  watchRoot: string;
  ignorePaths: string[];
  includePaths: string[];
};

type ServerConfigPayload = {
  serverConfig: ServerConfig;
};

type PreferencesChangePayload = WatchConfigChangePayload & ServerConfigPayload;

type UseAppPreferencesOptions = {
  fullDiskAccessStatus: FullDiskAccessStatus;
  isCheckingFullDiskAccess: boolean;
  refreshSearchResults: () => void;
  i18n: Pick<I18nInstance, 'changeLanguage'>;
};

type UseAppPreferencesResult = {
  isPreferencesOpen: boolean;
  closePreferences: () => void;
  trayIconEnabled: boolean;
  setTrayIconEnabled: (enabled: boolean) => void;
  watchRoot: string;
  defaultWatchRoot: string;
  ignorePaths: string[];
  defaultIgnorePaths: string[];
  includePaths: string[];
  defaultIncludePaths: string[];
  serverConfig: ServerConfig;
  defaultServerConfig: ServerConfig;
  preferencesResetToken: number;
  handlePreferencesChange: (next: PreferencesChangePayload) => void;
  handleResetPreferences: () => void;
};

const areStringArraysEqual = (left: string[], right: string[]): boolean =>
  left.length === right.length && left.every((value, index) => value === right[index]);

/**
 * Manages app preferences including watch config, tray, theme, language, and overlay state.
 * Provides actions for updating watch settings and resetting preferences to defaults.
 */
export function useAppPreferences({
  fullDiskAccessStatus,
  isCheckingFullDiskAccess,
  refreshSearchResults,
  i18n,
}: UseAppPreferencesOptions): UseAppPreferencesResult {
  const { watchRoot, setWatchRoot, defaultWatchRoot } = useWatchRoot();
  const { ignorePaths, setIgnorePaths, defaultIgnorePaths } = useIgnorePaths();
  const { includePaths, setIncludePaths, defaultIncludePaths } = useIncludePaths();
  const { serverConfig, setServerConfig, defaultServerConfig } = useServerConfig();
  const logicStartedRef = useRef(false);
  const [isPreferencesOpen, setIsPreferencesOpen] = useState(false);
  const [trayIconEnabled, setTrayIconEnabled] = useState<boolean>(() => getStoredTrayIconEnabled());
  const [preferencesResetToken, setPreferencesResetToken] = useState(0);

  useEffect(() => {
    persistTrayIconEnabled(trayIconEnabled);
    void setTrayEnabled(trayIconEnabled);
  }, [trayIconEnabled]);

  useEffect(() => {
    if (typeof window === 'undefined') {
      return;
    }

    const handleOpenPreferences = () => setIsPreferencesOpen(true);
    window.addEventListener(OPEN_PREFERENCES_EVENT, handleOpenPreferences);
    return () => window.removeEventListener(OPEN_PREFERENCES_EVENT, handleOpenPreferences);
  }, []);

  useEffect(() => {
    if (isCheckingFullDiskAccess) {
      return;
    }
    if (fullDiskAccessStatus !== 'granted') {
      return;
    }
    if (!watchRoot) {
      return;
    }
    if (logicStartedRef.current) {
      return;
    }

    logicStartedRef.current = true;
    void invoke('start_logic', { watchRoot, ignorePaths, includePaths });
  }, [fullDiskAccessStatus, ignorePaths, includePaths, isCheckingFullDiskAccess, watchRoot]);

  const applyWatchConfig = useCallback(
    (nextWatchRoot: string, nextIgnorePaths: string[], nextIncludePaths: string[]) => {
      const watchConfigChanged =
        nextWatchRoot !== watchRoot ||
        !areStringArraysEqual(nextIgnorePaths, ignorePaths) ||
        !areStringArraysEqual(nextIncludePaths, includePaths);

      if (!watchConfigChanged) {
        return;
      }

      setWatchRoot(nextWatchRoot);
      setIgnorePaths(nextIgnorePaths);
      setIncludePaths(nextIncludePaths);
      if (logicStartedRef.current && nextWatchRoot) {
        void setWatchConfig({
          watchRoot: nextWatchRoot,
          ignorePaths: nextIgnorePaths,
          includePaths: nextIncludePaths,
        });
      }
      refreshSearchResults();
    },
    [
      ignorePaths,
      includePaths,
      refreshSearchResults,
      setIgnorePaths,
      setIncludePaths,
      setWatchRoot,
      watchRoot,
    ],
  );

  const applyServerConfig = useCallback(
    (next: ServerConfig) => {
      if (next.enabled === serverConfig.enabled && next.endpoint === serverConfig.endpoint) {
        return;
      }

      setServerConfig(next);
      void invoke('set_server_config', { config: next });
    },
    [serverConfig.enabled, serverConfig.endpoint, setServerConfig],
  );

  const handlePreferencesChange = useCallback(
    (next: PreferencesChangePayload) => {
      applyWatchConfig(next.watchRoot, next.ignorePaths, next.includePaths);
      applyServerConfig(next.serverConfig);
    },
    [applyServerConfig, applyWatchConfig],
  );

  const handleResetPreferences = useCallback(() => {
    setTrayIconEnabled(false);
    applyServerConfig(defaultServerConfig);
    persistThemePreference('system');
    applyThemePreference('system');
    const nextLanguage = getBrowserLanguage();
    void i18n.changeLanguage(nextLanguage);
    setPreferencesResetToken((token) => token + 1);
  }, [applyServerConfig, defaultServerConfig, i18n]);

  const closePreferences = useCallback(() => setIsPreferencesOpen(false), []);

  return {
    isPreferencesOpen,
    closePreferences,
    trayIconEnabled,
    setTrayIconEnabled,
    watchRoot,
    defaultWatchRoot,
    ignorePaths,
    defaultIgnorePaths,
    includePaths,
    defaultIncludePaths,
    serverConfig,
    defaultServerConfig,
    preferencesResetToken,
    handlePreferencesChange,
    handleResetPreferences,
  };
}
