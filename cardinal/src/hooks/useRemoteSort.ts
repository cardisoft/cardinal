import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { SortKey, SortState } from '../types/sort';
import type { SlabIndex } from '../types/slab';
import { toSlabIndexArray } from '../types/slab';

const SORT_THRESHOLD_STORAGE_KEY = 'cardinal.sortThreshold';
const DEFAULT_SORTABLE_RESULT_THRESHOLD = 20000;

const clampSortThreshold = (value: number): number => {
  if (!Number.isFinite(value)) {
    return DEFAULT_SORTABLE_RESULT_THRESHOLD;
  }
  const rounded = Math.round(value);
  return Math.max(1, rounded);
};

const readStoredSortThreshold = (): number => {
  if (typeof window === 'undefined') {
    return DEFAULT_SORTABLE_RESULT_THRESHOLD;
  }
  const stored = window.localStorage.getItem(SORT_THRESHOLD_STORAGE_KEY);
  if (stored == null) {
    return DEFAULT_SORTABLE_RESULT_THRESHOLD;
  }
  const parsed = Number.parseInt(stored, 10);
  if (Number.isNaN(parsed)) {
    return DEFAULT_SORTABLE_RESULT_THRESHOLD;
  }
  return clampSortThreshold(parsed);
};

const persistSortThreshold = (value: number): void => {
  if (typeof window === 'undefined') {
    return;
  }
  try {
    window.localStorage.setItem(SORT_THRESHOLD_STORAGE_KEY, String(value));
  } catch {
    // Ignore storage failures.
  }
};

export type RemoteSortControls = {
  sortState: SortState;
  setSortState: (next: SortState) => void;
  sortedResults: SlabIndex[];
  displayedResults: SlabIndex[];
  sortThreshold: number;
  setSortThreshold: (value: number) => void;
  canSort: boolean;
  isSorting: boolean;
  sortLimitLabel: string;
  sortDisabledTooltip: string | null;
  sortButtonsDisabled: boolean;
  handleSortToggle: (key: SortKey) => void;
};

export const useRemoteSort = (
  results: SlabIndex[],
  locale: string,
  formatDisabledTooltip: (limit: string) => string | null,
): RemoteSortControls => {
  const [sortState, setSortState] = useState<SortState>(null);
  const [sortedResults, setSortedResults] = useState<SlabIndex[]>([]);
  const [sortThreshold, setSortThresholdState] = useState<number>(() => readStoredSortThreshold());
  const [isSorting, setIsSorting] = useState(false);
  const sortRequestRef = useRef(0);

  const canSort = results.length > 0 && results.length <= sortThreshold;
  const shouldUseSortedResults = Boolean(sortState && canSort);
  const displayedResults = shouldUseSortedResults ? sortedResults : results;

  const setSortThreshold = useCallback((value: number) => {
    const normalized = clampSortThreshold(value);
    setSortThresholdState(normalized);
    persistSortThreshold(normalized);
  }, []);

  const handleSortToggle = useCallback(
    (nextKey: SortKey) => {
      if (!canSort) {
        return;
      }
      setSortState((prev) => {
        if (!prev || prev.key !== nextKey) {
          return { key: nextKey, direction: 'asc' };
        }
        if (prev.direction === 'asc') {
          return { key: nextKey, direction: 'desc' };
        }
        return null;
      });
    },
    [canSort],
  );

  useEffect(() => {
    if (!canSort && sortState) {
      setSortState(null);
    }
  }, [canSort, sortState]);

  useEffect(() => {
    const requestId = sortRequestRef.current + 1;
    sortRequestRef.current = requestId;

    if (!sortState || !canSort || results.length === 0) {
      setIsSorting(false);
      setSortedResults(results);
      return;
    }

    setIsSorting(true);

    void (async () => {
      try {
        const ordered = await invoke<number[]>('get_sorted_view', {
          results,
          sort: sortState,
        });
        if (sortRequestRef.current === requestId) {
          setSortedResults(toSlabIndexArray(Array.isArray(ordered) ? ordered : []));
        }
      } catch (error) {
        console.error('Failed to sort results', error);
        if (sortRequestRef.current === requestId) {
          setSortedResults(results);
        }
      } finally {
        if (sortRequestRef.current === requestId) {
          setIsSorting(false);
        }
      }
    })();
  }, [results, sortState, canSort]);

  const sortLimitLabel = useMemo(
    () => new Intl.NumberFormat(locale).format(sortThreshold),
    [locale, sortThreshold],
  );
  const sortDisabledTooltip = canSort ? null : formatDisabledTooltip(sortLimitLabel);
  const sortButtonsDisabled = !canSort || isSorting;

  return {
    sortState,
    setSortState,
    sortedResults,
    displayedResults,
    sortThreshold,
    setSortThreshold,
    canSort,
    isSorting,
    sortLimitLabel,
    sortDisabledTooltip,
    sortButtonsDisabled,
    handleSortToggle,
  };
};
