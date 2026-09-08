import { useRef, useCallback, useEffect, useMemo, useState } from 'react';
import type { ChangeEvent, CSSProperties, MouseEvent as ReactMouseEvent } from 'react';
import './App.css';
import { FileRow } from './components/FileRow';
import { SearchBar } from './components/SearchBar';
import { SearchFilters } from './components/SearchFilters';
import { FilesTabContent } from './components/FilesTabContent';
import { PermissionOverlay } from './components/PermissionOverlay';
import PreferencesOverlay from './components/PreferencesOverlay';
import StatusBar from './components/StatusBar';
import type { SearchResultItem } from './types/search';
import { useColumnResize } from './hooks/useColumnResize';
import { useContextMenu } from './hooks/useContextMenu';
import { useFileOperationLog } from './hooks/useFileOperationLog';
import { useFileSearch } from './hooks/useFileSearch';
import { useEventColumnWidths } from './hooks/useEventColumnWidths';
import { useRecentFSEvents } from './hooks/useRecentFSEvents';
import { DEFAULT_SORTABLE_RESULT_THRESHOLD, useRemoteSort } from './hooks/useRemoteSort';
import { useSelection } from './hooks/useSelection';
import { useQuickLook } from './hooks/useQuickLook';
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
import { useFilesTabEffects } from './hooks/useFilesTabEffects';
import { useFilesTabState } from './hooks/useFilesTabState';
import { useWorkspaceCollections } from './hooks/useWorkspaceCollections';
import type { SavedSearch } from './hooks/useWorkspaceCollections';
import { WorkspacePanel, WorkspaceToolbar } from './components/WorkspacePanel';
import type { WorkspaceSection } from './components/WorkspacePanel';
import { openPreferences } from './utils/openPreferences';
import { invoke } from '@tauri-apps/api/core';

function App() {
  const {
    state,
    searchParams,
    updateSearchParams,
    queueSearch,
    queueFilterSearch,
    queueDirectorySearch,
    queueDirectoryScopeOpen,
    handleStatusUpdate,
    setLifecycleState,
    requestRescan,
    applySearchPreset,
    indexingPaused,
    toggleIndexingPaused,
    cancelCurrentSearch,
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
    isSortKeyDisabled,
    showNewest,
    cancelSort,
    isSorting,
    handleSortToggle,
  } = useRemoteSort(results, resultsVersion, i18n.language, (limit) =>
    t('sorting.disabled', { limit }),
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
  } = useSelection(displayedResults, displayedResultsVersion, virtualListRef);

  const navigateFromSearchToResults = useCallback(() => {
    if (displayedResults.length === 0) {
      return;
    }

    selectSingleRow(0);
    searchInputRef.current?.blur();
  }, [displayedResults.length, selectSingleRow]);

  const {
    activeTab,
    setActiveTab,
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
  const { filteredEvents } = useRecentFSEvents({
    caseSensitive,
    isActive: activeTab === 'events',
    eventFilterQuery,
  });
  const [workspaceSection, setWorkspaceSection] = useState<WorkspaceSection | null>(null);
  const { fileOperations, updateFileOperation } = useFileOperationLog();
  const { savedSearches, favorites, saveSearch, removeSavedSearch, addFavorites, removeFavorite } =
    useWorkspaceCollections();

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
  } = useContextMenu(autoFitColumns, toggleQuickLook, updateFileOperation);

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
    focusAndSelectSearchInput();
  }, [focusAndSelectSearchInput]);

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
    includePaths,
    defaultIncludePaths,
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
  const directoryScopeLabel = t('search.options.directoryScope');
  const searchPlaceholder =
    activeTab === 'files' ? t('search.placeholder.files') : t('search.placeholder.events');
  const directorySearchPlaceholder = t('search.placeholder.directory');
  const searchAriaLabel = t('search.aria.searchInput');
  const permissionSteps = [
    t('app.fullDiskAccess.steps.one'),
    t('app.fullDiskAccess.steps.two'),
    t('app.fullDiskAccess.steps.three'),
  ];
  const openSettingsLabel = t('app.fullDiskAccess.openSettings');
  const resultsContainerClassName = `results-container${
    isSearchFocused ? ' results-container--search-focused' : ''
  }`;

  const toggleWorkspaceSection = useCallback((section: WorkspaceSection) => {
    setWorkspaceSection((current) => (current === section ? null : section));
  }, []);

  const handleSaveCurrentSearch = useCallback(
    (name: string) => {
      saveSearch({ name, ...searchParams });
    },
    [saveSearch, searchParams],
  );

  const handleApplySavedSearch = useCallback(
    (savedSearch: SavedSearch) => {
      setActiveTab('files');
      applySearchPreset({
        query: savedSearch.query,
        filterQuery: savedSearch.filterQuery,
        directoryQuery: savedSearch.directoryQuery,
        directoryScopeOpen: savedSearch.directoryScopeOpen,
        caseSensitive: savedSearch.caseSensitive,
      });
    },
    [applySearchPreset, setActiveTab],
  );

  return (
    <>
      <main className="container" aria-hidden={showFullDiskAccessOverlay || isPreferencesOpen}>
        <div className="search-toolbar">
          <SearchBar
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
            showNewestLabel={activeTab === 'files' ? t('search.options.newest') : undefined}
            onShowNewest={activeTab === 'files' ? showNewest : undefined}
            newestActive={sortState?.key === 'mtime' && sortState.direction === 'desc'}
            onFocus={handleSearchFocus}
            onBlur={handleSearchBlur}
          />
          <div hidden={activeTab !== 'files'}>
            <SearchFilters
              query={searchParams.filterQuery}
              onChange={(filterQuery) => queueFilterSearch(filterQuery, { immediate: true })}
            />
          </div>
          <WorkspaceToolbar activeSection={workspaceSection} onSelect={toggleWorkspaceSection} />
        </div>
        <div className={`content-workspace${workspaceSection ? ' has-panel' : ''}`}>
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
                isSortKeyDisabled={isSortKeyDisabled}
                sortDisabledTooltip={sortDisabledTooltip}
              />
            )}
          </div>
          {workspaceSection ? (
            <WorkspacePanel
              activeSection={workspaceSection}
              onClose={() => setWorkspaceSection(null)}
              savedSearches={savedSearches}
              favorites={favorites}
              selectedPaths={selectedPaths}
              onSaveSearch={handleSaveCurrentSearch}
              onApplySavedSearch={handleApplySavedSearch}
              onRemoveSavedSearch={removeSavedSearch}
              onAddSelectedFavorites={() => addFavorites(selectedPaths)}
              onOpenFavorite={openResultPath}
              onRevealFavorite={(path) => void invoke('open_in_finder', { path })}
              onRemoveFavorite={removeFavorite}
              coverage={{
                watchRoot: watchRoot ?? defaultWatchRoot,
                ignorePaths,
                includePaths,
                fullDiskAccessGranted: fullDiskAccessStatus === 'granted',
                lifecycleState,
                scannedFiles,
                currentQuery,
                currentDirectoryQuery,
              }}
              operations={{
                indexingPaused,
                lifecycleState,
                activity: indexingPaused
                  ? 'idle'
                  : lifecycleState === 'Initializing' || isSorting
                    ? 'high'
                    : showLoadingUI
                      ? 'medium'
                      : lifecycleState === 'Updating'
                        ? 'low'
                        : 'idle',
                scannedFiles,
                fileOperations,
                isSearching: showLoadingUI,
                isSorting,
                onToggleIndexing: () => void toggleIndexingPaused(),
                onCancelSearch: () => void cancelCurrentSearch(),
                onCancelSort: cancelSort,
                onRequestRescan: () => void requestRescan(),
              }}
              onOpenPreferences={openPreferences}
            />
          ) : null}
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
          indexingPaused={indexingPaused}
          onToggleIndexing={() => void toggleIndexingPaused()}
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
        includePaths={includePaths}
        defaultIncludePaths={defaultIncludePaths}
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
