import { useCallback } from 'react';
import { useStoredState } from './useStoredState';

const STORAGE_KEY = 'cardinal.ignorePaths';
// Ignore generated, disposable, or high-churn macOS paths by default:
// - `/Volumes` can include external drives and network mounts that are large or slow to traverse.
// - `~/Library/CloudStorage` can contain cloud stubs that trigger downloads or network I/O.
// - `~/Library/Biome` stores system suggestions/activity data rather than user-authored files.
// - `~/Library/Caches`, `~/Library/Logs`, and `~/Library/Metadata` are cache/log/metadata stores rather than user-authored files.
// - `/Library/Caches` and `/System/Library/Caches` are transient system caches with low search value.
// - `/private/var` is a broad system runtime area for temp files, caches, logs, and databases.
// - `/private/tmp` is a temporary-file area with very high churn and little search value.
const DEFAULT_IGNORE_PATHS = [
  '/Volumes',
  '~/Library/CloudStorage',
  '~/Library/Biome',
  '~/Library/Caches',
  '~/Library/Logs',
  '~/Library/Metadata',
  '/Library/Caches',
  '/System/Library/Caches',
  '/private/var',
  '/private/tmp',
];

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
