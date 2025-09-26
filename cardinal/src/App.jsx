import { useRef, useCallback, useEffect, useReducer, useState } from 'react';
import './App.css';
import { ContextMenu } from './components/ContextMenu';
import { ColumnHeader } from './components/ColumnHeader';
import { FileRow } from './components/FileRow';
import StatusBar from './components/StatusBar';
import { useColumnResize } from './hooks/useColumnResize';
import { useContextMenu } from './hooks/useContextMenu';
import { ROW_HEIGHT, OVERSCAN_ROW_COUNT, SEARCH_DEBOUNCE_MS } from './constants';
import { VirtualList } from './components/VirtualList';
import { StateDisplay } from './components/StateDisplay';
import { usePreventRefresh } from './hooks/usePreventRefresh';
import { invoke } from '@tauri-apps/api/core';
import { listen, once } from '@tauri-apps/api/event';

const initialState = {
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

function reducer(state, action) {
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

function App() {
  usePreventRefresh();
  const [state, dispatch] = useReducer(reducer, initialState);
  const [useRegex, setUseRegex] = useState(false);
  const [caseSensitive, setCaseSensitive] = useState(false);
  const {
    results,
    isInitialized,
    scannedFiles,
    processedEvents,
    currentQuery,
    showLoadingUI,
    initialFetchCompleted,
    durationMs,
    resultCount,
    searchError
  } = state;
  const { colWidths, onResizeStart, autoFitColumns } = useColumnResize();
  const {
    menu, showContextMenu, showHeaderContextMenu, closeMenu, getMenuItems
  } = useContextMenu(autoFitColumns);

  const headerRef = useRef(null);
  const virtualListRef = useRef(null);
  const prevQueryRef = useRef('');
  const prevResultsLenRef = useRef(0);
  const debounceTimerRef = useRef(null);
  const loadingDelayTimerRef = useRef(null);
  const hasInitialSearchRunRef = useRef(false);
  const latestQueryRef = useRef('');

  useEffect(() => {
    let isMounted = true;
    let unlistenStatus;
    let unlistenInit;

    const setupListeners = async () => {
      unlistenStatus = await listen('status_bar_update', (event) => {
        if (!isMounted) return;
        const { scanned_files, processed_events } = event.payload;
        dispatch({
          type: 'STATUS_UPDATE',
          payload: {
            scannedFiles: scanned_files,
            processedEvents: processed_events
          }
        });
      });

      unlistenInit = await once('init_completed', () => {
        if (!isMounted) return;
        dispatch({ type: 'INIT_COMPLETED' });
      });
    };

    setupListeners();

    return () => {
      isMounted = false;
      if (typeof unlistenStatus === 'function') {
        unlistenStatus();
      }
      if (typeof unlistenInit === 'function') {
        unlistenInit();
      }
    };
  }, []);

  const handleSearch = useCallback(async (query) => {
    latestQueryRef.current = query;
    const startTs = performance.now();
    const isInitial = !hasInitialSearchRunRef.current;
    const trimmedQuery = query.trim();

    dispatch({ type: 'SEARCH_REQUEST', payload: { immediate: isInitial } });

    if (!isInitial) {
      if (loadingDelayTimerRef.current) {
        clearTimeout(loadingDelayTimerRef.current);
      }
      loadingDelayTimerRef.current = setTimeout(() => {
        dispatch({ type: 'SEARCH_LOADING_DELAY' });
        loadingDelayTimerRef.current = null;
      }, 150);
    }

    try {
      const searchResults = await invoke('search', {
        query,
        options: {
          useRegex,
          caseInsensitive: !caseSensitive,
        }
      });

      if (loadingDelayTimerRef.current) {
        clearTimeout(loadingDelayTimerRef.current);
        loadingDelayTimerRef.current = null;
      }

      const endTs = performance.now();
      const duration = endTs - startTs;

      dispatch({
        type: 'SEARCH_SUCCESS',
        payload: {
          results: searchResults,
          query: trimmedQuery,
          duration,
          count: Array.isArray(searchResults) ? searchResults.length : 0
        }
      });
    } catch (error) {
      console.error('Search failed:', error);

      if (loadingDelayTimerRef.current) {
        clearTimeout(loadingDelayTimerRef.current);
        loadingDelayTimerRef.current = null;
      }

      const endTs = performance.now();
      const duration = endTs - startTs;

      dispatch({
        type: 'SEARCH_FAILURE',
        payload: {
          error: error || 'An unknown error occurred.',
          duration
        }
      });
    } finally {
      hasInitialSearchRunRef.current = true;
    }
  }, [caseSensitive, dispatch, useRegex]);

  const onQueryChange = useCallback((e) => {
    const inputValue = e.target.value;
    latestQueryRef.current = inputValue;
    if (debounceTimerRef.current) {
      clearTimeout(debounceTimerRef.current);
    }
    debounceTimerRef.current = setTimeout(() => {
      handleSearch(inputValue);
    }, SEARCH_DEBOUNCE_MS);
  }, [handleSearch]);

  const onToggleRegex = useCallback((event) => {
    setUseRegex(event.target.checked);
  }, []);

  const onToggleCaseSensitive = useCallback((event) => {
    setCaseSensitive(event.target.checked);
  }, []);

  useEffect(() => () => {
    if (debounceTimerRef.current) {
      clearTimeout(debounceTimerRef.current);
    }
    if (loadingDelayTimerRef.current) {
      clearTimeout(loadingDelayTimerRef.current);
    }
  }, []);

  useEffect(() => {
    if (!hasInitialSearchRunRef.current) {
      handleSearch('');
    }
  }, [handleSearch]);

  useEffect(() => {
    if (!hasInitialSearchRunRef.current) {
      return;
    }
    handleSearch(latestQueryRef.current);
  }, [caseSensitive, handleSearch, useRegex]);

  // 优化的搜索结果处理逻辑（保持使用 useRef，但简化其他逻辑）
  useEffect(() => {
    if (results.length === 0) return;
    const isNewQuery = prevQueryRef.current !== currentQuery;
    const wasEmpty = prevResultsLenRef.current === 0;

    if (isNewQuery && virtualListRef.current) {
      virtualListRef.current.scrollToTop();
    }

    if ((isNewQuery || wasEmpty) && virtualListRef.current?.ensureRangeLoaded) {
      const preloadCount = Math.min(30, results.length);
      virtualListRef.current.ensureRangeLoaded(0, preloadCount - 1);
    }
    prevQueryRef.current = currentQuery;
    prevResultsLenRef.current = results.length;
  }, [results, currentQuery]);

  // 滚动同步处理 - 单向同步版本（Grid -> Header）
  const handleHorizontalSync = useCallback((scrollLeft) => {
    if (headerRef.current) headerRef.current.scrollLeft = scrollLeft;
  }, []);

  // 单元格渲染
  const renderRow = (rowIndex, item, rowStyle) => (
    <FileRow
      key={rowIndex}
      item={item}
      rowIndex={rowIndex}
      style={{ ...rowStyle, width: 'var(--columns-total)' }}
      onContextMenu={showContextMenu}
      searchQuery={currentQuery}
      caseInsensitive={!caseSensitive}
    />
  );

  const getDisplayState = () => {
    if (!initialFetchCompleted) return 'loading';
    if (searchError) return 'error';
    if (showLoadingUI && results.length === 0) return 'loading';
    if (results.length === 0) return 'empty';
    return 'results';
  };

  const displayState = getDisplayState();

  return (
    <main className="container">
      <div className="search-container">
        <div className="search-bar">
          <input
            id="search-input"
            onChange={onQueryChange}
            placeholder="Search for files and folders..."
            spellCheck={false}
            autoCorrect="off"
            autoComplete="off"
            autoCapitalize="off"
          />
          <div className="search-options">
            <label
              className="search-option"
              title="Toggle case-sensitive matching"
            >
              <input
                type="checkbox"
                checked={caseSensitive}
                onChange={onToggleCaseSensitive}
                aria-label="Toggle case-sensitive matching"
              />
              <span
                className="search-option__display"
                aria-hidden="true"
              >
                Aa
              </span>
              <span className="sr-only">Toggle case-sensitive matching</span>
            </label>
            <label
              className="search-option"
              title="Enable regular expression search"
            >
              <input
                type="checkbox"
                checked={useRegex}
                onChange={onToggleRegex}
                aria-label="Enable regular expression search"
              />
              <span
                className="search-option__display"
                aria-hidden="true"
              >
                .*
              </span>
              <span className="sr-only">Enable regular expression search</span>
            </label>
          </div>
        </div>
      </div>
      <div
        className="results-container"
        style={{
          ['--w-filename']: `${colWidths.filename}px`,
          ['--w-path']: `${colWidths.path}px`,
          ['--w-size']: `${colWidths.size}px`,
          ['--w-modified']: `${colWidths.modified}px`,
          ['--w-created']: `${colWidths.created}px`,
        }}
      >
        <div className="scroll-area">
          <ColumnHeader
            ref={headerRef}
            onResizeStart={onResizeStart}
            onContextMenu={showHeaderContextMenu}
          />
          <div className="flex-fill">
            {displayState !== 'results' ? (
              <StateDisplay state={displayState} message={searchError} query={currentQuery} />
            ) : (
              <VirtualList
                ref={virtualListRef}
                results={results}
                rowHeight={ROW_HEIGHT}
                overscan={OVERSCAN_ROW_COUNT}
                renderRow={renderRow}
                onScrollSync={handleHorizontalSync}
                className="virtual-list"
              />
            )}
          </div>
        </div>
      </div>
      {menu.visible && (
        <ContextMenu
          x={menu.x}
          y={menu.y}
          items={getMenuItems()}
          onClose={closeMenu}
        />
      )}
      <StatusBar
        scannedFiles={scannedFiles}
        processedEvents={processedEvents}
        isReady={isInitialized}
        searchDurationMs={durationMs}
        resultCount={resultCount}
      />
    </main>
  );
}

export default App;
