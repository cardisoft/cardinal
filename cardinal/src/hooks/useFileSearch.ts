import { useReducer, useRef, useCallback, useEffect } from 'react';
import type { MutableRefObject } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { SEARCH_DEBOUNCE_MS } from '../constants';
import type { AppLifecycleStatus, SearchResponsePayload } from '../types/ipc';
import type { SlabIndex } from '../types/slab';
import { toSlabIndexArray } from '../types/slab';

type SearchError = string | Error | null;

type SearchState = {
  results: SlabIndex[];
  resultsVersion: number;
  scannedFiles: number;
  processedEvents: number;
  currentQuery: string;
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
  caseSensitive: boolean;
};

type QueueSearchOptions = {
  onSearchCommitted?: (query: string) => void;
};

type SearchAction =
  | { type: 'STATUS_UPDATE'; payload: { scannedFiles: number; processedEvents: number } }
  | { type: 'SEARCH_REQUEST'; payload: { immediate: boolean } }
  | { type: 'SEARCH_LOADING_DELAY' }
  | {
      type: 'SEARCH_SUCCESS';
      payload: {
        results: SlabIndex[];
        query: string;
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
  | { type: 'SET_LIFECYCLE_STATE'; payload: { status: AppLifecycleStatus } };

const initialSearchState: SearchState = {
  results: [],
  resultsVersion: 0,
  scannedFiles: 0,
  processedEvents: 0,
  currentQuery: '',
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
  const next = { ...prev, ...patch };
  return next;
};

type UseFileSearchResult = {
  state: SearchState;
  searchParams: SearchParams;
  updateSearchParams: (patch: Partial<SearchParams>) => void;
  queueSearch: (query: string, options?: QueueSearchOptions) => void;
  handleSearch: (overrides?: Partial<SearchParams>) => Promise<void>;
  resetSearchQuery: () => void;
  cancelPendingSearches: () => void;
  handleStatusUpdate: (scannedFiles: number, processedEvents: number) => void;
  setLifecycleState: (status: AppLifecycleStatus) => void;
  requestRescan: () => Promise<void>;
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
    const requestVersion = searchVersionRef.current + 1;
    searchVersionRef.current = requestVersion;

    const { query, caseSensitive } = nextSearch;
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
        query,
        options: {
          caseInsensitive: !caseSensitive,
        },
        version: requestVersion,
      });

      const slabResults = Array.isArray(rawResults?.results) ? rawResults.results : [];
      const searchResults = toSlabIndexArray(slabResults);
      const highlightTerms = Array.isArray(rawResults?.highlights)
        ? rawResults.highlights.filter((term): term is string => typeof term === 'string')
        : [];

      if (searchVersionRef.current !== requestVersion) {
        return;
      }

      cancelTimer(loadingDelayTimerRef);

      const endTs = performance.now();
      const duration = endTs - startTs;

      dispatch({
        type: 'SEARCH_SUCCESS',
        payload: {
          results: searchResults,
          query,
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

  const queueSearch = useCallback(
    (query: string, options?: QueueSearchOptions) => {
      updateSearchParams({ query });
      cancelTimer(debounceTimerRef);
      debounceTimerRef.current = setTimeout(() => {
        options?.onSearchCommitted?.(query);
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

    if (!latestSearchRef.current.query) {
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
    handleSearch,
    resetSearchQuery,
    cancelPendingSearches,
    handleStatusUpdate,
    setLifecycleState,
    requestRescan,
  };
}
