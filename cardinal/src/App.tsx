import { useRef, useCallback, useEffect, useState } from 'react';
import type { ChangeEvent, CSSProperties } from 'react';
import './App.css';
import { ContextMenu } from './components/ContextMenu';
import { ColumnHeader } from './components/ColumnHeader';
import { FileRow } from './components/FileRow';
import type { SearchResultItem } from './components/FileRow';
import StatusBar from './components/StatusBar';
import type { StatusTabKey } from './components/StatusBar';
import { useColumnResize } from './hooks/useColumnResize';
import { useContextMenu } from './hooks/useContextMenu';
import { useFileSearch } from './hooks/useFileSearch';
import { useEventColumnWidths } from './hooks/useEventColumnWidths';
import { useRecentFSEvents } from './hooks/useRecentFSEvents';
import { ROW_HEIGHT, OVERSCAN_ROW_COUNT } from './constants';
import { VirtualList } from './components/VirtualList';
import type { VirtualListHandle } from './components/VirtualList';
import { StateDisplay } from './components/StateDisplay';
import FSEventsPanel from './components/FSEventsPanel';
import type { FSEventsPanelHandle } from './components/FSEventsPanel';
import { listen, once } from '@tauri-apps/api/event';
import type { UnlistenFn } from '@tauri-apps/api/event';

type ActiveTab = StatusTabKey;

type StatusUpdatePayload = {
  scanned_files: number;
  processed_events: number;
};


function App() {
  const {
    state,
    searchParams,
    updateSearchParams,
    queueSearch,
    resetSearchQuery,
    cancelPendingSearches,
    handleStatusUpdate,
    markInitialized,
  } = useFileSearch();
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
  const [activeTab, setActiveTab] = useState<ActiveTab>('files');
  const eventsPanelRef = useRef<FSEventsPanelHandle | null>(null);
  const headerRef = useRef<HTMLDivElement | null>(null);
  const virtualListRef = useRef<VirtualListHandle | null>(null);
  const isMountedRef = useRef(false);
  const { colWidths, onResizeStart, autoFitColumns } = useColumnResize();
  const { useRegex, caseSensitive } = searchParams;
  const { eventColWidths, onEventResizeStart, autoFitEventColumns } = useEventColumnWidths();
  const { filteredEvents, eventFilterQuery, setEventFilterQuery } = useRecentFSEvents({
    caseSensitive,
    useRegex,
  });

  const {
    menu: filesMenu,
    showContextMenu: showFilesContextMenu,
    showHeaderContextMenu: showFilesHeaderContextMenu,
    closeMenu: closeFilesMenu,
    getMenuItems: getFilesMenuItems,
  } = useContextMenu(autoFitColumns);

  const {
    menu: eventsMenu,
    showContextMenu: showEventsContextMenu,
    showHeaderContextMenu: showEventsHeaderContextMenu,
    closeMenu: closeEventsMenu,
    getMenuItems: getEventsMenuItems,
  } = useContextMenu(autoFitEventColumns);

  const menu = activeTab === 'events' ? eventsMenu : filesMenu;
  const showContextMenu = activeTab === 'events' ? showEventsContextMenu : showFilesContextMenu;
  const showHeaderContextMenu =
    activeTab === 'events' ? showEventsHeaderContextMenu : showFilesHeaderContextMenu;
  const closeMenu = activeTab === 'events' ? closeEventsMenu : closeFilesMenu;
  const getMenuItems = activeTab === 'events' ? getEventsMenuItems : getFilesMenuItems;

  useEffect(() => {
    isMountedRef.current = true;
    let unlistenStatus: UnlistenFn | undefined;
    let unlistenInit: UnlistenFn | undefined;

    const setupListeners = async (): Promise<void> => {
      unlistenStatus = await listen<StatusUpdatePayload>('status_bar_update', (event) => {
        if (!isMountedRef.current) return;
        const payload = event.payload;
        if (!payload) return;
        const { scanned_files, processed_events } = payload;
        handleStatusUpdate(scanned_files, processed_events);
      });

      unlistenInit = await once('init_completed', () => {
        if (!isMountedRef.current) return;
        markInitialized();
      });
    };

    void setupListeners();

    return () => {
      isMountedRef.current = false;
      unlistenStatus?.();
      unlistenInit?.();
    };
  }, [handleStatusUpdate, markInitialized]);

  const onQueryChange = useCallback(
    (e: ChangeEvent<HTMLInputElement>) => {
      const inputValue = e.target.value;

      if (activeTab === 'events') {
        setEventFilterQuery(inputValue);
      } else {
        queueSearch(inputValue);
      }
    },
    [activeTab, queueSearch, setEventFilterQuery],
  );

  const onToggleRegex = useCallback(
    (event: ChangeEvent<HTMLInputElement>) => {
      const nextValue = event.target.checked;
      updateSearchParams({ useRegex: nextValue });
    },
    [updateSearchParams],
  );

  const onToggleCaseSensitive = useCallback(
    (event: ChangeEvent<HTMLInputElement>) => {
      const nextValue = event.target.checked;
      updateSearchParams({ caseSensitive: nextValue });
    },
    [updateSearchParams],
  );

  useEffect(() => {
    // Reset vertical scroll and prefetch initial rows to keep first render responsive
    const list = virtualListRef.current;
    if (!list) return;

    list.scrollToTop?.();

    if (!results.length || !list.ensureRangeLoaded) {
      return;
    }

    const preloadCount = Math.min(30, results.length);
    list.ensureRangeLoaded(0, preloadCount - 1);
  }, [results]);

  const handleHorizontalSync = useCallback((scrollLeft: number) => {
    // VirtualList drives the scroll position; mirror it onto the sticky header for alignment
    if (headerRef.current) {
      headerRef.current.scrollLeft = scrollLeft;
    }
  }, []);

  const renderRow = useCallback(
    (rowIndex: number, item: SearchResultItem | undefined, rowStyle: CSSProperties) => (
      <FileRow
        key={rowIndex}
        item={item ?? ''}
        rowIndex={rowIndex}
        style={{ ...rowStyle, width: 'var(--columns-total)' }} // Enforce column width CSS vars for virtualization rows
        onContextMenu={showContextMenu}
        searchQuery={currentQuery}
        caseInsensitive={!caseSensitive}
      />
    ),
    [showContextMenu, currentQuery, caseSensitive],
  );

  const getDisplayState = (): 'loading' | 'error' | 'empty' | 'results' => {
    // Derive the UI state from search lifecycle, preserving existing semantics
    if (!initialFetchCompleted) return 'loading';
    if (showLoadingUI) return 'loading';
    if (searchError) return 'error';
    if (results.length === 0) return 'empty';
    return 'results';
  };

  const displayState = getDisplayState();
  const searchErrorMessage =
    typeof searchError === 'string' ? searchError : searchError?.message ?? null;

  useEffect(() => {
    if (activeTab === 'events') {
      // Defer to next microtask so AutoSizer/Virtualized list have measured before scrolling
      queueMicrotask(() => {
        eventsPanelRef.current?.scrollToBottom?.();
      });
    }
  }, [activeTab]);

  const handleTabChange = useCallback(
    (newTab: ActiveTab) => {
      setActiveTab(newTab);
      if (newTab === 'events') {
        // Switch to events: always show newest items and clear transient filters
        setEventFilterQuery('');
      } else {
        // Switch to files: sync with reducer-managed search state and cancel pending timers
        resetSearchQuery();
        cancelPendingSearches();
      }
    },
    [cancelPendingSearches, resetSearchQuery, setEventFilterQuery],
  );

  const searchInputValue = activeTab === 'events' ? eventFilterQuery : searchParams.query;

  const containerStyle = {
    '--w-filename': `${colWidths.filename}px`,
    '--w-path': `${colWidths.path}px`,
    '--w-size': `${colWidths.size}px`,
    '--w-modified': `${colWidths.modified}px`,
    '--w-created': `${colWidths.created}px`,
    '--w-event-name': `${eventColWidths.name}px`,
    '--w-event-path': `${eventColWidths.path}px`,
    '--w-event-time': `${eventColWidths.time}px`,
    '--columns-events-total': `${
      eventColWidths.name + eventColWidths.path + eventColWidths.time
    }px`,
  } as CSSProperties;

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
      <div className="results-container" style={containerStyle}>
        {activeTab === 'events' ? (
          <FSEventsPanel
            ref={eventsPanelRef}
            events={filteredEvents}
            onResizeStart={onEventResizeStart}
            onContextMenu={showContextMenu}
            onHeaderContextMenu={showHeaderContextMenu}
            searchQuery={eventFilterQuery}
            caseInsensitive={!caseSensitive}
          />
        ) : (
          <div className="scroll-area">
            <ColumnHeader
              ref={headerRef}
              onResizeStart={onResizeStart}
              onContextMenu={showHeaderContextMenu}
            />
            <div className="flex-fill">
              {displayState !== 'results' ? (
                <StateDisplay
                  state={displayState}
                  message={searchErrorMessage}
                  query={currentQuery}
                />
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
