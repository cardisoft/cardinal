import { useCallback, useEffect } from 'react';
import { useStoredState } from './useStoredState';
import { broadcastUiPreferencesChanged } from '../runtime/uiPreferences';
import {
  FONT_SIZE_DEFAULT,
  clampFontSize,
  rowHeightForFontSize,
  type FontSizeSettings,
} from '../types/fontSize';

export const FONT_SIZE_STORAGE_KEY = 'cardinal.fontSize.v1';

const readFontSize = (raw: string): FontSizeSettings | null => {
  const parsed = Number.parseFloat(raw);
  if (!Number.isFinite(parsed)) {
    return null;
  }
  return { base: clampFontSize(parsed) };
};

const writeFontSize = (value: FontSizeSettings): string => String(value.base);

export type FontSizeController = {
  fontSize: number;
  rowHeight: number;
  setFontSize: (value: number) => void;
  resetFontSize: () => void;
};

/**
 * Global UI font size. Applied as `--app-font-size` on the document root so every element
 * sized in `rem` (results rows, bookmarks, filter dialogs, buttons) scales together.
 */
export function useFontSize(): FontSizeController {
  const [settings, setSettings] = useStoredState<FontSizeSettings>({
    key: FONT_SIZE_STORAGE_KEY,
    defaultValue: { base: FONT_SIZE_DEFAULT },
    read: readFontSize,
    write: writeFontSize,
    normalize: (value) => ({ base: clampFontSize(value.base) }),
    readErrorMessage: 'Unable to read saved font size',
    writeErrorMessage: 'Unable to persist font size',
  });

  useEffect(() => {
    if (typeof document === 'undefined') {
      return;
    }
    document.documentElement.style.setProperty('--app-font-size', `${settings.base}px`);
    // The bookmarks window sizes everything in `rem` too, so it needs the same value.
    broadcastUiPreferencesChanged();
  }, [settings.base]);

  const setFontSize = useCallback(
    (value: number) => {
      setSettings({ base: clampFontSize(value) });
    },
    [setSettings],
  );

  const resetFontSize = useCallback(() => {
    setSettings({ base: FONT_SIZE_DEFAULT });
  }, [setSettings]);

  return {
    fontSize: settings.base,
    rowHeight: rowHeightForFontSize(settings.base),
    setFontSize,
    resetFontSize,
  };
}
