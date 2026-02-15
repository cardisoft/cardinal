import { useCallback } from 'react';
import { useStoredState } from './useStoredState';

const STORAGE_KEY = 'cardinal.watchRoot';
const DEFAULT_WATCH_ROOT = '/';

export function useWatchRoot() {
  const [watchRoot, setWatchRootState] = useStoredState<string>({
    key: STORAGE_KEY,
    defaultValue: DEFAULT_WATCH_ROOT,
    read: (raw) => {
      const trimmed = raw.trim();
      return trimmed.length > 0 ? trimmed : null;
    },
    write: (value) => value,
    readErrorMessage: 'Unable to read saved watch root',
    writeErrorMessage: 'Unable to persist default watch root',
  });

  const setWatchRoot = useCallback((next: string) => {
    const trimmed = next.trim();
    const normalized = trimmed.length > 0 ? trimmed : DEFAULT_WATCH_ROOT;
    setWatchRootState(normalized);
  }, [setWatchRootState]);

  return { watchRoot, setWatchRoot, defaultWatchRoot: DEFAULT_WATCH_ROOT };
}
