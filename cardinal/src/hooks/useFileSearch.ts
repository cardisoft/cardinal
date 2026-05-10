import { useReducer, useRef, useCallback, useEffect, useState } from 'react';
import type { MutableRefObject } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { SEARCH_DEBOUNCE_MS } from '../constants';
import {
  SearchStatusCode,
  type AppLifecycleStatus,
  type SearchResponsePayload,
} from '../types/ipc';
import type { SlabIndex } from '../types/slab';

type SearchError = string | Error | null;

type SearchState = {
  results: SlabIndex[];
  resultsVersion: number;
  scannedFiles: number;
  processedEvents: number;
  rescanErrors: number;
  currentQuery: string;
  currentDirectoryQuery: string;
  highlightTerms: string[];
  showLoadingUI: boolean;
  initialFetchCompleted: boolean;
  durationMs: number | null;
  resultCount: number;
  searchError: SearchError;
  lifecycleState: AppLifecycleStatus;
};

type SearchParams = {
  query: string;
  directoryQuery: string;
  directoryScopeOpen: boolean;
  caseSensitive: boolean;
};

export const DIRECTORY_SCOPE_OPEN_STORAGE_KEY = 'cardinal.search.directoryScopeOpen';

type QueueSearchOptions = {
  immediate?: boolean;
  onSearchCommitted?: () => void;
};

type SearchAction =
  | {
      type: 'STATUS_UPDATE';
      payload: { scannedFiles: number; processedEvents: number; rescanErrors: number };
    }
  | { type: 'SEARCH_REQUEST'; payload: { immediate: boolean } }
  | { type: 'SEARCH_LOADING_DELAY' }
  | {
      type: 'SEARCH_SUCCESS';
      payload: {
        results: SlabIndex[];
        query: string;
        directoryQuery: string;
        duration: number;
        count: number;
        highlightTerms: string[];
      };
    }
  | {
      type: 'SEARCH_FAILURE';
      payload: {
        error: SearchError;
        duration: number;
      };
    }
  | { type: 'SEARCH_CANCELLED' }
  | { type: 'SET_LIFECYCLE_STATE'; payload: { status: AppLifecycleStatus } };

const initialSearchState: SearchState = {
  results: [],
  resultsVersion: 0,
  scannedFiles: 0,
  processedEvents: 0,
  rescanErrors: 0,
  currentQuery: '',
  currentDirectoryQuery: '',
  highlightTerms: [],
  showLoadingUI: false,
  initialFetchCompleted: false,
  durationMs: null,
  resultCount: 0,
  searchError: null,
  lifecycleState: 'Initializing',
};

const initialSearchParams: SearchParams = {
  query: '',
  directoryQuery: '',
  directoryScopeOpen: false,
  caseSensitive: false,
};

const readStoredDirectoryScopeOpen = (): boolean => {
  if (typeof window === 'undefined') {
    return initialSearchParams.directoryScopeOpen;
  }
  try {
    return window.localStorage.getItem(DIRECTORY_SCOPE_OPEN_STORAGE_KEY) === 'true';
  } catch {
    return initialSearchParams.directoryScopeOpen;
  }
};

const persistDirectoryScopeOpen = (open: boolean): void => {
  if (typeof window === 'undefined') {
    return;
  }
  try {
    window.localStorage.setItem(DIRECTORY_SCOPE_OPEN_STORAGE_KEY, open ? 'true' : 'false');
  } catch {
    // Ignore storage failures.
  }
};

const cancelTimer = (timerRef: MutableRefObject<ReturnType<typeof setTimeout> | null>) => {
  if (timerRef.current) {
    clearTimeout(timerRef.current);
    timerRef.current = null;
  }
};

// Keep reducer pure and colocated so useReducer stays predictable.
function reducer(state: SearchState, action: SearchAction): SearchState {
  switch (action.type) {
    case 'STATUS_UPDATE':
      return {
        ...state,
        scannedFiles: action.payload.scannedFiles,
        processedEvents: action.payload.processedEvents,
        rescanErrors: action.payload.rescanErrors,
      };
    case 'SEARCH_REQUEST':
      return {
        ...state,
        searchError: null,
        showLoadingUI: action.payload.immediate ? true : state.showLoadingUI,
      };
    case 'SEARCH_LOADING_DELAY':
      return {
        ...state,
        showLoadingUI: true,
      };
    case 'SEARCH_SUCCESS':
      return {
        ...state,
        results: action.payload.results,
        resultsVersion: state.resultsVersion + 1,
        currentQuery: action.payload.query,
        currentDirectoryQuery: action.payload.directoryQuery,
        highlightTerms: action.payload.highlightTerms,
        showLoadingUI: false,
        initialFetchCompleted: true,
        durationMs: action.payload.duration,
        resultCount: action.payload.count,
        searchError: null,
      };
    case 'SEARCH_FAILURE':
      return {
        ...state,
        showLoadingUI: false,
        searchError: action.payload.error,
        initialFetchCompleted: true,
        durationMs: action.payload.duration,
        resultCount: 0,
        highlightTerms: [],
      };
    case 'SEARCH_CANCELLED':
      return {
        ...state,
        showLoadingUI: false,
        initialFetchCompleted: true,
      };
    case 'SET_LIFECYCLE_STATE':
      return {
        ...state,
        lifecycleState: action.payload.status,
      };
    default:
      return state;
  }
}

const searchParamsReducer = (prev: SearchParams, patch: Partial<SearchParams>): SearchParams => {
  return { ...prev, ...patch };
};

const searchParamOrNull = (value: string): string | null => (value.length > 0 ? value : null);

const activeDirectoryQuery = ({ directoryQuery, directoryScopeOpen }: SearchParams): string =>
  directoryScopeOpen ? directoryQuery : '';

type UseFileSearchResult = {
  state: SearchState;
  searchParams: SearchParams;
  updateSearchParams: (patch: Partial<SearchParams>) => void;
  queueSearch: (query: string, options?: QueueSearchOptions) => void;
  queueDirectorySearch: (directoryQuery: string, options?: QueueSearchOptions) => void;
  queueDirectoryScopeOpen: (directoryScopeOpen: boolean) => void;
  handleStatusUpdate: (scannedFiles: number, processedEvents: number, rescanErrors: number) => void;
  setLifecycleState: (status: AppLifecycleStatus) => void;
  requestRescan: () => Promise<void>;
};

export function useFileSearch(): UseFileSearchResult {
  const [initialSearchParamsForHook] = useState<SearchParams>(() => ({
    ...initialSearchParams,
    directoryScopeOpen: readStoredDirectoryScopeOpen(),
  }));
  const [state, dispatch] = useReducer(reducer, initialSearchState);
  const latestSearchRef = useRef<SearchParams>(initialSearchParamsForHook);
  // `search-cancellation` maintains an atomic counter
  // and will auto-increment for each search request
  // so this only serves as a defence-in-depth
  const searchVersionRef = useRef(0);
  const hasInitialSearchRunRef = useRef(false);
  const debounceTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const loadingDelayTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const [searchParams, patchSearchParams] = useReducer(
    searchParamsReducer,
    initialSearchParamsForHook,
  );

  const updateSearchParams = useCallback((patch: Partial<SearchParams>) => {
    latestSearchRef.current = { ...latestSearchRef.current, ...patch };
    if (patch.directoryScopeOpen !== undefined) {
      persistDirectoryScopeOpen(patch.directoryScopeOpen);
    }
    patchSearchParams(patch);
  }, []);

  const handleStatusUpdate = useCallback(
    (scannedFiles: number, processedEvents: number, rescanErrors: number) => {
      dispatch({
        type: 'STATUS_UPDATE',
        payload: { scannedFiles, processedEvents, rescanErrors },
      });
    },
    [],
  );

  const setLifecycleState = useCallback((status: AppLifecycleStatus) => {
    dispatch({ type: 'SET_LIFECYCLE_STATE', payload: { status } });
  }, []);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const status = await invoke<AppLifecycleStatus>('get_app_status');
        if (!cancelled) {
          setLifecycleState(status);
        }
      } catch (error) {
        console.error('Failed to fetch app lifecycle status:', error);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [setLifecycleState]);

  const cancelPendingSearches = useCallback(() => {
    cancelTimer(debounceTimerRef);
    cancelTimer(loadingDelayTimerRef);
  }, []);

  const handleSearch = useCallback(async (overrides: Partial<SearchParams> = {}) => {
    const nextSearch = { ...latestSearchRef.current, ...overrides };
    latestSearchRef.current = nextSearch;
    // the backend already has search version cancellation
    // but we keep the check at frontend to make sure that
    // the UI always reflects the latest request
    const requestVersion = searchVersionRef.current + 1;
    searchVersionRef.current = requestVersion;

    const { query, caseSensitive } = nextSearch;
    const directoryQuery = activeDirectoryQuery(nextSearch);
    const startTs = performance.now();
    const isInitial = !hasInitialSearchRunRef.current;

    dispatch({ type: 'SEARCH_REQUEST', payload: { immediate: isInitial } });

    if (!isInitial) {
      cancelTimer(loadingDelayTimerRef);
      loadingDelayTimerRef.current = setTimeout(() => {
        dispatch({ type: 'SEARCH_LOADING_DELAY' });
        loadingDelayTimerRef.current = null;
      }, 150);
    }

    try {
      const rawResults = await invoke<SearchResponsePayload>('search', {
        query: searchParamOrNull(query),
        directoryQuery: searchParamOrNull(directoryQuery),
        options: {
          caseInsensitive: !caseSensitive,
        },
      });

      if (searchVersionRef.current !== requestVersion) {
        return;
      }

      if (rawResults.statusCode === SearchStatusCode.CANCELLED) {
        cancelTimer(loadingDelayTimerRef);
        dispatch({ type: 'SEARCH_CANCELLED' });
        return;
      }

      const searchResults = rawResults.results as SlabIndex[];
      const highlightTerms = Array.isArray(rawResults.highlights)
        ? rawResults.highlights.filter((term): term is string => typeof term === 'string')
        : [];

      cancelTimer(loadingDelayTimerRef);

      const endTs = performance.now();
      const duration = endTs - startTs;

      dispatch({
        type: 'SEARCH_SUCCESS',
        payload: {
          results: searchResults,
          query,
          directoryQuery,
          duration,
          count: searchResults.length,
          highlightTerms,
        },
      });
    } catch (error) {
      console.error('Search failed:', error);

      if (searchVersionRef.current !== requestVersion) {
        return;
      }

      cancelTimer(loadingDelayTimerRef);

      const endTs = performance.now();
      const duration = endTs - startTs;

      const normalisedError =
        error instanceof Error ? error : error ? String(error) : 'An unknown error occurred.';

      dispatch({
        type: 'SEARCH_FAILURE',
        payload: {
          error: normalisedError,
          duration,
        },
      });
    } finally {
      hasInitialSearchRunRef.current = true;
    }
  }, []);

  const queueSearchParams = useCallback(
    (patch: Partial<SearchParams>, options?: QueueSearchOptions) => {
      updateSearchParams(patch);
      cancelPendingSearches();
      if (options?.immediate) {
        options.onSearchCommitted?.();
        void handleSearch(patch);
        return;
      }

      debounceTimerRef.current = setTimeout(() => {
        options?.onSearchCommitted?.();
        handleSearch(patch);
      }, SEARCH_DEBOUNCE_MS);
    },
    [cancelPendingSearches, handleSearch, updateSearchParams],
  );

  const queueSearch = useCallback(
    (query: string, options?: QueueSearchOptions) => {
      queueSearchParams({ query }, options);
    },
    [queueSearchParams],
  );

  const queueDirectorySearch = useCallback(
    (directoryQuery: string, options?: QueueSearchOptions) => {
      queueSearchParams({ directoryQuery }, options);
    },
    [queueSearchParams],
  );

  const queueDirectoryScopeOpen = useCallback(
    (directoryScopeOpen: boolean) => {
      queueSearchParams({ directoryScopeOpen }, { immediate: true });
    },
    [queueSearchParams],
  );

  useEffect(() => cancelPendingSearches, [cancelPendingSearches]);

  useEffect(() => {
    if (!hasInitialSearchRunRef.current) {
      void handleSearch({ query: '' });
      return;
    }

    const nextSearch = latestSearchRef.current;
    if (!nextSearch.query && !activeDirectoryQuery(nextSearch)) {
      return;
    }

    void handleSearch();
  }, [handleSearch, searchParams.caseSensitive]);

  const requestRescan = useCallback(async () => {
    await invoke('trigger_rescan');
  }, []);

  return {
    state,
    searchParams,
    updateSearchParams,
    queueSearch,
    queueDirectorySearch,
    queueDirectoryScopeOpen,
    handleStatusUpdate,
    setLifecycleState,
    requestRescan,
  };
}
