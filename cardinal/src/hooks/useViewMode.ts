import { useCallback } from 'react';
import { useStoredState } from './useStoredState';
import type { IconSizePreset } from '../components/IconGrid';
import type { ResultsViewMode } from '../types/viewResults';

type ViewPreferences = {
  mode: ResultsViewMode;
  iconSize: IconSizePreset;
};

const VIEW_MODE_STORAGE_KEY = 'cardinal.viewMode.v1';

const isValidMode = (value: unknown): value is ResultsViewMode =>
  value === 'list' || value === 'grid';

const isValidIconSize = (value: unknown): value is IconSizePreset =>
  value === 'small' || value === 'medium' || value === 'large' || value === 'xlarge';

const readViewMode = (raw: string): ViewPreferences | null => {
  try {
    const parsed = JSON.parse(raw) as Partial<ViewPreferences>;
    if (!parsed || !isValidMode(parsed.mode) || !isValidIconSize(parsed.iconSize)) {
      return null;
    }
    return { mode: parsed.mode, iconSize: parsed.iconSize };
  } catch {
    return null;
  }
};

const writeViewMode = (value: ViewPreferences): string => JSON.stringify(value);

const defaultViewMode: ViewPreferences = { mode: 'list', iconSize: 'medium' };

export type ViewModeController = {
  viewMode: ResultsViewMode;
  iconSize: IconSizePreset;
  setViewMode: (mode: ResultsViewMode) => void;
  setIconSize: (size: IconSizePreset) => void;
};

export function useViewMode(): ViewModeController {
  const [prefs, setPrefs] = useStoredState<ViewPreferences>({
    key: VIEW_MODE_STORAGE_KEY,
    defaultValue: defaultViewMode,
    read: readViewMode,
    write: writeViewMode,
    normalize: (value) => value,
    readErrorMessage: 'Unable to read view mode preference',
    writeErrorMessage: 'Unable to persist view mode preference',
  });

  const setViewMode = useCallback(
    (mode: ResultsViewMode) => {
      setPrefs((prev) => ({ ...prev, mode }));
    },
    [setPrefs],
  );

  const setIconSize = useCallback(
    (size: IconSizePreset) => {
      setPrefs((prev) => ({ ...prev, iconSize: size }));
    },
    [setPrefs],
  );

  return {
    viewMode: prefs.mode,
    iconSize: prefs.iconSize,
    setViewMode,
    setIconSize,
  };
}
