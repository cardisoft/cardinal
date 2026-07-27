import { useRef, useCallback, useEffect, useMemo } from 'react';
import type { ChangeEvent, CSSProperties, MouseEvent as ReactMouseEvent } from 'react';
import '../App.css';
import { FileRow } from './FileRow';
import { SearchBar } from './SearchBar';
import { FilesTabContent } from './FilesTabContent';
import StatusBar from './StatusBar';
import type { SearchResultItem } from '../types/search';
import { useColumnResize } from '../hooks/useColumnResize';
import { useContextMenu } from '../hooks/useContextMenu';
import { useFileSearch } from '../hooks/useFileSearch';
import { useEventColumnWidths } from '../hooks/useEventColumnWidths';
import { useRecentFSEvents } from '../hooks/useRecentFSEvents';
import { useRemoteSort } from '../hooks/useRemoteSort';
import { useSelection } from '../hooks/useSelection';
import { useQuickLook } from '../hooks/useQuickLook';
import { ROW_HEIGHT, OVERSCAN_ROW_COUNT } from '../constants';
import type { VirtualListHandle } from './VirtualList';
import FSEventsPanel from './FSEventsPanel';
import type { FSEventsPanelHandle } from './FSEventsPanel';
import { useTranslation } from 'react-i18next';
import type { DisplayState } from './StateDisplay';
import { openResultPath } from '../utils/openResultPath';
import { splitPath } from '../utils/path';
import { useStableEvent } from '../hooks/useStableEvent';
import { useAppHotkeys } from '../hooks/useAppHotkeys';
import { useAppWindowListeners } from '../hooks/useAppWindowListeners';
import { useFilesTabEffects } from '../hooks/useFilesTabEffects';
import { useFilesTabState } from '../hooks/useFilesTabState';
import { useHierarchicalResults } from '../hooks/useHierarchicalResults';

export type TabRuntimeControls = {
  refreshSearchResults: () => void;
  sortThreshold: number;
  setSortThreshold: (value: number) => void;
};

type TabContentProps = {
  tabId: string;
  initialQuery?: string;
  isActive: boolean;
  isBlocked: boolean;
  onControlsChange: (tabId: string, controls: TabRuntimeControls | null) => void;
  onTitleChange: (tabId: string, title: string) => void;
};

export function TabContent({
  tabId,
  initialQuery,
  isActive,
  isBlocked,
  onControlsChange,
  onTitleChange,
}: TabContentProps) {
  const {
    state,
    searchParams,
    updateSearchParams,
    queueSearch,
    queueDirectorySearch,
    queueDirectoryScopeOpen,
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
    currentDirectoryQuery,
    highlightTerms,
    showLoadingUI,
    initialFetchCompleted,
    durationMs,
    resultCount,
    searchError,
    lifecycleState,
  } = state;

  const eventsPanelRef = useRef<FSEventsPanelHandle | null>(null);
  const contentRootRef = useRef<HTMLElement | null>(null);
  const headerRef = useRef<HTMLDivElement | null>(null);
  const virtualListRef = useRef<VirtualListHandle | null>(null);
  const searchInputRef = useRef<HTMLInputElement>(null);

  const { colWidths, onResizeStart, autoFitColumns } = useColumnResize();
  const { caseSensitive, directoryQuery, directoryScopeOpen } = searchParams;
  const { eventColWidths, onEventResizeStart, autoFitEventColumns } = useEventColumnWidths();
  const { t, i18n } = useTranslation();
  // `resultsVersion` tracks raw backend search result-set changes.
  // `displayedResultsVersion` additionally tracks UI ordering/projection changes (e.g. sort toggle).
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
  const hierarchicalResults = useHierarchicalResults(
    results,
    displayedResults,
    resultsVersion,
    displayedResultsVersion,
  );

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
  } = useSelection(
    hierarchicalResults.results,
    hierarchicalResults.resultsVersion,
    virtualListRef,
  );

  const navigateFromSearchToResults = useCallback(() => {
    if (hierarchicalResults.results.length === 0) {
      return;
    }

    selectSingleRow(0);
    searchInputRef.current?.blur();
  }, [hierarchicalResults.results.length, selectSingleRow]);

  const {
    activeTab,
    isSearchFocused,
    handleSearchFocus,
    handleSearchBlur,
    eventFilterQuery,
    setEventFilterQuery,
    onTabChange,
    searchInputValue,
    directoryInputValue,
    onQueryChange,
    onDirectoryQueryChange,
    onDirectoryInputKeyDown,
    onSearchInputKeyDown,
    submitFilesQuery,
  } = useFilesTabState({
    searchQuery: searchParams.query,
    directoryQuery,
    queueSearch,
    queueDirectorySearch,
    onNavigateFromSearchToResults: navigateFromSearchToResults,
  });

  const initialQueryRef = useRef(initialQuery);
  useEffect(() => {
    const query = initialQueryRef.current;
    if (!query) {
      return;
    }
    initialQueryRef.current = undefined;
    submitFilesQuery(query, { immediate: true });
  }, [submitFilesQuery]);

  const { filteredEvents } = useRecentFSEvents({
    caseSensitive,
    isActive: isActive && activeTab === 'events',
    eventFilterQuery,
  });

  const getQuickLookPaths = useCallback(
    () => (activeTab === 'files' ? selectedPaths : []),
    [activeTab, selectedPaths],
  );
  // Quick Look controller keeps preview panel in sync with whichever rows are currently selected.
  const { toggleQuickLook, updateQuickLook, closeQuickLook } = useQuickLook({
    getPaths: getQuickLookPaths,
    rootRef: contentRootRef,
  });

  const {
    showContextMenu: showFilesContextMenu,
    showHeaderContextMenu: showFilesHeaderContextMenu,
  } = useContextMenu(autoFitColumns, toggleQuickLook);

  const {
    showContextMenu: showEventsContextMenu,
    showHeaderContextMenu: showEventsHeaderContextMenu,
  } = useContextMenu(autoFitEventColumns);

  const focusSearchInput = useCallback(() => {
    requestAnimationFrame(() => {
      const input = searchInputRef.current;
      if (!input) return;
      input.focus();
    });
  }, []);

  const focusAndSelectSearchInput = useCallback(() => {
    requestAnimationFrame(() => {
      const input = searchInputRef.current;
      if (!input) return;
      input.focus();
      input.select();
    });
  }, []);

  useEffect(() => {
    if (isActive) {
      focusAndSelectSearchInput();
    }
  }, [focusAndSelectSearchInput, isActive]);

  const refreshSearchResults = useCallback(() => {
    queueSearch(currentQuery, { immediate: true });
  }, [currentQuery, queueSearch]);

  const tabTitle = useMemo(() => {
    const queryTitle = searchParams.query.trim();
    if (queryTitle) {
      return queryTitle;
    }

    if (directoryScopeOpen) {
      const trimmedDirectory = directoryQuery.trim();
      const normalizedDirectory =
        trimmedDirectory === '/' ? trimmedDirectory : trimmedDirectory.replace(/\/+$/, '');
      const directoryTitle = splitPath(normalizedDirectory).name;
      if (directoryTitle) {
        return directoryTitle;
      }
    }

    return t('tabs.newSearch');
  }, [directoryQuery, directoryScopeOpen, searchParams.query, t]);

  useEffect(() => {
    onTitleChange(tabId, tabTitle);
  }, [onTitleChange, tabId, tabTitle]);

  const runtimeControls = useMemo<TabRuntimeControls>(
    () => ({
      refreshSearchResults,
      sortThreshold,
      setSortThreshold,
    }),
    [refreshSearchResults, setSortThreshold, sortThreshold],
  );

  useEffect(() => {
    onControlsChange(tabId, runtimeControls);
    return () => onControlsChange(tabId, null);
  }, [onControlsChange, runtimeControls, tabId]);

  useAppWindowListeners({
    isActive,
    activeTab,
    searchInputRef,
    focusAndSelectSearchInput,
    handleStatusUpdate,
    setLifecycleState,
    submitFilesQuery,
    setEventFilterQuery,
  });

  const navigateSelection = useStableEvent(moveSelection);
  const triggerQuickLook = useStableEvent(toggleQuickLook);

  useAppHotkeys({
    enabled: isActive && !isBlocked,
    activeTab,
    activeRowIndex,
    selectedPaths,
    selectedIndicesRef,
    focusSearchInput,
    focusAndSelectSearchInput,
    clearSelection,
    navigateSelection,
    triggerQuickLook,
  });

  useFilesTabEffects({
    isWorkspaceActive: isActive,
    activeTab,
    selectedIndices,
    activeRowIndex,
    closeQuickLook,
    updateQuickLook,
    clearSelection,
    resultsVersion,
    virtualListRef,
    eventsPanelRef,
  });

  const onToggleCaseSensitive = useCallback(
    (event: ChangeEvent<HTMLInputElement>) => {
      const nextValue = event.target.checked;
      updateSearchParams({ caseSensitive: nextValue });
    },
    [updateSearchParams],
  );

  const toggleDirectoryScope = useCallback(() => {
    queueDirectoryScopeOpen(!directoryScopeOpen);
  }, [directoryScopeOpen, queueDirectoryScopeOpen]);

  const handleHorizontalSync = useCallback((scrollLeft: number) => {
    // VirtualList drives the scroll position; mirror it onto the sticky header for alignment.
    if (headerRef.current) {
      headerRef.current.scrollLeft = scrollLeft;
    }
  }, []);

  const selectedIndexSet = useMemo(() => new Set(selectedIndices), [selectedIndices]);

  const handleRowContextMenu = useCallback(
    (event: ReactMouseEvent<HTMLDivElement>, path: string, rowIndex: number) => {
      const isRowSelected = selectedIndexSet.has(rowIndex);
      if (!isRowSelected) {
        selectSingleRow(rowIndex);
      }
      const targetPaths = isRowSelected && selectedPaths.length > 0 ? selectedPaths : [path];
      showFilesContextMenu(event, targetPaths);
    },
    [selectedIndexSet, selectedPaths, selectSingleRow, showFilesContextMenu],
  );

  const handleEventsContextMenu = useCallback(
    (event: ReactMouseEvent<HTMLDivElement>, path: string) => {
      showEventsContextMenu(event, [path]);
    },
    [showEventsContextMenu],
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
          treeDepth={hierarchicalResults.rows[rowIndex]?.depth}
          isExpandable={hierarchicalResults.rows[rowIndex]?.hasChildren}
          isExpanded={hierarchicalResults.rows[rowIndex]?.isExpanded}
          onToggleExpanded={() => {
            const treeRow = hierarchicalResults.rows[rowIndex];
            if (treeRow?.hasChildren) {
              hierarchicalResults.toggleExpanded(treeRow.slabIndex);
            }
          }}
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
      hierarchicalResults,
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

  const caseSensitiveLabel = t('search.options.caseSensitive');
  const directoryScopeLabel = t('search.options.directoryScope');
  const searchPlaceholder =
    activeTab === 'files' ? t('search.placeholder.files') : t('search.placeholder.events');
  const directorySearchPlaceholder = t('search.placeholder.directory');
  const searchAriaLabel = t('search.aria.searchInput');
  const resultsContainerClassName = `results-container${
    isSearchFocused ? ' results-container--search-focused' : ''
  }`;

  return (
      <main
        ref={contentRootRef}
        className="container"
        aria-hidden={!isActive || isBlocked}
        data-tab-id={tabId}
      >
        <SearchBar
          inputId={`search-input-${tabId}`}
          directoryInputId={`directory-scope-input-${tabId}`}
          inputRef={searchInputRef}
          placeholder={searchPlaceholder}
          ariaLabel={searchAriaLabel}
          value={searchInputValue}
          onChange={onQueryChange}
          onKeyDown={onSearchInputKeyDown}
          directoryScopeEnabled={activeTab === 'files'}
          directoryScopeOpen={directoryScopeOpen}
          directoryScopeLabel={directoryScopeLabel}
          directoryPlaceholder={directorySearchPlaceholder}
          directoryValue={directoryInputValue}
          onToggleDirectoryScope={toggleDirectoryScope}
          onDirectoryChange={onDirectoryQueryChange}
          onDirectoryKeyDown={onDirectoryInputKeyDown}
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
              onContextMenu={handleEventsContextMenu}
              onHeaderContextMenu={showEventsHeaderContextMenu}
              searchQuery={eventFilterQuery}
              caseInsensitive={!caseSensitive}
            />
          ) : (
            // `dataResultsVersion`: backend result-set changes. This resets row metadata cache.
            // `displayedResultsVersion`: visible-order/projection changes. This refreshes viewport
            // work such as icon hydration and frozen-view handoff in VirtualList.
            <FilesTabContent
              headerRef={headerRef}
              onResizeStart={onResizeStart}
              onHeaderContextMenu={showFilesHeaderContextMenu}
              displayState={displayState}
              searchErrorMessage={searchErrorMessage}
              currentQuery={currentQuery}
              currentDirectoryQuery={currentDirectoryQuery}
              virtualListRef={virtualListRef}
              results={hierarchicalResults.results}
              dataResultsVersion={resultsVersion}
              displayedResultsVersion={hierarchicalResults.resultsVersion}
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
          onTabChange={onTabChange}
          onRequestRescan={requestRescan}
          rescanErrorCount={rescanErrors}
        />
      </main>
  );
}
