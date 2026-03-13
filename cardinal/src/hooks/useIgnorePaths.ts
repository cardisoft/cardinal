import { useCallback } from 'react';
import { useStoredState } from './useStoredState';

const STORAGE_KEY = 'cardinal.ignorePaths';
// Ignore /Volumes, ~/Library/CloudStorage, and ~/Library/Caches by default:
// - `/Volumes` contains external drives and network mounts that can be large or slow to traverse.
// - `~/Library/CloudStorage` contains cloud provider stubs (for example OneDrive) that may trigger network I/O on first traversal.
// - `~/Library/Caches` is high-churn, disposable data that adds indexing cost with little search value.
const DEFAULT_IGNORE_PATHS = ['/Volumes', '~/Library/CloudStorage', '~/Library/Caches'];

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
