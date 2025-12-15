const TRAY_ICON_ENABLED_STORAGE_KEY = 'cardinal.trayIconEnabled';

export const getStoredTrayIconEnabled = (): boolean => {
  const stored = window.localStorage.getItem(TRAY_ICON_ENABLED_STORAGE_KEY);
  return stored === 'true';
};

export const persistTrayIconEnabled = (enabled: boolean): void => {
  try {
    if (enabled) {
      window.localStorage.setItem(TRAY_ICON_ENABLED_STORAGE_KEY, 'true');
    } else {
      window.localStorage.removeItem(TRAY_ICON_ENABLED_STORAGE_KEY);
    }
  } catch {
    // Ignore storage failures.
  }
};
