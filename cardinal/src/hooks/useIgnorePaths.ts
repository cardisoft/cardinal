import { useCallback } from 'react';
import { useStoredState } from './useStoredState';

const STORAGE_KEY = 'cardinal.ignorePaths';
const DEFAULT_IGNORE_PATHS = ['/Volumes', '~/Library/CloudStorage'];

const cleanPaths = (next: string[]): string[] =>
  next.map((item) => item.trim()).filter((item) => item.length > 0);

export function useIgnorePaths() {
  const [ignorePaths, setIgnorePathsState] = useStoredState<string[]>({
    key: STORAGE_KEY,
    defaultValue: DEFAULT_IGNORE_PATHS,
    read: (raw) => {
      const parsed = JSON.parse(raw);
      if (!Array.isArray(parsed)) return null;
      return cleanPaths(parsed.filter((item): item is string => typeof item === 'string'));
    },
    write: (value) => JSON.stringify(value),
    readErrorMessage: 'Unable to read saved ignore paths',
    writeErrorMessage: 'Unable to persist default ignore paths',
  });

  const setIgnorePaths = useCallback(
    (next: string[]) => {
      const cleaned = cleanPaths(next);
      setIgnorePathsState(cleaned);
    },
    [setIgnorePathsState],
  );

  return { ignorePaths, setIgnorePaths, defaultIgnorePaths: DEFAULT_IGNORE_PATHS };
}
