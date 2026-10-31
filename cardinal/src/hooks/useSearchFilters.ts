import { useCallback, useEffect, useState } from 'react';
import { useStoredState } from './useStoredState';
import { DEFAULT_FILTER_PRESETS, type SearchFilter } from '../types/searchFilters';

const FILTERS_STORAGE_KEY = 'cardinal.searchFilters.v1';

// Drop the selection key that older builds persisted. The dropdown selection used to be
// restored on launch while the first tab's filter started empty, so the UI claimed a filter
// was active while the search ran unfiltered (the "Folders selected but files shown" bug).
const LEGACY_SELECTED_STORAGE_KEY = 'cardinal.searchFilters.selected.v1';

const createId = (): string => {
  const random = Math.random().toString(36).slice(2, 10);
  return `filter-${Date.now().toString(36)}-${random}`;
};

const readFilters = (raw: string): SearchFilter[] | null => {
  try {
    const parsed = JSON.parse(raw) as unknown;
    if (!Array.isArray(parsed)) {
      return null;
    }
    return parsed
      .filter(
        (entry): entry is SearchFilter =>
          !!entry &&
          typeof (entry as SearchFilter).id === 'string' &&
          typeof (entry as SearchFilter).name === 'string' &&
          typeof (entry as SearchFilter).search === 'string',
      )
      .map((entry) => ({ id: entry.id, name: entry.name, search: entry.search }));
  } catch {
    return null;
  }
};

const writeFilters = (filters: SearchFilter[]): string => JSON.stringify(filters);

const buildDefaultFilters = (): SearchFilter[] =>
  DEFAULT_FILTER_PRESETS.map((preset) => ({
    id: createId(),
    name: preset.name,
    search: preset.search,
  }));

export type SearchFiltersController = {
  filters: SearchFilter[];
  /** Search fragment of the selected preset, or '' when nothing is selected. */
  activeSearch: string;
  selectedId: string | null;
  selectFilter: (id: string | null) => void;
  addFilter: (name: string, search: string) => void;
  updateFilter: (id: string, patch: { name: string; search: string }) => void;
  deleteFilter: (id: string) => void;
  replaceFilters: (filters: SearchFilter[]) => void;
  appendFilters: (filters: { name: string; search: string }[]) => void;
};

/**
 * Configurable search filter dropdown. Presets are seeded on first run and then owned by the
 * user (edited/imported/deleted), so nothing is re-seeded once localStorage has a value.
 */
export function useSearchFilters(): SearchFiltersController {
  const [filters, setFilters] = useStoredState<SearchFilter[]>({
    key: FILTERS_STORAGE_KEY,
    defaultValue: buildDefaultFilters(),
    read: readFilters,
    write: writeFilters,
    normalize: (value) => value,
    readErrorMessage: 'Unable to read saved search filters',
    writeErrorMessage: 'Unable to persist search filters',
  });

  // Selection is per session only. Each search tab already carries its own `filterQuery`,
  // and that is what actually drives the search; persisting a separate "selected preset"
  // across launches let the dropdown disagree with the results.
  const [selectedId, setSelectedId] = useState<string | null>(null);

  useEffect(() => {
    if (typeof window === 'undefined') {
      return;
    }
    try {
      window.localStorage.removeItem(LEGACY_SELECTED_STORAGE_KEY);
    } catch {
      // Ignore storage failures.
    }
  }, []);

  const activeFilter = filters.find((filter) => filter.id === selectedId);

  const selectFilter = useCallback(
    (id: string | null) => {
      setSelectedId(id);
    },
    [setSelectedId],
  );

  const addFilter = useCallback(
    (name: string, search: string) => {
      const trimmedName = name.trim();
      if (!trimmedName) {
        return;
      }
      setFilters((prev) => [...prev, { id: createId(), name: trimmedName, search: search.trim() }]);
    },
    [setFilters],
  );

  const updateFilter = useCallback(
    (id: string, patch: { name: string; search: string }) => {
      const trimmedName = patch.name.trim();
      if (!trimmedName) {
        return;
      }
      setFilters((prev) =>
        prev.map((filter) =>
          filter.id === id ? { ...filter, name: trimmedName, search: patch.search.trim() } : filter,
        ),
      );
    },
    [setFilters],
  );

  const deleteFilter = useCallback(
    (id: string) => {
      setFilters((prev) => prev.filter((filter) => filter.id !== id));
      // Functional update: `selectedId` from this render may be stale when the caller
      // selects and deletes in the same batch.
      setSelectedId((prev) => (prev === id ? null : prev));
    },
    [setFilters, setSelectedId],
  );

  const replaceFilters = useCallback(
    (next: SearchFilter[]) => {
      setFilters(next);
      setSelectedId(null);
    },
    [setFilters, setSelectedId],
  );

  const appendFilters = useCallback(
    (entries: { name: string; search: string }[]) => {
      if (entries.length === 0) {
        return;
      }
      setFilters((prev) => [
        ...prev,
        ...entries.map((entry) => ({
          id: createId(),
          name: entry.name.trim(),
          search: entry.search.trim(),
        })),
      ]);
    },
    [setFilters],
  );

  return {
    filters,
    activeSearch: activeFilter?.search ?? '',
    selectedId,
    selectFilter,
    addFilter,
    updateFilter,
    deleteFilter,
    replaceFilters,
    appendFilters,
  };
}
