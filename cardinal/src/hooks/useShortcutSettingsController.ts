import { useCallback, useEffect, useState } from 'react';
import { refreshAppMenu, setMenuShortcutsEnabled } from '../menu';
import {
  DEFAULT_SHORTCUTS,
  getStoredShortcuts,
  persistShortcuts,
  type ShortcutMap,
} from '../shortcuts';
import { refreshTrayMenu } from '../tray';
import { setGlobalShortcutsPaused, updateQuickLaunchShortcut } from '../utils/globalShortcuts';

type UseShortcutSettingsControllerResult = {
  isShortcutSettingsOpen: boolean;
  shortcuts: ShortcutMap;
  defaultShortcuts: ShortcutMap;
  openShortcutSettings: () => void;
  closeShortcutSettings: () => void;
  handleShortcutSettingsSave: (nextShortcuts: ShortcutMap) => Promise<void>;
};

export function useShortcutSettingsController(): UseShortcutSettingsControllerResult {
  const [isShortcutSettingsOpen, setIsShortcutSettingsOpen] = useState(false);
  const [shortcuts, setShortcuts] = useState<ShortcutMap>(() => getStoredShortcuts());

  useEffect(() => {
    void setGlobalShortcutsPaused(isShortcutSettingsOpen);
    setMenuShortcutsEnabled(!isShortcutSettingsOpen);
    return () => {
      if (isShortcutSettingsOpen) {
        void setGlobalShortcutsPaused(false);
        setMenuShortcutsEnabled(true);
      }
    };
  }, [isShortcutSettingsOpen]);

  const openShortcutSettings = useCallback(() => {
    setIsShortcutSettingsOpen(true);
  }, []);

  const closeShortcutSettings = useCallback(() => {
    setIsShortcutSettingsOpen(false);
  }, []);

  const handleShortcutSettingsSave = useCallback(async (nextShortcuts: ShortcutMap) => {
    await updateQuickLaunchShortcut(nextShortcuts.quickLaunch);
    persistShortcuts(nextShortcuts);
    setShortcuts(nextShortcuts);
    await Promise.all([refreshAppMenu(), refreshTrayMenu()]);
  }, []);

  return {
    isShortcutSettingsOpen,
    shortcuts,
    defaultShortcuts: DEFAULT_SHORTCUTS,
    openShortcutSettings,
    closeShortcutSettings,
    handleShortcutSettingsSave,
  };
}
