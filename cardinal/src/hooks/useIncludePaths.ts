import { useCallback } from 'react';
import { useStoredState } from './useStoredState';

const STORAGE_KEY = 'cardinal.includePaths';
// Include paths are opt-in: by default no path is rescued from the ignore list.
// Users add specific subpaths here when they want to override an ignored
// ancestor (e.g. ignore `/Volumes` broadly but keep `/Volumes/media` indexed).
const DEFAULT_INCLUDE_PATHS: string[] = [];

const cleanPaths = (next: string[]): string[] =>
  next.map((item) => item.trim()).filter((item) => item.length > 0);

export function useIncludePaths() {
  const [includePaths, setIncludePathsState] = useStoredState<string[]>({
    key: STORAGE_KEY,
    defaultValue: DEFAULT_INCLUDE_PATHS,
    read: (raw) => {
      const parsed = JSON.parse(raw);
      if (!Array.isArray(parsed)) return null;
      return cleanPaths(parsed.filter((item): item is string => typeof item === 'string'));
    },
    write: (value) => JSON.stringify(value),
    readErrorMessage: 'Unable to read saved include paths',
    writeErrorMessage: 'Unable to persist include paths',
  });

  const setIncludePaths = useCallback(
    (next: string[]) => {
      const cleaned = cleanPaths(next);
      setIncludePathsState(cleaned);
    },
    [setIncludePathsState],
  );

  return { includePaths, setIncludePaths, defaultIncludePaths: DEFAULT_INCLUDE_PATHS };
}
