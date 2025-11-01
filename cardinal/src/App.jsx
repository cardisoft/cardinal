import { useRef, useCallback, useEffect, useReducer, useState, useMemo } from 'react';
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
import FSEventsPanel from './components/FSEventsPanel';
import { usePreventRefresh } from './hooks/usePreventRefresh';
import { invoke } from '@tauri-apps/api/core';
import { listen, once } from '@tauri-apps/api/event';

const cancelTimer = (timerRef) => {
  if (timerRef.current) {
    clearTimeout(timerRef.current);
    timerRef.current = null;
  }
};

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
    searchError,
  } = state;
  const [activeTab, setActiveTab] = useState('files');
  const [recentEvents, setRecentEvents] = useState([]);
  const [eventFilterQuery, setEventFilterQuery] = useState('');
  const { colWidths, onResizeStart, autoFitColumns } = useColumnResize();
  
  // Files context menu
  const { 
    menu: filesMenu, 
    showContextMenu: showFilesContextMenu, 
    showHeaderContextMenu: showFilesHeaderContextMenu, 
    closeMenu: closeFilesMenu, 
    getMenuItems: getFilesMenuItems 
  } = useContextMenu(autoFitColumns);

  // Event columns resize state
  const [eventColWidths, setEventColWidths] = useState(() => {
    const totalWidth = window.innerWidth - 60;
    return {
      name: Math.floor(totalWidth * 0.25),
      path: Math.floor(totalWidth * 0.50),
      time: Math.floor(totalWidth * 0.25),
    };
  });

  const onEventResizeStart = useCallback(
    (e, key) => {
      e.preventDefault();
      e.stopPropagation();

      const startX = e.clientX;
      const startWidth = eventColWidths[key];

      const handleMouseMove = (moveEvent) => {
        const delta = moveEvent.clientX - startX;
        const newWidth = Math.max(80, Math.min(800, startWidth + delta));
        setEventColWidths((prev) => ({ ...prev, [key]: newWidth }));
      };

      const handleMouseUp = () => {
        document.removeEventListener('mousemove', handleMouseMove);
        document.removeEventListener('mouseup', handleMouseUp);
        document.body.style.userSelect = '';
        document.body.style.cursor = '';
      };

      document.addEventListener('mousemove', handleMouseMove);
      document.addEventListener('mouseup', handleMouseUp);
      document.body.style.userSelect = 'none';
      document.body.style.cursor = 'col-resize';
    },
    [eventColWidths],
  );

  const autoFitEventColumns = useCallback(() => {
    const totalWidth = window.innerWidth - 60;
    setEventColWidths({
      name: Math.floor(totalWidth * 0.25),
      path: Math.floor(totalWidth * 0.50),
      time: Math.floor(totalWidth * 0.25),
    });
  }, []);

  // Events context menu
  const { 
    menu: eventsMenu, 
    showContextMenu: showEventsContextMenu, 
    showHeaderContextMenu: showEventsHeaderContextMenu, 
    closeMenu: closeEventsMenu, 
    getMenuItems: getEventsMenuItems 
  } = useContextMenu(autoFitEventColumns);

  // Unified menu interface based on active tab
  const menu = activeTab === 'events' ? eventsMenu : filesMenu;
  const showContextMenu = activeTab === 'events' ? showEventsContextMenu : showFilesContextMenu;
  const showHeaderContextMenu = activeTab === 'events' ? showEventsHeaderContextMenu : showFilesHeaderContextMenu;
  const closeMenu = activeTab === 'events' ? closeEventsMenu : closeFilesMenu;
  const getMenuItems = activeTab === 'events' ? getEventsMenuItems : getFilesMenuItems;

  const headerRef = useRef(null);
  const virtualListRef = useRef(null);
  const isMountedRef = useRef(false);
  const debounceTimerRef = useRef(null);
  const loadingDelayTimerRef = useRef(null);
  const hasInitialSearchRunRef = useRef(false);
  const latestSearchRef = useRef({ query: '', useRegex: false, caseSensitive: false });
  const searchVersionRef = useRef(0);
  const [searchParams, updateSearchParams] = useReducer((prev, patch) => {
    const next = { ...prev, ...patch };
    latestSearchRef.current = next;
    return next;
  }, latestSearchRef.current);
  const { useRegex, caseSensitive } = searchParams;

  useEffect(() => {
    isMountedRef.current = true;
    let unlistenStatus;
    let unlistenInit;

    const setupListeners = async () => {
      unlistenStatus = await listen('status_bar_update', (event) => {
        if (!isMountedRef.current) return;
        const { scanned_files, processed_events } = event.payload;
        dispatch({
          type: 'STATUS_UPDATE',
          payload: {
            scannedFiles: scanned_files,
            processedEvents: processed_events,
          },
        });
      });

      unlistenInit = await once('init_completed', () => {
        if (!isMountedRef.current) return;
        dispatch({ type: 'INIT_COMPLETED' });
      });
    };

    setupListeners();

    return () => {
      isMountedRef.current = false;
      if (typeof unlistenStatus === 'function') {
        unlistenStatus();
      }
      if (typeof unlistenInit === 'function') {
        unlistenInit();
      }
    };
  }, []);

  // Listen for new file system events and maintain them in JS
  useEffect(() => {
    let unlistenEvents;
    const MAX_EVENTS = 10000;

    const setupEventsListener = async () => {
      try {
        unlistenEvents = await listen('fs_events_batch', (event) => {
          if (!isMountedRef.current) return;
          const newEvents = Array.isArray(event?.payload) ? event.payload : [];
          if (newEvents.length === 0) return;

          setRecentEvents((prev) => {
            // Prepend new events, keep only the most recent MAX_EVENTS
            const updated = [...newEvents, ...prev];
            return updated.slice(0, MAX_EVENTS);
          });
        });
      } catch (error) {
        console.error('Failed to listen for file events', error);
      }
    };

    setupEventsListener();

    return () => {
      if (typeof unlistenEvents === 'function') {
        unlistenEvents();
      }
    };
  }, []);

  // Filter events based on eventFilterQuery
  const filteredEvents = useMemo(() => {
    if (!eventFilterQuery.trim()) {
      return recentEvents;
    }

    const query = eventFilterQuery;
    
    // Use regex if enabled
    if (useRegex) {
      try {
        const flags = caseSensitive ? '' : 'i';
        const regex = new RegExp(query, flags);
        return recentEvents.filter((event) => {
          const path = event.path || '';
          const name = path.split('/').pop() || '';
          return regex.test(path) || regex.test(name);
        });
      } catch (error) {
        // Invalid regex, return all events
        return recentEvents;
      }
    }
    
    // Simple string matching
    const searchQuery = caseSensitive ? query : query.toLowerCase();
    return recentEvents.filter((event) => {
      const path = event.path || '';
      const name = path.split('/').pop() || '';
      const testPath = caseSensitive ? path : path.toLowerCase();
      const testName = caseSensitive ? name : name.toLowerCase();
      return (
        testPath.includes(searchQuery) ||
        testName.includes(searchQuery)
      );
    });
  }, [recentEvents, eventFilterQuery, caseSensitive, useRegex]);

  const handleSearch = useCallback(
    async (overrides = {}) => {
      const nextSearch = { ...latestSearchRef.current, ...overrides };
      latestSearchRef.current = nextSearch;
      const requestVersion = searchVersionRef.current + 1;
      searchVersionRef.current = requestVersion;

      const { query, useRegex: nextUseRegex, caseSensitive: nextCaseSensitive } = nextSearch;
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
        const searchResults = await invoke('search', {
          query,
          options: {
            useRegex: nextUseRegex,
            caseInsensitive: !nextCaseSensitive,
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
            results: searchResults,
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

        dispatch({
          type: 'SEARCH_FAILURE',
          payload: {
            error: error || 'An unknown error occurred.',
            duration,
          },
        });
      } finally {
        hasInitialSearchRunRef.current = true;
      }
    },
    [dispatch],
  );

  const onQueryChange = useCallback(
    (e) => {
      const inputValue = e.target.value;
      
      if (activeTab === 'events') {
        // For events tab, just update the filter query (no debounce, instant filter)
        setEventFilterQuery(inputValue);
      } else {
        // For files tab, perform the search with debounce
        updateSearchParams({ query: inputValue });
        cancelTimer(debounceTimerRef);
        debounceTimerRef.current = setTimeout(() => {
          handleSearch({ query: inputValue });
        }, SEARCH_DEBOUNCE_MS);
      }
    },
    [activeTab, handleSearch, updateSearchParams],
  );

  const onToggleRegex = useCallback(
    (event) => {
      const nextValue = event.target.checked;
      updateSearchParams({ useRegex: nextValue });
    },
    [updateSearchParams],
  );

  const onToggleCaseSensitive = useCallback(
    (event) => {
      const nextValue = event.target.checked;
      updateSearchParams({ caseSensitive: nextValue });
    },
    [updateSearchParams],
  );

  useEffect(
    () => () => {
      cancelTimer(debounceTimerRef);
      cancelTimer(loadingDelayTimerRef);
    },
    [],
  );

  useEffect(() => {
    if (!hasInitialSearchRunRef.current) {
      handleSearch({ query: '' });
      return;
    }
    handleSearch();
  }, [caseSensitive, handleSearch, useRegex]);

  // scroll position reset & initial data preload on `results` change
  useEffect(() => {
    const list = virtualListRef.current;
    if (!list) return;

    list.scrollToTop?.();

    if (!results.length || !list.ensureRangeLoaded) {
      return;
    }

    const preloadCount = Math.min(30, results.length);
    list.ensureRangeLoaded(0, preloadCount - 1);
  }, [results]);

  // 滚动同步处理 - 单向同步版本（Grid -> Header）
  const handleHorizontalSync = useCallback((scrollLeft) => {
    if (headerRef.current) headerRef.current.scrollLeft = scrollLeft;
  }, []);

  // 单元格渲染 - 使用 useCallback 避免 VirtualList 不必要的重渲染
  const renderRow = useCallback(
    (rowIndex, item, rowStyle) => (
      <FileRow
        key={rowIndex}
        item={item}
        rowIndex={rowIndex}
        style={{ ...rowStyle, width: 'var(--columns-total)' }}
        onContextMenu={showContextMenu}
        searchQuery={currentQuery}
        caseInsensitive={!caseSensitive}
      />
    ),
    [showContextMenu, currentQuery, caseSensitive],
  );

  const getDisplayState = () => {
    if (!initialFetchCompleted) return 'loading';
    if (showLoadingUI) return 'loading';
    if (searchError) return 'error';
    if (results.length === 0) return 'empty';
    return 'results';
  };

  const displayState = getDisplayState();

  // Handle tab change: clear search input when switching tabs
  const handleTabChange = useCallback((newTab) => {
    setActiveTab(newTab);
    if (newTab === 'events') {
      setEventFilterQuery('');
    } else {
      // Clear the search input for files tab
      const searchInput = document.getElementById('search-input');
      if (searchInput) {
        searchInput.value = '';
      }
      updateSearchParams({ query: '' });
      cancelTimer(debounceTimerRef);
    }
  }, [updateSearchParams]);

  // Get the current search input value based on active tab
  const searchInputValue = activeTab === 'events' ? eventFilterQuery : currentQuery;

  return (
    <main className="container">
      <div className="search-container">
        <div className="search-bar">
          <input
            id="search-input"
            value={searchInputValue}
            onChange={onQueryChange}
            placeholder={
              activeTab === 'files'
                ? 'Search for files and folders...'
                : 'Filter events by path or name...'
            }
            spellCheck={false}
            autoCorrect="off"
            autoComplete="off"
            autoCapitalize="off"
          />
          <div className="search-options">
            <label className="search-option" title="Toggle case-sensitive matching">
              <input
                type="checkbox"
                checked={caseSensitive}
                onChange={onToggleCaseSensitive}
                aria-label="Toggle case-sensitive matching"
              />
              <span className="search-option__display" aria-hidden="true">
                Aa
              </span>
              <span className="sr-only">Toggle case-sensitive matching</span>
            </label>
            <label className="search-option" title="Enable regular expression search">
              <input
                type="checkbox"
                checked={useRegex}
                onChange={onToggleRegex}
                aria-label="Enable regular expression search"
              />
              <span className="search-option__display" aria-hidden="true">
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
          ['--w-event-name']: `${eventColWidths.name}px`,
          ['--w-event-path']: `${eventColWidths.path}px`,
          ['--w-event-time']: `${eventColWidths.time}px`,
          ['--columns-events-total']: `${eventColWidths.name + eventColWidths.path + eventColWidths.time + 2 * 12 + 10}px`,
        }}
      >
        {activeTab === 'events' ? (
          <div className="events-view">
            <FSEventsPanel
              events={filteredEvents}
              onResizeStart={onEventResizeStart}
              onContextMenu={showContextMenu}
              onHeaderContextMenu={showHeaderContextMenu}
              searchQuery={eventFilterQuery}
              caseInsensitive={!caseSensitive}
            />
          </div>
        ) : (
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
        )}
      </div>
      {menu.visible && (
        <ContextMenu x={menu.x} y={menu.y} items={getMenuItems()} onClose={closeMenu} />
      )}
      <StatusBar
        scannedFiles={scannedFiles}
        processedEvents={processedEvents}
        isReady={isInitialized}
        searchDurationMs={durationMs}
        resultCount={resultCount}
        activeTab={activeTab}
        onTabChange={handleTabChange}
      />
    </main>
  );
}

export default App;
