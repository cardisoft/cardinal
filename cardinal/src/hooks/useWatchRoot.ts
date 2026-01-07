import { useCallback, useEffect, useState } from 'react';

const STORAGE_KEY = 'cardinal.watchRoot';
const DEFAULT_WATCH_ROOT = '/';

const readStoredValue = (): string | null => {
  try {
    const stored = window.localStorage.getItem(STORAGE_KEY);
    if (!stored) return null;
    const trimmed = stored.trim();
    return trimmed.length > 0 ? trimmed : null;
  } catch (error) {
    console.warn('Unable to read saved watch root', error);
    return null;
  }
};

export function useWatchRoot() {
  const [watchRoot, setWatchRootState] = useState<string>(
    () => readStoredValue() ?? DEFAULT_WATCH_ROOT,
  );

  useEffect(() => {
    const stored = readStoredValue();
    if (stored) return;
    try {
      window.localStorage.setItem(STORAGE_KEY, DEFAULT_WATCH_ROOT);
    } catch (error) {
      console.warn('Unable to persist default watch root', error);
    }
  }, []);

  const setWatchRoot = useCallback((next: string) => {
    const trimmed = next.trim();
    const normalized = trimmed.length > 0 ? trimmed : DEFAULT_WATCH_ROOT;
    setWatchRootState(normalized);
    try {
      window.localStorage.setItem(STORAGE_KEY, normalized);
    } catch (error) {
      console.warn('Unable to persist watch root', error);
    }
  }, []);

  return { watchRoot, setWatchRoot, defaultWatchRoot: DEFAULT_WATCH_ROOT };
}
