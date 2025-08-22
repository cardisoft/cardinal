import { useState, useEffect, useRef, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen, once } from '@tauri-apps/api/event';
import { LRUCache } from '../utils/LRUCache';
import { CACHE_SIZE, SEARCH_DEBOUNCE_MS } from '../constants';

export function useAppState() {
  const [results, setResults] = useState([]);
  const [isInitialized, setIsInitialized] = useState(false);
  const [scannedFiles, setScannedFiles] = useState(0);
  const [processedEvents, setProcessedEvents] = useState(0);

  useEffect(() => {
    listen('status_bar_update', (event) => {
      const { scanned_files, processed_events } = event.payload;
      setScannedFiles(scanned_files);
      setProcessedEvents(processed_events);
    });
    once('init_completed', () => setIsInitialized(true));
  }, []);

  return {
    results,
    setResults,
    isInitialized,
    scannedFiles,
    processedEvents
  };
}

export function useSearch(setResults, lruCache) {
  const debounceTimerRef = useRef(null);

  const handleSearch = useCallback(async (query) => {
    let searchResults = [];
    if (query.trim() !== '') {
      searchResults = await invoke("search", { query });
    }
    lruCache.current.clear();
    setResults(searchResults);
  }, [setResults, lruCache]);

  const onQueryChange = useCallback((e) => {
    const currentQuery = e.target.value;
    clearTimeout(debounceTimerRef.current);
    debounceTimerRef.current = setTimeout(() => {
      handleSearch(currentQuery);
    }, SEARCH_DEBOUNCE_MS);
  }, [handleSearch]);

  return { onQueryChange };
}

export function useVirtualizedList(results) {
  const lruCache = useRef(new LRUCache(CACHE_SIZE));
  const infiniteLoaderRef = useRef(null);

  useEffect(() => {
    if (infiniteLoaderRef.current) {
      infiniteLoaderRef.current.resetLoadMoreRowsCache(true);
    }
  }, [results]);

  const isCellLoaded = useCallback(({ rowIndex }) => 
    lruCache.current.has(rowIndex), []);

  const loadMoreRows = useCallback(async ({ startIndex, stopIndex }) => {
    const rows = results.slice(startIndex, stopIndex + 1);
    const searchResults = await invoke("get_nodes_info", { results: rows });
    for (let i = startIndex; i <= stopIndex; i++) {
      lruCache.current.put(i, searchResults[i - startIndex]);
    }
  }, [results]);

  return {
    lruCache,
    infiniteLoaderRef,
    isCellLoaded,
    loadMoreRows
  };
}

// Re-export other hooks
export { useColumnResize } from './useColumnResize';
export { useContextMenu } from './useContextMenu';
export { useHeaderContextMenu } from './useHeaderContextMenu';
