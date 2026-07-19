import { useCallback } from 'react';
import { useStoredState } from './useStoredState';

const STORAGE_KEY = 'cardinal.recentSearches';
const DEFAULT_MAX_ENTRIES = 50;

type UseRecentSearchesOptions = {
  maxEntries?: number;
};

type UseRecentSearchesResult = {
  recentSearches: string[];
  recordSearch: (query: string) => void;
  clearRecentSearches: () => void;
};

const readEntries = (raw: string): string[] | null => {
  try {
    const parsed: unknown = JSON.parse(raw);
    if (!Array.isArray(parsed)) {
      return null;
    }
    return parsed.filter((item): item is string => typeof item === 'string');
  } catch {
    return null;
  }
};

/**
 * Persisted list of committed search queries (newest first), deduplicated and capped.
 * Backs the search-history dropdown so queries survive app restarts.
 */
export function useRecentSearches(options: UseRecentSearchesOptions = {}): UseRecentSearchesResult {
  const maxEntries = options.maxEntries ?? DEFAULT_MAX_ENTRIES;

  const [recentSearches, setRecentSearches] = useStoredState<string[]>({
    key: STORAGE_KEY,
    defaultValue: [],
    read: readEntries,
    write: (value) => JSON.stringify(value),
    normalize: (value) => value.slice(0, maxEntries),
    readErrorMessage: 'Failed to read recent searches from storage',
    writeErrorMessage: 'Failed to persist recent searches to storage',
  });

  const recordSearch = useCallback(
    (query: string) => {
      const trimmed = query.trim();
      if (!trimmed) {
        return;
      }

      const next = [trimmed, ...recentSearches.filter((entry) => entry !== trimmed)];
      setRecentSearches(next);
    },
    [recentSearches, setRecentSearches],
  );

  const clearRecentSearches = useCallback(() => {
    setRecentSearches([]);
  }, [setRecentSearches]);

  return { recentSearches, recordSearch, clearRecentSearches };
}
