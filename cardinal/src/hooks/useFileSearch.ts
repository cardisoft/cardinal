import { useReducer, useRef, useCallback, useEffect } from 'react';
import type { MutableRefObject } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { SEARCH_DEBOUNCE_MS } from '../constants';
import type { SearchResultItem } from '../components/FileRow';

type SearchError = string | Error | null;

type SearchState = {
  results: SearchResultItem[];
  isInitialized: boolean;
  scannedFiles: number;
  processedEvents: number;
  currentQuery: string;
  showLoadingUI: boolean;
  initialFetchCompleted: boolean;
  durationMs: number | null;
  resultCount: number;
  searchError: SearchError;
};

type SearchParams = {
  query: string;
  useRegex: boolean;
  caseSensitive: boolean;
};

type SearchAction =
  | { type: 'STATUS_UPDATE'; payload: { scannedFiles: number; processedEvents: number } }
  | { type: 'INIT_COMPLETED' }
  | { type: 'SEARCH_REQUEST'; payload: { immediate: boolean } }
  | { type: 'SEARCH_LOADING_DELAY' }
  | {
      type: 'SEARCH_SUCCESS';
      payload: {
        results: SearchResultItem[];
        query: string;
        duration: number;
        count: number;
      };
    }
  | {
      type: 'SEARCH_FAILURE';
      payload: {
        error: SearchError;
        duration: number;
      };
    };

const initialSearchState: SearchState = {
  results: [],
  isInitialized: false,
  scannedFiles: 0,
  processedEvents: 0,
  currentQuery: '',
  showLoadingUI: false,
  initialFetchCompleted: false,
  durationMs: null,
  resultCount: 0,
  searchError: null,
};

const initialSearchParams: SearchParams = {
  query: '',
  useRegex: false,
  caseSensitive: false,
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
      };
    case 'INIT_COMPLETED':
      return { ...state, isInitialized: true };
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
        currentQuery: action.payload.query,
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
      };
    default:
      return state;
  }
}

const searchParamsReducer = (
  prev: SearchParams,
  patch: Partial<SearchParams>,
): SearchParams => {
  const next = { ...prev, ...patch };
  return next;
};

type UseFileSearchResult = {
  state: SearchState;
  searchParams: SearchParams;
  updateSearchParams: (patch: Partial<SearchParams>) => void;
  queueSearch: (query: string) => void;
  handleSearch: (overrides?: Partial<SearchParams>) => Promise<void>;
  resetSearchQuery: () => void;
  cancelPendingSearches: () => void;
  handleStatusUpdate: (scannedFiles: number, processedEvents: number) => void;
  markInitialized: () => void;
};

export function useFileSearch(): UseFileSearchResult {
  const [state, dispatch] = useReducer(reducer, initialSearchState);
  const latestSearchRef = useRef<SearchParams>(initialSearchParams);
  const searchVersionRef = useRef(0);
  const hasInitialSearchRunRef = useRef(false);
  const debounceTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const loadingDelayTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const [searchParams, patchSearchParams] = useReducer(searchParamsReducer, initialSearchParams);

  const updateSearchParams = useCallback((patch: Partial<SearchParams>) => {
    latestSearchRef.current = { ...latestSearchRef.current, ...patch };
    patchSearchParams(patch);
  }, []);

  const handleStatusUpdate = useCallback((scannedFiles: number, processedEvents: number) => {
    dispatch({
      type: 'STATUS_UPDATE',
      payload: { scannedFiles, processedEvents },
    });
  }, []);

  const markInitialized = useCallback(() => {
    dispatch({ type: 'INIT_COMPLETED' });
  }, []);

  const cancelPendingSearches = useCallback(() => {
    cancelTimer(debounceTimerRef);
    cancelTimer(loadingDelayTimerRef);
  }, []);

  const handleSearch = useCallback(
    async (overrides: Partial<SearchParams> = {}) => {
      const nextSearch = { ...latestSearchRef.current, ...overrides };
      latestSearchRef.current = nextSearch;
      const requestVersion = searchVersionRef.current + 1;
      searchVersionRef.current = requestVersion;

      const { query, useRegex, caseSensitive } = nextSearch;
      const startTs = performance.now();
      const isInitial = !hasInitialSearchRunRef.current;
      const trimmedQuery = query.trim();

      dispatch({ type: 'SEARCH_REQUEST', payload: { immediate: isInitial } });

      if (!isInitial) {
        cancelTimer(loadingDelayTimerRef);
        loadingDelayTimerRef.current = setTimeout(() => {
          dispatch({ type: 'SEARCH_LOADING_DELAY' });
          loadingDelayTimerRef.current = null;
        }, 150);
      }

      try {
        const searchResults = await invoke<SearchResultItem[]>('search', {
          query,
          options: {
            useRegex,
            caseInsensitive: !caseSensitive,
          },
        });

        if (searchVersionRef.current !== requestVersion) {
          return;
        }

        cancelTimer(loadingDelayTimerRef);

        const endTs = performance.now();
        const duration = endTs - startTs;

        dispatch({
          type: 'SEARCH_SUCCESS',
          payload: {
            results: Array.isArray(searchResults) ? searchResults : [],
            query: trimmedQuery,
            duration,
            count: Array.isArray(searchResults) ? searchResults.length : 0,
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
    },
    [],
  );

  const queueSearch = useCallback(
    (query: string) => {
      updateSearchParams({ query });
      cancelTimer(debounceTimerRef);
      debounceTimerRef.current = setTimeout(() => {
        handleSearch({ query });
      }, SEARCH_DEBOUNCE_MS);
    },
    [handleSearch, updateSearchParams],
  );

  const resetSearchQuery = useCallback(() => {
    updateSearchParams({ query: '' });
    cancelPendingSearches();
  }, [cancelPendingSearches, updateSearchParams]);

  useEffect(() => cancelPendingSearches, [cancelPendingSearches]);

  useEffect(() => {
    if (!hasInitialSearchRunRef.current) {
      void handleSearch({ query: '' });
      return;
    }

    if (!(latestSearchRef.current.query || '').trim()) {
      return;
    }

    void handleSearch();
  }, [handleSearch, searchParams.caseSensitive, searchParams.useRegex]);

  return {
    state,
    searchParams,
    updateSearchParams,
    queueSearch,
    handleSearch,
    resetSearchQuery,
    cancelPendingSearches,
    handleStatusUpdate,
    markInitialized,
  };
}
