import { useRef, useCallback, useEffect, useMemo, useState } from 'react';
import type {
  ChangeEvent,
  CSSProperties,
  KeyboardEvent as ReactKeyboardEvent,
  MouseEvent as ReactMouseEvent,
} from 'react';
import './App.css';
import { FileRowRenderer } from './components/FileRowRenderer';
import { SearchBar } from './components/SearchBar';
import { FilesTabContent } from './components/FilesTabContent';
import { PermissionOverlay } from './components/PermissionOverlay';
import PreferencesOverlay from './components/PreferencesOverlay';
import StatusBar from './components/StatusBar';
import type { StatusTabKey } from './components/StatusBar';
import type { SearchResultItem } from './types/search';
import type { AppLifecycleStatus, StatusBarUpdatePayload } from './types/ipc';
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
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { primaryMonitor, getCurrentWindow } from '@tauri-apps/api/window';
import type { DragDropEvent } from '@tauri-apps/api/window';
import type { Event as TauriEvent } from '@tauri-apps/api/event';
import type { UnlistenFn } from '@tauri-apps/api/event';
import { useTranslation } from 'react-i18next';
import { useFullDiskAccessPermission } from './hooks/useFullDiskAccessPermission';
import { OPEN_PREFERENCES_EVENT } from './constants/appEvents';
import type { DisplayState } from './components/StateDisplay';
import { openResultPath } from './utils/openResultPath';
import { useStableEvent } from './hooks/useStableEvent';
import { setWatchConfig } from './utils/watchConfig';
import { getStoredTrayIconEnabled, persistTrayIconEnabled } from './trayIconPreference';
import { setTrayEnabled } from './tray';
import { useWatchRoot } from './hooks/useWatchRoot';
import { useIgnorePaths } from './hooks/useIgnorePaths';
import { applyThemePreference, persistThemePreference } from './theme';
import { getBrowserLanguage } from './i18n/config';

type ActiveTab = StatusTabKey;

type QuickLookKeydownPayload = {
  keyCode: number;
  characters?: string | null;
  modifiers: {
    shift: boolean;
    control: boolean;
    option: boolean;
    command: boolean;
  };
};

type WindowGeometry = {
  windowOrigin: { x: number; y: number };
  mainScreenHeight: number;
};

const isEditableTarget = (target: EventTarget | null): boolean => {
  const element = target as HTMLElement | null;
  if (!element) return false;
  const tagName = element.tagName;
  return tagName === 'INPUT' || tagName === 'TEXTAREA' || element.isContentEditable;
};

const QUICK_LOOK_KEYCODE_DOWN = 125;
const QUICK_LOOK_KEYCODE_UP = 126;
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
  const [activeTab, setActiveTab] = useState<ActiveTab>('files');
  // Drives the inactive selection styling for the files panel via data-window-focused.
  const [isWindowFocused, setIsWindowFocused] = useState<boolean>(() => {
    return document.hasFocus();
  });
  const [isSearchFocused, setIsSearchFocused] = useState(false);
  const eventsPanelRef = useRef<FSEventsPanelHandle | null>(null);
  const headerRef = useRef<HTMLDivElement | null>(null);
  const virtualListRef = useRef<VirtualListHandle | null>(null);
  const searchInputRef = useRef<HTMLInputElement | null>(null);
  const isMountedRef = useRef(false);
  const keyboardStateRef = useRef<{ activeTab: ActiveTab; activePath: string | null }>({
    activeTab,
    activePath: null,
  });
  const {
    handleInputChange: updateHistoryFromInput,
    navigate: navigateSearchHistory,
    ensureTailValue: ensureHistoryBuffer,
    resetCursorToTail,
  } = useSearchHistory({ maxEntries: MAX_SEARCH_HISTORY_ENTRIES });
  const { colWidths, onResizeStart, autoFitColumns } = useColumnResize();
  const { caseSensitive } = searchParams;
  const { eventColWidths, onEventResizeStart, autoFitEventColumns } = useEventColumnWidths();
  const { filteredEvents, eventFilterQuery, setEventFilterQuery } = useRecentFSEvents({
    caseSensitive,
    isActive: activeTab === 'events',
  });
  const { t, i18n } = useTranslation();
  const {
    sortState,
    displayedResults,
    displayedResultsVersion,
    sortThreshold,
    setSortThreshold,
    canSort,
    isSorting,
    sortDisabledTooltip,
    sortButtonsDisabled,
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

  const getQuickLookPaths = useCallback(
    () => (activeTab === 'files' ? selectedPaths : []),
    [activeTab, selectedPaths],
  );
  // Quick Look controller keeps preview panel in sync with whichever rows are currently selected.
  const { toggleQuickLook, updateQuickLook, closeQuickLook } = useQuickLook({
    getPaths: getQuickLookPaths,
  });
  const triggerQuickLook = useStableEvent(toggleQuickLook);

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
  const navigateSelection = useStableEvent(moveSelection);

  const {
    status: fullDiskAccessStatus,
    isChecking: isCheckingFullDiskAccess,
    requestPermission: requestFullDiskAccessPermission,
  } = useFullDiskAccessPermission();
  const { watchRoot, setWatchRoot, defaultWatchRoot } = useWatchRoot();
  const logicStartedRef = useRef(false);
  const { ignorePaths, setIgnorePaths, defaultIgnorePaths } = useIgnorePaths();
  const [isPreferencesOpen, setIsPreferencesOpen] = useState(false);
  const [trayIconEnabled, setTrayIconEnabled] = useState<boolean>(() => getStoredTrayIconEnabled());
  const [preferencesResetToken, setPreferencesResetToken] = useState(0);

  useEffect(() => {
    persistTrayIconEnabled(trayIconEnabled);
    void setTrayEnabled(trayIconEnabled);
  }, [trayIconEnabled]);

  const activePath =
    activeRowIndex !== null
      ? (virtualListRef.current?.getItem?.(activeRowIndex)?.path ?? null)
      : null;
  useEffect(() => {
    keyboardStateRef.current.activeTab = activeTab;
  }, [activeTab]);
  useEffect(() => {
    keyboardStateRef.current.activePath = activePath;
  }, [activePath]);

  useEffect(() => {
    if (isCheckingFullDiskAccess) {
      return;
    }
    if (fullDiskAccessStatus !== 'granted') {
      return;
    }

    if (!watchRoot) {
      return;
    }
    if (logicStartedRef.current) {
      return;
    }

    logicStartedRef.current = true;
    void invoke('start_logic', { watchRoot, ignorePaths });
  }, [fullDiskAccessStatus, ignorePaths, isCheckingFullDiskAccess, watchRoot]);

  const refreshSearchResults = useCallback(() => {
    queueSearch(currentQuery, { immediate: true });
  }, [currentQuery, queueSearch]);

  const applyWatchConfig = useCallback(
    (nextWatchRoot: string, nextIgnorePaths: string[]) => {
      setWatchRoot(nextWatchRoot);
      setIgnorePaths(nextIgnorePaths);
      if (logicStartedRef.current && nextWatchRoot) {
        void setWatchConfig({ watchRoot: nextWatchRoot, ignorePaths: nextIgnorePaths });
      }
      refreshSearchResults();
    },
    [refreshSearchResults, setIgnorePaths, setWatchRoot],
  );

  const handleWatchConfigChange = useCallback(
    (next: { watchRoot: string; ignorePaths: string[] }) => {
      applyWatchConfig(next.watchRoot, next.ignorePaths);
    },
    [applyWatchConfig],
  );

  const handleResetPreferences = useCallback(() => {
    setTrayIconEnabled(false);
    persistThemePreference('system');
    applyThemePreference('system');
    const nextLanguage = getBrowserLanguage();
    void i18n.changeLanguage(nextLanguage);
    setPreferencesResetToken((token) => token + 1);
  }, [i18n]);

  const focusSearchInput = useCallback(() => {
    requestAnimationFrame(() => {
      const input = searchInputRef.current;
      if (!input) return;
      input.focus();
      input.select();
    });
  }, []);
  const focusSearchInputStable = useStableEvent(focusSearchInput);
  const handleMetaShortcut = useStableEvent(
    (event: KeyboardEvent, currentTab: ActiveTab, currentPath: string | null) => {
      const key = event.key.toLowerCase();
      if (key === 'f') {
        event.preventDefault();
        focusSearchInputStable();
        return true;
      }

      if (currentTab !== 'files') {
        return false;
      }

      if (key === 'r' && selectedPaths.length > 0) {
        event.preventDefault();
        selectedPaths.forEach((path) => {
          void invoke('open_in_finder', { path });
        });
        return true;
      }

      if (key === 'o' && selectedPaths.length > 0) {
        event.preventDefault();
        selectedPaths.forEach((path) => openResultPath(path));
        return true;
      }

      if (key === 'c' && selectedPaths.length > 0) {
        event.preventDefault();
        void invoke('copy_files_to_clipboard', { paths: selectedPaths }).catch((error) => {
          console.error('Failed to copy files to clipboard', error);
        });
        return true;
      }

      return false;
    },
  );

  const handleFilesNavigation = useStableEvent((event: KeyboardEvent) => {
    const target = event.target as HTMLElement | null;
    if (isEditableTarget(target)) {
      return false;
    }

    const isSpaceKey = event.code === 'Space' || event.key === ' ';
    if (isSpaceKey) {
      if (event.repeat || !selectedIndicesRef.current.length) {
        return true;
      }
      event.preventDefault();
      triggerQuickLook();
      return true;
    }

    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      if (event.altKey || event.ctrlKey || event.metaKey) {
        return true;
      }
      event.preventDefault();
      const delta = event.key === 'ArrowDown' ? 1 : -1;
      navigateSelection(delta, { extend: event.shiftKey });
      return true;
    }

    return false;
  });

  const handleSearchFocus = useCallback(() => {
    setIsSearchFocused(true);
  }, []);

  const handleSearchBlur = useCallback(() => {
    setIsSearchFocused(false);
  }, []);

  useEffect(() => {
    isMountedRef.current = true;
    let unlistenStatus: UnlistenFn | undefined;
    let unlistenLifecycle: UnlistenFn | undefined;
    let unlistenQuickLaunch: UnlistenFn | undefined;

    const setupListeners = async (): Promise<void> => {
      unlistenStatus = await listen<StatusBarUpdatePayload>('status_bar_update', (event) => {
        if (!isMountedRef.current) return;
        const payload = event.payload;
        if (!payload) return;
        const { scannedFiles, processedEvents, rescanErrors } = payload;
        handleStatusUpdate(scannedFiles, processedEvents, rescanErrors);
      });

      unlistenLifecycle = await listen<AppLifecycleStatus>('app_lifecycle_state', (event) => {
        if (!isMountedRef.current) return;
        const status = event.payload;
        if (!status) return;
        setLifecycleState(status);
      });

      unlistenQuickLaunch = await listen('quick_launch', () => {
        if (!isMountedRef.current) return;
        focusSearchInput();
      });
    };

    void setupListeners();

    return () => {
      isMountedRef.current = false;
      unlistenStatus?.();
      unlistenLifecycle?.();
      unlistenQuickLaunch?.();
    };
  }, [focusSearchInput, handleStatusUpdate, setLifecycleState]);

  useEffect(() => {
    focusSearchInput();
  }, [focusSearchInput]);

  useEffect(() => {
    const handleOpenPreferences = () => setIsPreferencesOpen(true);

    window.addEventListener(OPEN_PREFERENCES_EVENT, handleOpenPreferences);
    return () => window.removeEventListener(OPEN_PREFERENCES_EVENT, handleOpenPreferences);
  }, []);

  useEffect(() => {
    if (typeof window === 'undefined') {
      return;
    }
    const handleWindowFocus = () => {
      setIsWindowFocused(true);
    };
    const handleWindowBlur = () => setIsWindowFocused(false);
    window.addEventListener('focus', handleWindowFocus);
    window.addEventListener('blur', handleWindowBlur);
    return () => {
      window.removeEventListener('focus', handleWindowFocus);
      window.removeEventListener('blur', handleWindowBlur);
    };
  }, []);

  useEffect(() => {
    if (typeof document === 'undefined') {
      return;
    }
    document.documentElement.dataset.windowFocused = isWindowFocused ? 'true' : 'false';
  }, [isWindowFocused]);

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
    if (typeof window === 'undefined') {
      return;
    }

    const handleKeyDown = (event: KeyboardEvent) => {
      const { activeTab: currentTab, activePath: currentPath } = keyboardStateRef.current;

      if (event.metaKey && handleMetaShortcut(event, currentTab, currentPath)) {
        return;
      }

      if (currentTab !== 'files') {
        return;
      }

      if (handleFilesNavigation(event)) {
        return;
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [handleMetaShortcut, handleFilesNavigation]);

  useEffect(() => {
    let unlisten: UnlistenFn | null = null;

    const setup = async () => {
      try {
        unlisten = await listen<QuickLookKeydownPayload>('quicklook-keydown', (event) => {
          if (keyboardStateRef.current.activeTab !== 'files') {
            return;
          }

          const payload = event.payload;
          if (!payload || !selectedIndicesRef.current.length) {
            return;
          }

          const { keyCode, modifiers } = payload;
          if (modifiers.command || modifiers.option || modifiers.control) {
            return;
          }

          if (keyCode === QUICK_LOOK_KEYCODE_DOWN) {
            navigateSelection(1, { extend: modifiers.shift });
          } else if (keyCode === QUICK_LOOK_KEYCODE_UP) {
            navigateSelection(-1, { extend: modifiers.shift });
          }
        });
      } catch (error) {
        console.error('Failed to subscribe to Quick Look key events', error);
      }
    };

    void setup();

    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, [navigateSelection]);

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

  const handleWindowDragDrop = useStableEvent((event: TauriEvent<DragDropEvent>) => {
    const payload = event.payload;
    if (payload.type !== 'drop') {
      return;
    }
    const nextValue = payload.paths[0]?.trim();
    if (!nextValue) {
      return;
    }
    const query = `"${nextValue}"`;
    if (activeTab === 'events') {
      setEventFilterQuery(query);
      return;
    }
    queueSearch(query, {
      immediate: true,
      onSearchCommitted: updateHistoryFromInput,
    });
  });

  useEffect(() => {
    let unlisten: UnlistenFn | null = null;
    getCurrentWindow()
      .onDragDropEvent(handleWindowDragDrop)
      .then((unsubscribe) => {
        unlisten = unsubscribe;
      })
      .catch((error) => {
        console.error('Failed to register drag-drop listener', error);
      });

    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, [handleWindowDragDrop]);

  const handleHorizontalSync = useCallback((scrollLeft: number) => {
    // VirtualList drives the scroll position; mirror it onto the sticky header for alignment
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
        <FileRowRenderer
          key={item.path}
          rowIndex={rowIndex}
          item={item}
          style={rowStyle}
          isSelected={selectedIndexSet.has(rowIndex)}
          selectedPaths={selectedPaths}
          caseInsensitive={!caseSensitive}
          highlightTerms={highlightTerms}
          onContextMenu={(event, contextPath) => handleRowContextMenu(event, contextPath, rowIndex)}
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
        resetCursorToTail();
      } else {
        // Switch to files: sync with reducer-managed search state and cancel pending timers
        ensureHistoryBuffer('');
        queueSearch('', { immediate: true });
      }
    },
    [ensureHistoryBuffer, queueSearch, resetCursorToTail, setEventFilterQuery],
  );

  const searchInputValue = activeTab === 'events' ? eventFilterQuery : searchParams.query;

  const containerStyle = {
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
  } as CSSProperties;

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
        onClose={() => setIsPreferencesOpen(false)}
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
