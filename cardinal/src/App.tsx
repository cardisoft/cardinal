import { useRef, useCallback, useEffect, useMemo, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { ChangeEvent, CSSProperties, MouseEvent as ReactMouseEvent } from 'react';
import './App.css';
import { FileRow } from './components/FileRow';
import { IconGrid } from './components/IconGrid';
import type { IconSizePreset } from './components/IconGrid';
import { IconGridItem } from './components/IconGridItem';
import { SearchTabsBar } from './components/SearchTabsBar';
import { SearchBar } from './components/SearchBar';
import { FilesTabContent } from './components/FilesTabContent';
import { PermissionOverlay } from './components/PermissionOverlay';
import PreferencesOverlay from './components/PreferencesOverlay';
import StatusBar from './components/StatusBar';
import type { SearchResultItem } from './types/search';
import type { SlabIndex } from './types/slab';
import type { ResultsViewHandle } from './types/viewResults';
import { useColumnResize } from './hooks/useColumnResize';
import { useContextMenu } from './hooks/useContextMenu';
import { useFileSearch } from './hooks/useFileSearch';
import { useEventColumnWidths } from './hooks/useEventColumnWidths';
import { useRecentFSEvents } from './hooks/useRecentFSEvents';
import { DEFAULT_SORTABLE_RESULT_THRESHOLD, useRemoteSort } from './hooks/useRemoteSort';
import { useSelection } from './hooks/useSelection';
import { useQuickLook } from './hooks/useQuickLook';
import { useBookmarks } from './hooks/useBookmarks';
import { useSearchTabs } from './hooks/useSearchTabs';
import { useViewMode } from './hooks/useViewMode';
import { useSearchFilters } from './hooks/useSearchFilters';
import { useFontSize } from './hooks/useFontSize';
import { FiltersOverlay } from './components/FiltersOverlay';
import { OpenWithMenu } from './components/OpenWithMenu';
import { SortMenu } from './components/SortMenu';
import type { SortKey, SortDirection } from './types/sort';
import { DEFAULT_FILTER_PRESETS } from './types/searchFilters';
import { FONT_SIZE_DEFAULT, FONT_SIZE_MAX, FONT_SIZE_MIN } from './types/fontSize';
import { OVERSCAN_ROW_COUNT, RECENT_PAGE_SIZE } from './constants';
import { OPEN_BOOKMARKS_EVENT } from './constants/appEvents';
import type { Bookmark, BookmarkOpenMode } from './types/bookmarks';
import FSEventsPanel from './components/FSEventsPanel';
import type { FSEventsPanelHandle } from './components/FSEventsPanel';
import { useTranslation } from 'react-i18next';
import { useFullDiskAccessPermission } from './hooks/useFullDiskAccessPermission';
import type { DisplayState } from './components/StateDisplay';
import { openResultPath } from './utils/openResultPath';
import { useBookmarkOpenListener } from './hooks/useBookmarkOpenListener';
import { useStableEvent } from './hooks/useStableEvent';
import { useAppHotkeys } from './hooks/useAppHotkeys';
import { useAppPreferences } from './hooks/useAppPreferences';
import { useAppWindowListeners } from './hooks/useAppWindowListeners';
import { useFilesTabEffects } from './hooks/useFilesTabEffects';
import { useFilesTabState } from './hooks/useFilesTabState';
import type { SelectionSummary, StatusTabKey } from './components/StatusBar';
import { formatSize, formatTimestamp } from './utils/format';

function App() {
  const {
    state,
    searchParams,
    updateSearchParams,
    queueSearch,
    appendResults,
    queueFilterQuery,
    queueDirectorySearch,
    queueDirectoryScopeOpen,
    queueTabSearch,
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
    indexing,
    indexingScope,
    unavailableScope,
    lifecycleState,
  } = state;

  const eventsPanelRef = useRef<FSEventsPanelHandle | null>(null);
  const headerRef = useRef<HTMLDivElement | null>(null);
  const { colWidths, onResizeStart, autoFitColumns } = useColumnResize();
  const { caseSensitive, directoryQuery, directoryScopeOpen } = searchParams;
  const { eventColWidths, onEventResizeStart, autoFitEventColumns } = useEventColumnWidths();
  const { t, i18n } = useTranslation();

  // Multi search tabs: each tab owns a query; the active tab drives the search input.
  const searchTabs = useSearchTabs();
  const bookmarks = useBookmarks();
  const viewModeControls = useViewMode();
  const searchFilters = useSearchFilters();
  const fontSettings = useFontSize();

  const virtualListRef = useRef<ResultsViewHandle | null>(null);
  const searchInputRef = useRef<HTMLInputElement>(null);
  const [isFiltersOpen, setIsFiltersOpen] = useState(false);
  const [bookmarkToast, setBookmarkToast] = useState<string | null>(null);
  const [sortMenuPoint, setSortMenuPoint] = useState<{ x: number; y: number } | null>(null);
  // `resultsVersion` tracks raw backend search result-set changes.
  // `displayedResultsVersion` additionally tracks UI ordering/projection changes (e.g. sort toggle).
  const {
    sortState,
    setSortState,
    displayedResults,
    displayedResultsVersion,
    sortThreshold,
    setSortThreshold,
    sortDisabledTooltip,
    sortButtonsDisabled,
    handleSortToggle,
  } = useRemoteSort(
    results,
    resultsVersion,
    i18n.language,
    (limit) => t('sorting.disabled', { limit }),
    searchTabs.activeTabId,
  );

  // Centralized selection management for the virtualized files list.
  // Provides memoized helpers for click/keyboard selection and keeps Quick Look hooks fed.
  const {
    selectedIndices,
    selectedIndicesRef,
    activeRowIndex,
    selectedPaths,
    selectedItems,
    handleRowSelect,
    selectSingleRow,
    clearSelection,
    moveSelection,
  } = useSelection(displayedResults, displayedResultsVersion, virtualListRef);

  // The first selected row feeds the status bar (path + size) and the toolbar (timestamps).
  const activeSelection = useMemo(() => {
    const item = selectedItems[0];
    return item ? { item, count: selectedItems.length } : null;
  }, [selectedItems]);

  const selectionSummary = useMemo<SelectionSummary | null>(() => {
    if (!activeSelection) {
      return null;
    }
    return {
      path: activeSelection.item.path,
      size: formatSize(activeSelection.item.size),
      count: activeSelection.count,
    };
  }, [activeSelection]);

  const navigateFromSearchToResults = useCallback(() => {
    if (displayedResults.length === 0) {
      return;
    }

    selectSingleRow(0);
    searchInputRef.current?.blur();
  }, [displayedResults.length, selectSingleRow]);

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
    // The folder scope is per tab, so mirror every edit into the active tab.
    onDirectoryCommitted: searchTabs.setActiveTabDirectory,
    onNavigateFromSearchToResults: navigateFromSearchToResults,
  });
  const { filteredEvents } = useRecentFSEvents({
    caseSensitive,
    isActive: activeTab === 'events',
    eventFilterQuery,
  });

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
    openWithAnchor,
    closeOpenWith,
  } = useContextMenu(autoFitColumns, toggleQuickLook);

  const {
    showContextMenu: showEventsContextMenu,
    showHeaderContextMenu: showEventsHeaderContextMenu,
  } = useContextMenu(autoFitEventColumns);

  const {
    status: fullDiskAccessStatus,
    isChecking: isCheckingFullDiskAccess,
    requestPermission: requestFullDiskAccessPermission,
    recheck: recheckFullDiskAccess,
  } = useFullDiskAccessPermission();

  // Cmd+S: the bookmark's name and query are the current search text, and it keeps the
  // active filter so reopening reproduces the search. Saved into the permanent category.
  const handleSaveCurrentSearchAsBookmark = useCallback(() => {
    const query = searchTabs.activeTab.query.trim();
    if (!query) {
      return;
    }
    bookmarks.addBookmark(bookmarks.defaultCategoryId, {
      name: query,
      query,
      filterSearch: searchTabs.activeTab.filterQuery,
    });
    setBookmarkToast(t('bookmarks.savedToast'));
  }, [bookmarks, searchTabs]);

  useEffect(() => {
    if (!bookmarkToast) {
      return;
    }
    const timer = setTimeout(() => setBookmarkToast(null), 2200);
    return () => clearTimeout(timer);
  }, [bookmarkToast]);

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

  // Hotkeys stay armed while the permission overlay is up: the overlay blocks clicks, but the
  // bookmarks manager is independent of indexing and must remain reachable.
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
    // Same native toggle as the menu item and the in-app shortcut.
    toggleBookmarks: () => void invoke('toggle_bookmarks_window'),
    saveCurrentSearchAsBookmark: handleSaveCurrentSearchAsBookmark,
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

  // Right-clicking empty space in the results pane opens the per-tab sort menu. Rows and the
  // header keep their own context menus.
  const handleResultsContextMenu = useCallback(
    (event: ReactMouseEvent<HTMLDivElement>) => {
      if (activeTab !== 'files') {
        return;
      }
      const target = event.target as HTMLElement;
      if (target.closest('.row, .icon-grid-item, .header-row-container, .sort-menu')) {
        return;
      }
      event.preventDefault();
      setSortMenuPoint({ x: event.clientX, y: event.clientY });
    },
    [activeTab],
  );

  // Launch-list paging: only the empty-query view pages, because a real search result set is
  // already complete (and re-ranking would mean `stat`-ing files on every scroll).
  const pagingRef = useRef(false);
  const isLaunchList = useMemo(
    () =>
      !currentQuery &&
      !currentDirectoryQuery &&
      !searchTabs.activeTab.filterQuery &&
      results.length > 0,
    [currentDirectoryQuery, currentQuery, results.length, searchTabs.activeTab.filterQuery],
  );

  const handleLoadMore = useCallback(() => {
    if (!isLaunchList || pagingRef.current) {
      return;
    }
    pagingRef.current = true;
    void invoke<number[]>('get_recent_nodes', {
      offset: results.length,
      limit: RECENT_PAGE_SIZE,
    })
      .then((page) => {
        if (page.length === 0) {
          return;
        }
        // The backend re-ranks the same bounded scan on every page request, so a page may
        // overlap what is already shown; drop anything already present.
        const existing = new Set<SlabIndex>(results);
        const fresh = (page as SlabIndex[]).filter((index) => !existing.has(index));
        appendResults(fresh);
      })
      .catch((error) => {
        console.error('Failed to load more recent files', error);
      })
      .finally(() => {
        pagingRef.current = false;
      });
  }, [appendResults, isLaunchList, results.length]);

  const handleSortMenuSelect = useCallback(
    (selection: { key: SortKey; direction: SortDirection } | null) => {
      setSortState(selection);
    },
    [setSortState],
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

  const renderGridItem = useCallback(
    (rowIndex: number, item: SearchResultItem | undefined, itemStyle: CSSProperties) => {
      if (!item) {
        return (
          <div
            key={`grid-placeholder-${rowIndex}`}
            className="icon-grid-item icon-grid-item--loading"
            style={itemStyle}
          />
        );
      }

      return (
        <IconGridItem
          key={item.path}
          item={item}
          rowIndex={rowIndex}
          style={itemStyle}
          isSelected={selectedIndexSet.has(rowIndex)}
          selectedPathsForDrag={selectedPaths}
          onSelect={handleRowSelect}
          onOpen={openResultPath}
          onContextMenu={handleRowContextMenu}
        />
      );
    },
    [handleRowContextMenu, handleRowSelect, selectedIndexSet, selectedPaths],
  );

  const displayState: DisplayState = (() => {
    if (!initialFetchCompleted) return 'loading';
    if (indexing) return 'indexing';
    if (unavailableScope) return 'scope-unavailable';
    if (showLoadingUI) return 'loading';
    if (searchError) return 'error';
    if (results.length === 0) return 'empty';
    return 'results';
  })();
  const searchErrorMessage =
    typeof searchError === 'string' ? searchError : (searchError?.message ?? null);
  // While the index is building, show what is being scanned instead of a bare spinner.
  const scannedCount = scannedFiles.toLocaleString();
  const stateMessage = !indexing
    ? searchErrorMessage
    : indexingScope
      ? t('stateDisplay.scopeIndexingMessage', {
          scope: indexingScope,
          formatted: scannedCount,
        })
      : t('stateDisplay.indexingProgress', { formatted: scannedCount });
  const stateTitle = indexing && indexingScope ? t('stateDisplay.scopeIndexingTitle') : null;

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

  const filterLabelById = useMemo(
    () =>
      new Map(
        searchFilters.filters.map((filter) => {
          // Built-in presets keep their translated labels; user entries use their own name.
          const preset = DEFAULT_FILTER_PRESETS.find((candidate) => candidate.name === filter.name);
          const labelKey = preset?.labelKey;
          return [
            filter.id,
            labelKey ? t(`search.filters.presets.${labelKey}`) : filter.name,
          ] as const;
        }),
      ),
    [searchFilters.filters, t],
  );

  const filterOptions = useMemo(
    () =>
      searchFilters.filters.map((filter) => ({
        id: filter.id,
        label: filterLabelById.get(filter.id) ?? filter.name,
        search: filter.search,
      })),
    [filterLabelById, searchFilters.filters],
  );

  // Selecting a preset updates the active tab's filter so switching tabs restores it.
  const handleSelectFilter = useCallback(
    (id: string | null) => {
      searchFilters.selectFilter(id);
      const filter = searchFilters.filters.find((candidate) => candidate.id === id);
      searchTabs.setActiveTabFilter(filter?.search ?? '');
      queueFilterQuery(filter?.search ?? '', { immediate: true });
    },
    [queueFilterQuery, searchFilters, searchTabs],
  );

  // Points the dropdown at the preset matching `filterSearch` without touching any tab.
  const selectFilterPreset = useCallback(
    (filterSearch: string) => {
      const match = searchFilters.filters.find(
        (filter) => filter.search.trim() === filterSearch.trim(),
      );
      searchFilters.selectFilter(match?.id ?? null);
    },
    [searchFilters],
  );

  const openBookmark = useCallback(
    (bookmark: Bookmark, mode: BookmarkOpenMode) => {
      // Bookmarks carry their own filter: opening one replaces both the text query and the
      // filter in the target tab (a new tab starts from that filter too).
      if (mode === 'new') {
        // The filter rides along on the new tab; writing it through `setActiveTabFilter`
        // here would land on the *previous* active tab, because `createTab`'s state update
        // has not been committed yet. That is what used to rewrite the other tab's filter.
        searchTabs.createTab(bookmark.query, bookmark.filterSearch, '');
        selectFilterPreset(bookmark.filterSearch);
        // A new tab starts without a folder scope. Going through `queueSearchWithFilter`
        // would keep the previous tab's scope in the shared search params, so the tab would
        // look unscoped while actually searching a folder.
        queueTabSearch(
          { query: bookmark.query, filterQuery: bookmark.filterSearch, directoryQuery: '' },
          { immediate: true },
        );
        return;
      }

      searchTabs.setActiveTabQuery(bookmark.query);
      searchTabs.setActiveTabFilter(bookmark.filterSearch);
      selectFilterPreset(bookmark.filterSearch);
      // Reuse the active tab's own scope: opening a bookmark here is scoped like the tab.
      queueTabSearch(
        {
          query: bookmark.query,
          filterQuery: bookmark.filterSearch,
          directoryQuery: searchTabs.activeTab.directoryQuery,
        },
        { immediate: true },
      );
    },
    [queueTabSearch, searchTabs, selectFilterPreset],
  );

  useEffect(() => {
    if (typeof window === 'undefined') {
      return;
    }
    // Both the in-app shortcut (Cmd+Shift+B in the webview) and the native menu item
    // (same accelerator) can deliver this event. When macOS routes the key to the menu the
    // webview normally does not see it, but a double delivery would toggle twice and look
    // like "the panel never opens", so identical events in quick succession are ignored.
    let lastHandledAt = 0;
    const open = () => {
      const now = Date.now();
      if (now - lastHandledAt < 250) {
        return;
      }
      lastHandledAt = now;
      // The manager is its own OS window now, so it can be dragged anywhere on screen
      // instead of covering the result list.
      void invoke('toggle_bookmarks_window');
    };
    window.addEventListener(OPEN_BOOKMARKS_EVENT, open);
    return () => window.removeEventListener(OPEN_BOOKMARKS_EVENT, open);
  }, []);

  // The bookmarks window asks the main window to run a search (it cannot search itself).
  // The hook subscribes once no matter how often `openBookmark` changes identity.
  useBookmarkOpenListener(({ bookmark, mode }) => openBookmark(bookmark, mode));

  const handleTabActivate = useCallback(
    (id: string) => {
      const tab = searchTabs.tabs.find((candidate) => candidate.id === id);
      if (!tab || tab.id === searchTabs.activeTabId) {
        return;
      }
      searchTabs.activateTab(id);
      // Restore the tab's own filter. Only the dropdown is touched: writing the filter back
      // into the tab here would target the *previous* active tab, because `activateTab`'s
      // state update has not been committed yet.
      selectFilterPreset(tab.filterQuery);
      // The folder scope belongs to the tab too; restoring only query + filter left a scope
      // typed in the previous tab narrowing every other tab.
      queueTabSearch(
        {
          query: tab.query,
          filterQuery: tab.filterQuery,
          directoryQuery: tab.directoryQuery,
        },
        { immediate: true },
      );
    },
    [queueTabSearch, searchTabs, selectFilterPreset],
  );

  const handleTabAdd = useCallback(() => {
    searchTabs.createTab('', '', '');
    selectFilterPreset('');
    queueTabSearch({ query: '', filterQuery: '', directoryQuery: '' }, { immediate: true });
  }, [queueTabSearch, searchTabs, selectFilterPreset]);

  const handleTabClose = useCallback(
    (id: string, event: ReactMouseEvent) => {
      event.stopPropagation();
      const remaining = searchTabs.tabs.filter((tab) => tab.id !== id);
      const wasActive = id === searchTabs.activeTabId;

      if (remaining.length === 0) {
        searchTabs.closeTab(id);
        queueSearch('', { immediate: true });
        return;
      }

      const closingIndex = searchTabs.tabs.findIndex((tab) => tab.id === id);
      const nextActive = remaining[Math.min(closingIndex, remaining.length - 1)];

      searchTabs.closeTab(id);

      if (wasActive) {
        selectFilterPreset(nextActive.filterQuery);
        queueTabSearch(
          {
            query: nextActive.query,
            filterQuery: nextActive.filterQuery,
            directoryQuery: nextActive.directoryQuery,
          },
          { immediate: true },
        );
      }
    },
    [queueTabSearch, searchTabs, selectFilterPreset],
  );

  const handleQueryInputChange = useCallback(
    (event: ChangeEvent<HTMLInputElement>) => {
      const value = event.target.value;
      searchTabs.setActiveTabQuery(value);
      onQueryChange(event);
    },
    [onQueryChange, searchTabs],
  );

  return (
    <>
      <main
        className="container"
        // The bookmarks panel is non-modal (results stay visible and clickable), so it must
        // not be hidden from assistive tech while open.
        aria-hidden={showFullDiskAccessOverlay || isPreferencesOpen || isFiltersOpen}
      >
        {activeTab === 'files' ? (
          <SearchTabsBar
            tabs={searchTabs.tabs}
            activeTabId={searchTabs.activeTabId}
            onActivate={handleTabActivate}
            onClose={handleTabClose}
            onAdd={handleTabAdd}
          />
        ) : null}
        <SearchBar
          inputRef={searchInputRef}
          placeholder={searchPlaceholder}
          ariaLabel={searchAriaLabel}
          value={searchInputValue}
          onChange={handleQueryInputChange}
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
          filterOptions={filterOptions}
          filterValue={searchFilters.selectedId}
          filterLabel={t('search.filters.all')}
          filterManageLabel={t('search.filters.manage')}
          onSelectFilter={handleSelectFilter}
          onManageFilters={() => setIsFiltersOpen(true)}
          onFocus={handleSearchFocus}
          onBlur={handleSearchBlur}
        />
        <div
          className={resultsContainerClassName}
          style={containerStyle}
          onContextMenu={handleResultsContextMenu}
        >
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
            <>
              <div className="view-toolbar">
                <div className="view-toolbar__group">
                  <button
                    type="button"
                    className={`view-toolbar__toggle${
                      viewModeControls.viewMode === 'list' ? ' is-active' : ''
                    }`}
                    onClick={() => viewModeControls.setViewMode('list')}
                    title={t('view.list')}
                  >
                    ☰
                  </button>
                  <button
                    type="button"
                    className={`view-toolbar__toggle${
                      viewModeControls.viewMode === 'grid' ? ' is-active' : ''
                    }`}
                    onClick={() => viewModeControls.setViewMode('grid')}
                    title={t('view.grid')}
                  >
                    ▦
                  </button>
                </div>
                {viewModeControls.viewMode === 'grid' ? (
                  <div className="view-toolbar__group">
                    {(['small', 'medium', 'large', 'xlarge'] as IconSizePreset[]).map((size) => (
                      <button
                        key={size}
                        type="button"
                        className={`view-toolbar__size${
                          viewModeControls.iconSize === size ? ' is-active' : ''
                        }`}
                        onClick={() => viewModeControls.setIconSize(size)}
                        title={t(`view.iconSizes.${size}`)}
                      >
                        {t(`view.iconSizes.${size}`)}
                      </button>
                    ))}
                  </div>
                ) : null}
                {activeSelection ? (
                  <div className="view-toolbar__meta">
                    <span className="view-toolbar__meta-item">
                      <span className="view-toolbar__meta-label">{t('view.created')}</span>
                      <span className="view-toolbar__meta-value">
                        {formatTimestamp(activeSelection.item.ctime) ?? '—'}
                      </span>
                    </span>
                    <span className="view-toolbar__meta-item">
                      <span className="view-toolbar__meta-label">{t('view.modified')}</span>
                      <span className="view-toolbar__meta-value">
                        {formatTimestamp(activeSelection.item.mtime) ?? '—'}
                      </span>
                    </span>
                  </div>
                ) : null}
              </div>
              <FilesTabContent
                headerRef={headerRef}
                onResizeStart={onResizeStart}
                onHeaderContextMenu={showFilesHeaderContextMenu}
                displayState={displayState}
                stateMessage={stateMessage}
                stateTitle={stateTitle}
                unavailableScope={unavailableScope}
                currentQuery={currentQuery}
                currentDirectoryQuery={currentDirectoryQuery}
                resultsViewRef={virtualListRef}
                results={displayedResults}
                dataResultsVersion={resultsVersion}
                displayedResultsVersion={displayedResultsVersion}
                rowHeight={fontSettings.rowHeight}
                fontSize={fontSettings.fontSize}
                overscan={OVERSCAN_ROW_COUNT}
                renderRow={renderRow}
                renderGridItem={renderGridItem}
                onScrollSync={handleHorizontalSync}
                sortState={sortState}
                onSortToggle={handleSortToggle}
                sortDisabled={sortButtonsDisabled}
                sortDisabledTooltip={sortDisabledTooltip}
                viewMode={viewModeControls.viewMode}
                iconSize={viewModeControls.iconSize}
                onNearEnd={isLaunchList ? handleLoadMore : undefined}
              />
            </>
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
          selection={selectionSummary}
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
        fontSize={fontSettings.fontSize}
        defaultFontSize={FONT_SIZE_DEFAULT}
        fontSizeMin={FONT_SIZE_MIN}
        fontSizeMax={FONT_SIZE_MAX}
        onFontSizeChange={fontSettings.setFontSize}
        onFontSizeReset={fontSettings.resetFontSize}
      />
      {showFullDiskAccessOverlay && (
        <PermissionOverlay
          title={t('app.fullDiskAccess.title')}
          description={t('app.fullDiskAccess.description')}
          steps={permissionSteps}
          statusMessage={overlayStatusMessage}
          onRequestPermission={requestFullDiskAccessPermission}
          onRetry={() => void recheckFullDiskAccess()}
          retryLabel={t('app.fullDiskAccess.retry')}
          autoDetectHint={t('app.fullDiskAccess.autoDetect')}
          disabled={isCheckingFullDiskAccess}
          actionLabel={openSettingsLabel}
        />
      )}
      {sortMenuPoint ? (
        <SortMenu
          sortState={sortState}
          onSelect={handleSortMenuSelect}
          labels={{
            sortBySize: t('sortMenu.bySize'),
            sortByCreated: t('sortMenu.byCreated'),
            sortByModified: t('sortMenu.byModified'),
            sortByName: t('sortMenu.byName'),
            ascending: t('sortMenu.ascending'),
            descending: t('sortMenu.descending'),
            clear: t('sortMenu.clear'),
          }}
          x={sortMenuPoint.x}
          y={sortMenuPoint.y}
          onClose={() => setSortMenuPoint(null)}
        />
      ) : null}
      {openWithAnchor ? (
        <OpenWithMenu
          path={openWithAnchor.path}
          x={openWithAnchor.x}
          y={openWithAnchor.y}
          labels={{
            title: t('contextMenu.openWith'),
            loading: t('contextMenu.openWithLoading'),
            empty: t('contextMenu.openWithEmpty'),
            chooseOther: t('contextMenu.openWithOther'),
            defaultSuffix: t('contextMenu.openWithDefault'),
          }}
          onClose={closeOpenWith}
        />
      ) : null}
      {bookmarkToast ? (
        <div className="app-toast" role="status">
          {bookmarkToast}
        </div>
      ) : null}
      <FiltersOverlay
        open={isFiltersOpen}
        onClose={() => setIsFiltersOpen(false)}
        controller={searchFilters}
      />
    </>
  );
}

export default App;
