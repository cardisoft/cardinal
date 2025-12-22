import { ViewMode, DEFAULT_ICON_SIZE } from '../constants';

const VIEW_MODE_STORAGE_KEY = 'cardinal.viewMode';
const ICON_SIZE_STORAGE_KEY = 'cardinal.gridIconSize';

export const getStoredViewMode = (): ViewMode => {
  if (typeof window === 'undefined') return 'list';
  const stored = window.localStorage.getItem(VIEW_MODE_STORAGE_KEY);
  return stored === 'list' || stored === 'grid' ? stored : 'list';
};

export const persistViewMode = (mode: ViewMode): void => {
  if (typeof window === 'undefined') return;
  try {
    window.localStorage.setItem(VIEW_MODE_STORAGE_KEY, mode);
  } catch {
    // Ignore storage failures.
  }
};

export const getStoredIconSize = (): number => {
  if (typeof window === 'undefined') return DEFAULT_ICON_SIZE;
  const stored = window.localStorage.getItem(ICON_SIZE_STORAGE_KEY);
  if (stored) {
    const parsed = parseInt(stored, 10);
    if (!isNaN(parsed)) return parsed;
  }
  return DEFAULT_ICON_SIZE;
};

export const persistIconSize = (size: number): void => {
  if (typeof window === 'undefined') return;
  try {
    window.localStorage.setItem(ICON_SIZE_STORAGE_KEY, String(size));
  } catch {
    // Ignore storage failures.
  }
};
