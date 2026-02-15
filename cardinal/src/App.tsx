import { useRef, useCallback, useEffect, useMemo } from 'react';
import type {
  ChangeEvent,
  CSSProperties,
  KeyboardEvent as ReactKeyboardEvent,
  MouseEvent as ReactMouseEvent,
} from 'react';
import './App.css';
import { FileRow } from './components/FileRow';
import { SearchBar } from './components/SearchBar';
import { FilesTabContent } from './components/FilesTabContent';
import { PermissionOverlay } from './components/PermissionOverlay';
import PreferencesOverlay from './components/PreferencesOverlay';
import StatusBar from './components/StatusBar';
import type { StatusTabKey } from './components/StatusBar';
import type { SearchResultItem } from './types/search';
import { useColumnResize } from './hooks/useColumnResize';
import { useContextMenu } from './hooks/useContextMenu';
import { useFileSearch } from './hooks/useFileSearch';
import { useEventColumnWidths } from './hooks/useEventColumnWidths';
import { useRecentFSEvents } from './hooks/useRecentFSEvents';
import { DEFAULT_SORTABLE_RESULT_THRESHOLD, useRemoteSort } from './hooks/useRemoteSort';
import { useSelection } from './hooks/useSelection';
import { useQuickLook } from './hooks/useQuickLook';
import { useSearchHistory } from './hooks/useSearchHistory';
import { ROW_HEIGHT, OVERSCAN_ROW_COUNT } from './constants';
import type { VirtualListHandle } from './components/VirtualList';
import FSEventsPanel from './components/FSEventsPanel';
import type { FSEventsPanelHandle } from './components/FSEventsPanel';
import { useTranslation } from 'react-i18next';
import { useFullDiskAccessPermission } from './hooks/useFullDiskAccessPermission';
import type { DisplayState } from './components/StateDisplay';
import { openResultPath } from './utils/openResultPath';
import { useStableEvent } from './hooks/useStableEvent';
import { useAppHotkeys } from './hooks/useAppHotkeys';
import { useAppPreferences } from './hooks/useAppPreferences';
import { useAppWindowListeners } from './hooks/useAppWindowListeners';
import { useFilesTabState } from './hooks/useFilesTabState';

const MAX_SEARCH_HISTORY_ENTRIES = 50;

function App() {
  const {
    state,
    searchParams,
    updateSearchParams,
    queueSearch,
    handleStatusUpdate,
    setLifecycleState,
    requestRescan,
  } = useFileSearch();
  const {
    results,
    resultsVersion,
    scannedFiles,
    processedEvents,
    rescanErrors,
    currentQuery,
    highlightTerms,
    showLoadingUI,
    initialFetchCompleted,
    durationMs,
    resultCount,
    searchError,
    lifecycleState,
  } = state;

  const eventsPanelRef = useRef<FSEventsPanelHandle | null>(null);
  const headerRef = useRef<HTMLDivElement | null>(null);
  const virtualListRef = useRef<VirtualListHandle | null>(null);
  const searchInputRef = useRef<HTMLInputElement | null>(null);

  const {
    handleInputChange: updateHistoryFromInput,
    navigate: navigateSearchHistory,
    ensureTailValue: ensureHistoryBuffer,
    resetCursorToTail,
  } = useSearchHistory({ maxEntries: MAX_SEARCH_HISTORY_ENTRIES });
  const { colWidths, onResizeStart, autoFitColumns } = useColumnResize();
  const { caseSensitive } = searchParams;
  const { eventColWidths, onEventResizeStart, autoFitEventColumns } = useEventColumnWidths();
  const { t, i18n } = useTranslation();
  const {
    sortState,
    displayedResults,
    displayedResultsVersion,
    sortThreshold,
    setSortThreshold,
    sortDisabledTooltip,
    sortButtonsDisabled,
    handleSortToggle,
  } = useRemoteSort(results, resultsVersion, i18n.language, (limit) =>
    t('sorting.disabled', { limit }),
  );

  const { activeTab, setActiveTab, isSearchFocused, handleSearchFocus, handleSearchBlur } =
    useFilesTabState();
  const { filteredEvents, eventFilterQuery, setEventFilterQuery } = useRecentFSEvents({
    caseSensitive,
    isActive: activeTab === 'events',
  });

  // Centralized selection management for the virtualized files list.
  // Provides memoized helpers for click/keyboard selection and keeps Quick Look hooks fed.
  const {
    selectedIndices,
    selectedIndicesRef,
    activeRowIndex,
    selectedPaths,
    handleRowSelect,
    selectSingleRow,
    clearSelection,
    moveSelection,
  } = useSelection(displayedResults, displayedResultsVersion, virtualListRef);

  const getQuickLookPaths = useCallback(
    () => (activeTab === 'files' ? selectedPaths : []),
    [activeTab, selectedPaths],
  );
  // Quick Look controller keeps preview panel in sync with whichever rows are currently selected.
  const { toggleQuickLook, updateQuickLook, closeQuickLook } = useQuickLook({
    getPaths: getQuickLookPaths,
  });

  const {
    showContextMenu: showFilesContextMenu,
    showHeaderContextMenu: showFilesHeaderContextMenu,
  } = useContextMenu(autoFitColumns, toggleQuickLook, () =>
    activeTab === 'files' ? selectedPaths : [],
  );

  const {
    showContextMenu: showEventsContextMenu,
    showHeaderContextMenu: showEventsHeaderContextMenu,
  } = useContextMenu(autoFitEventColumns);

  const {
    status: fullDiskAccessStatus,
    isChecking: isCheckingFullDiskAccess,
    requestPermission: requestFullDiskAccessPermission,
  } = useFullDiskAccessPermission();

  const focusSearchInput = useCallback(() => {
    requestAnimationFrame(() => {
      const input = searchInputRef.current;
      if (!input) return;
      input.focus();
      input.select();
    });
  }, []);

  useEffect(() => {
    focusSearchInput();
  }, [focusSearchInput]);

  const refreshSearchResults = useCallback(() => {
    queueSearch(currentQuery, { immediate: true });
  }, [currentQuery, queueSearch]);

  const {
    isPreferencesOpen,
    closePreferences,
    trayIconEnabled,
    setTrayIconEnabled,
    watchRoot,
    defaultWatchRoot,
    ignorePaths,
    defaultIgnorePaths,
    preferencesResetToken,
    handleWatchConfigChange,
    handleResetPreferences,
  } = useAppPreferences({
    fullDiskAccessStatus,
    isCheckingFullDiskAccess,
    refreshSearchResults,
    i18n,
  });

  useAppWindowListeners({
    activeTab,
    focusSearchInput,
    handleStatusUpdate,
    setLifecycleState,
    queueSearch,
    setEventFilterQuery,
    updateHistoryFromInput,
  });

  const navigateSelection = useStableEvent(moveSelection);
  const triggerQuickLook = useStableEvent(toggleQuickLook);

  useAppHotkeys({
    activeTab,
    selectedPaths,
    selectedIndicesRef,
    focusSearchInput,
    navigateSelection,
    triggerQuickLook,
  });

  useEffect(() => {
    if (activeTab !== 'files') {
      clearSelection();
    }
  }, [activeTab, clearSelection]);

  useEffect(() => {
    if (activeTab !== 'files') {
      closeQuickLook();
      return;
    }

    if (selectedIndices.length) {
      updateQuickLook();
    }
  }, [activeTab, selectedIndices, closeQuickLook, updateQuickLook]);

  useEffect(() => {
    if (activeRowIndex == null) {
      return;
    }

    virtualListRef.current?.scrollToRow?.(activeRowIndex, 'nearest');
  }, [activeRowIndex]);

  useEffect(() => {
    clearSelection();
    virtualListRef.current?.scrollToTop?.();
  }, [results, clearSelection]);

  useEffect(() => {
    if (activeTab === 'events') {
      // Defer to next microtask so AutoSizer/Virtualized list have measured before scrolling.
      queueMicrotask(() => {
        eventsPanelRef.current?.scrollToBottom?.();
      });
    }
  }, [activeTab]);

  const onQueryChange = useCallback(
    (event: ChangeEvent<HTMLInputElement>) => {
      const inputValue = event.target.value;

      if (activeTab === 'events') {
        setEventFilterQuery(inputValue);
        return;
      }

      queueSearch(inputValue, { onSearchCommitted: updateHistoryFromInput });
    },
    [activeTab, queueSearch, setEventFilterQuery, updateHistoryFromInput],
  );

  const onToggleCaseSensitive = useCallback(
    (event: ChangeEvent<HTMLInputElement>) => {
      const nextValue = event.target.checked;
      updateSearchParams({ caseSensitive: nextValue });
    },
    [updateSearchParams],
  );

  const handleHistoryNavigation = useCallback(
    (direction: 'older' | 'newer') => {
      if (activeTab !== 'files') {
        return;
      }
      const nextValue = navigateSearchHistory(direction);
      if (nextValue === null) {
        return;
      }
      queueSearch(nextValue);
    },
    [activeTab, navigateSearchHistory, queueSearch],
  );

  const onSearchInputKeyDown = useCallback(
    (event: ReactKeyboardEvent<HTMLInputElement>) => {
      if (activeTab !== 'files') {
        return;
      }
      if (event.key === 'Enter') {
        queueSearch(event.currentTarget.value, {
          immediate: true,
          onSearchCommitted: updateHistoryFromInput,
        });
        return;
      }
      if (event.key !== 'ArrowUp' && event.key !== 'ArrowDown') {
        return;
      }
      if (event.altKey || event.metaKey || event.ctrlKey || event.shiftKey) {
        return;
      }

      event.preventDefault();
      handleHistoryNavigation(event.key === 'ArrowUp' ? 'older' : 'newer');
    },
    [activeTab, handleHistoryNavigation, queueSearch, updateHistoryFromInput],
  );

  const handleHorizontalSync = useCallback((scrollLeft: number) => {
    // VirtualList drives the scroll position; mirror it onto the sticky header for alignment.
    if (headerRef.current) {
      headerRef.current.scrollLeft = scrollLeft;
    }
  }, []);

  const selectedIndexSet = useMemo(() => new Set(selectedIndices), [selectedIndices]);

  const handleRowContextMenu = useCallback(
    (event: ReactMouseEvent<HTMLDivElement>, path: string, rowIndex: number) => {
      if (!selectedIndexSet.has(rowIndex)) {
        selectSingleRow(rowIndex);
      }
      if (path) {
        showFilesContextMenu(event, path);
      }
    },
    [selectedIndexSet, selectSingleRow, showFilesContextMenu],
  );

  const renderRow = useCallback(
    (rowIndex: number, item: SearchResultItem | undefined, rowStyle: CSSProperties) => {
      if (!item) {
        return (
          <div
            key={`placeholder-${rowIndex}`}
            className="row columns row-loading"
            style={{ ...rowStyle, width: 'var(--columns-total)' }}
          />
        );
      }

      return (
        <FileRow
          key={item.path}
          rowIndex={rowIndex}
          item={item}
          style={{ ...rowStyle, width: 'var(--columns-total)' }}
          isSelected={selectedIndexSet.has(rowIndex)}
          selectedPathsForDrag={selectedPaths}
          caseInsensitive={!caseSensitive}
          highlightTerms={highlightTerms}
          onContextMenu={handleRowContextMenu}
          onSelect={handleRowSelect}
          onOpen={openResultPath}
        />
      );
    },
    [
      handleRowContextMenu,
      handleRowSelect,
      highlightTerms,
      caseSensitive,
      selectedIndexSet,
      selectedPaths,
    ],
  );

  const displayState: DisplayState = (() => {
    if (!initialFetchCompleted) return 'loading';
    if (showLoadingUI) return 'loading';
    if (searchError) return 'error';
    if (results.length === 0) return 'empty';
    return 'results';
  })();
  const searchErrorMessage =
    typeof searchError === 'string' ? searchError : (searchError?.message ?? null);

  const handleTabChange = useCallback(
    (newTab: StatusTabKey) => {
      setActiveTab(newTab);
      if (newTab === 'events') {
        // Switch to events: always show newest items and clear transient filters.
        setEventFilterQuery('');
        resetCursorToTail();
      } else {
        // Switch to files: sync with reducer-managed search state and cancel pending timers.
        ensureHistoryBuffer('');
        queueSearch('', { immediate: true });
      }
    },
    [ensureHistoryBuffer, queueSearch, resetCursorToTail, setEventFilterQuery],
  );

  const searchInputValue = activeTab === 'events' ? eventFilterQuery : searchParams.query;

  const containerStyle = useMemo(
    () =>
      ({
        '--w-filename': `${colWidths.filename}px`,
        '--w-path': `${colWidths.path}px`,
        '--w-size': `${colWidths.size}px`,
        '--w-modified': `${colWidths.modified}px`,
        '--w-created': `${colWidths.created}px`,
        '--w-event-flags': `${eventColWidths.event}px`,
        '--w-event-name': `${eventColWidths.name}px`,
        '--w-event-path': `${eventColWidths.path}px`,
        '--w-event-time': `${eventColWidths.time}px`,
        '--columns-events-total': `${
          eventColWidths.event + eventColWidths.name + eventColWidths.path + eventColWidths.time
        }px`,
      }) as CSSProperties,
    [colWidths, eventColWidths],
  );

  const showFullDiskAccessOverlay = fullDiskAccessStatus === 'denied';
  const overlayStatusMessage = isCheckingFullDiskAccess
    ? t('app.fullDiskAccess.status.checking')
    : t('app.fullDiskAccess.status.disabled');
  const caseSensitiveLabel = t('search.options.caseSensitive');
  const searchPlaceholder =
    activeTab === 'files' ? t('search.placeholder.files') : t('search.placeholder.events');
  const permissionSteps = [
    t('app.fullDiskAccess.steps.one'),
    t('app.fullDiskAccess.steps.two'),
    t('app.fullDiskAccess.steps.three'),
  ];
  const openSettingsLabel = t('app.fullDiskAccess.openSettings');
  const resultsContainerClassName = `results-container${
    isSearchFocused ? ' results-container--search-focused' : ''
  }`;

  return (
    <>
      <main className="container" aria-hidden={showFullDiskAccessOverlay || isPreferencesOpen}>
        <SearchBar
          inputRef={searchInputRef}
          placeholder={searchPlaceholder}
          value={searchInputValue}
          onChange={onQueryChange}
          onKeyDown={onSearchInputKeyDown}
          caseSensitive={caseSensitive}
          onToggleCaseSensitive={onToggleCaseSensitive}
          caseSensitiveLabel={caseSensitiveLabel}
          onFocus={handleSearchFocus}
          onBlur={handleSearchBlur}
        />
        <div className={resultsContainerClassName} style={containerStyle}>
          {activeTab === 'events' ? (
            <FSEventsPanel
              ref={eventsPanelRef}
              events={filteredEvents}
              onResizeStart={onEventResizeStart}
              onContextMenu={showEventsContextMenu}
              onHeaderContextMenu={showEventsHeaderContextMenu}
              searchQuery={eventFilterQuery}
              caseInsensitive={!caseSensitive}
            />
          ) : (
            <FilesTabContent
              headerRef={headerRef}
              onResizeStart={onResizeStart}
              onHeaderContextMenu={showFilesHeaderContextMenu}
              displayState={displayState}
              searchErrorMessage={searchErrorMessage}
              currentQuery={currentQuery}
              virtualListRef={virtualListRef}
              results={displayedResults}
              dataResultsVersion={resultsVersion}
              displayedResultsVersion={displayedResultsVersion}
              rowHeight={ROW_HEIGHT}
              overscan={OVERSCAN_ROW_COUNT}
              renderRow={renderRow}
              onScrollSync={handleHorizontalSync}
              sortState={sortState}
              onSortToggle={handleSortToggle}
              sortDisabled={sortButtonsDisabled}
              sortDisabledTooltip={sortDisabledTooltip}
            />
          )}
        </div>
        <StatusBar
          scannedFiles={scannedFiles}
          processedEvents={processedEvents}
          lifecycleState={lifecycleState}
          searchDurationMs={durationMs}
          resultCount={resultCount}
          activeTab={activeTab}
          onTabChange={handleTabChange}
          onRequestRescan={requestRescan}
          rescanErrorCount={rescanErrors}
        />
      </main>
      <PreferencesOverlay
        open={isPreferencesOpen}
        onClose={closePreferences}
        sortThreshold={sortThreshold}
        defaultSortThreshold={DEFAULT_SORTABLE_RESULT_THRESHOLD}
        onSortThresholdChange={setSortThreshold}
        trayIconEnabled={trayIconEnabled}
        onTrayIconEnabledChange={setTrayIconEnabled}
        watchRoot={watchRoot ?? defaultWatchRoot}
        defaultWatchRoot={defaultWatchRoot}
        onWatchConfigChange={handleWatchConfigChange}
        ignorePaths={ignorePaths}
        defaultIgnorePaths={defaultIgnorePaths}
        onReset={handleResetPreferences}
        themeResetToken={preferencesResetToken}
      />
      {showFullDiskAccessOverlay && (
        <PermissionOverlay
          title={t('app.fullDiskAccess.title')}
          description={t('app.fullDiskAccess.description')}
          steps={permissionSteps}
          statusMessage={overlayStatusMessage}
          onRequestPermission={requestFullDiskAccessPermission}
          disabled={isCheckingFullDiskAccess}
          actionLabel={openSettingsLabel}
        />
      )}
    </>
  );
}

export default App;
