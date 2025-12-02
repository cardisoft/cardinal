import { useRef, useCallback, useEffect, useState } from 'react';
import type { ChangeEvent, CSSProperties, MouseEvent as ReactMouseEvent } from 'react';
import './App.css';
import { FileRow } from './components/FileRow';
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
import { useRemoteSort } from './hooks/useRemoteSort';
import { useSelection } from './hooks/useSelection';
import { ROW_HEIGHT, OVERSCAN_ROW_COUNT } from './constants';
import type { VirtualListHandle } from './components/VirtualList';
import FSEventsPanel from './components/FSEventsPanel';
import type { FSEventsPanelHandle } from './components/FSEventsPanel';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { primaryMonitor, getCurrentWindow } from '@tauri-apps/api/window';
import type { UnlistenFn } from '@tauri-apps/api/event';
import { useTranslation } from 'react-i18next';
import { useFullDiskAccessPermission } from './hooks/useFullDiskAccessPermission';
import { OPEN_PREFERENCES_EVENT } from './constants/appEvents';
import type { DisplayState } from './components/StateDisplay';

type ActiveTab = StatusTabKey;

type QuickLookRect = {
  x: number;
  y: number;
  width: number;
  height: number;
};

type QuickLookItemPayload = {
  path: string;
  rect?: QuickLookRect;
  transitionImage?: string;
};

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

const escapePathForSelector = (value: string): string => {
  return window.CSS.escape(value);
};

const isEditableTarget = (target: EventTarget | null): boolean => {
  const element = target as HTMLElement | null;
  if (!element) return false;
  const tagName = element.tagName;
  return tagName === 'INPUT' || tagName === 'TEXTAREA' || element.isContentEditable;
};

const QUICK_LOOK_KEYCODE_DOWN = 125;
const QUICK_LOOK_KEYCODE_UP = 126;

function App() {
  const {
    state,
    searchParams,
    updateSearchParams,
    queueSearch,
    resetSearchQuery,
    cancelPendingSearches,
    handleStatusUpdate,
    setLifecycleState,
    requestRescan,
  } = useFileSearch();
  const {
    results,
    scannedFiles,
    processedEvents,
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
  const [isWindowFocused, setIsWindowFocused] = useState<boolean>(() => {
    return document.hasFocus();
  });
  const eventsPanelRef = useRef<FSEventsPanelHandle | null>(null);
  const headerRef = useRef<HTMLDivElement | null>(null);
  const virtualListRef = useRef<VirtualListHandle | null>(null);
  const searchInputRef = useRef<HTMLInputElement | null>(null);
  const isMountedRef = useRef(false);
  const { colWidths, onResizeStart, autoFitColumns } = useColumnResize();
  const { caseSensitive } = searchParams;
  const { eventColWidths, onEventResizeStart, autoFitEventColumns } = useEventColumnWidths();
  const { filteredEvents, eventFilterQuery, setEventFilterQuery } = useRecentFSEvents({
    caseSensitive,
  });
  const { t, i18n } = useTranslation();
  const {
    sortState,
    displayedResults,
    sortThreshold,
    setSortThreshold,
    canSort,
    isSorting,
    sortDisabledTooltip,
    sortButtonsDisabled,
    handleSortToggle,
  } = useRemoteSort(results, i18n.language, (limit) => t('sorting.disabled', { limit }));
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
  } = useSelection(displayedResults, virtualListRef);

  const getQuickLookItems = useCallback(async (): Promise<QuickLookItemPayload[]> => {
    if (activeTab !== 'files') {
      return [];
    }

    const paths = selectedPaths;
    if (!paths.length) {
      return [];
    }

    let windowGeometry: WindowGeometry | null | undefined;

    const resolveWindowGeometry = async (): Promise<WindowGeometry | null> => {
      if (windowGeometry !== undefined) {
        return windowGeometry;
      }

      if (typeof window === 'undefined') {
        windowGeometry = null;
        return windowGeometry;
      }

      try {
        const currentWindow = getCurrentWindow();
        const [position, monitor, scaleFactor] = await Promise.all([
          currentWindow.innerPosition(),
          primaryMonitor(),
          currentWindow.scaleFactor(),
        ]);

        if (!monitor) {
          windowGeometry = null;
          return windowGeometry;
        }

        const scale = scaleFactor || monitor.scaleFactor || window.devicePixelRatio || 1;
        windowGeometry = {
          windowOrigin: {
            x: position.x / scale,
            y: position.y / scale,
          },
          mainScreenHeight: monitor.size.height / scale,
        };
      } catch (error) {
        console.warn('Failed to resolve window metrics for Quick Look', error);
        windowGeometry = null;
      }

      return windowGeometry;
    };

    const buildItem = async (path: string): Promise<QuickLookItemPayload> => {
      const selector = `[data-row-path="${escapePathForSelector(path)}"]`;
      const row = document.querySelector<HTMLElement>(selector);
      if (!row) {
        return { path };
      }
      const anchor = row.querySelector<HTMLElement>('.file-icon, .file-icon-placeholder');
      if (!anchor) {
        return { path };
      }
      const iconImage = row.querySelector<HTMLImageElement>('img.file-icon');
      if (!iconImage) {
        return { path };
      }
      const transitionImage = iconImage.src;

      const rect = anchor.getBoundingClientRect();
      const geometry = await resolveWindowGeometry();
      if (!geometry) {
        return { path };
      }

      // This compensates for a coordinate system mismatch on macOS:
      // - `geometry.windowOrigin.y` (from Tauri's `innerPosition`) is relative to the *visible* screen area (below the menu bar).
      // - `geometry.mainScreenHeight` is the *full* screen height.
      // - `window.screen.availTop` provides the height of the menu bar, allowing us to correctly adjust `logicalYTop`
      //   to be relative to the absolute top of the screen for `QLPreviewPanel`'s `sourceFrameOnScreenForPreviewItem`.
      const screenTopOffset = window.screen.availTop ?? 0;

      const logicalX = geometry.windowOrigin.x + rect.left;
      const logicalYTop = geometry.windowOrigin.y + screenTopOffset + rect.top;
      const logicalWidth = rect.width;
      const logicalHeight = rect.height;

      const x = logicalX;
      const y = geometry.mainScreenHeight - (logicalYTop + logicalHeight);

      return {
        path,
        rect: {
          x,
          y,
          width: logicalWidth,
          height: logicalHeight,
        },
        transitionImage,
      };
    };

    const items = await Promise.all(paths.map((path) => buildItem(path)));
    return items;
  }, [activeTab, selectedPaths]);

  const toggleQuickLookPanel = useCallback(() => {
    void (async () => {
      const items = await getQuickLookItems();
      if (!items.length) {
        return;
      }
      try {
        await invoke('toggle_quicklook', { items });
      } catch (error) {
        console.error('Failed to preview file with Quick Look', error);
      }
    })();
  }, [getQuickLookItems]);

  const updateQuickLookPanel = useCallback(() => {
    void (async () => {
      const items = await getQuickLookItems();
      if (!items.length) {
        return;
      }
      try {
        await invoke('update_quicklook', { items });
      } catch (error) {
        console.error('Failed to update Quick Look', error);
      }
    })();
  }, [getQuickLookItems]);

  const {
    showContextMenu: showFilesContextMenu,
    showHeaderContextMenu: showFilesHeaderContextMenu,
  } = useContextMenu(autoFitColumns, toggleQuickLookPanel);

  const {
    showContextMenu: showEventsContextMenu,
    showHeaderContextMenu: showEventsHeaderContextMenu,
  } = useContextMenu(autoFitEventColumns);

  const {
    status: fullDiskAccessStatus,
    isChecking: isCheckingFullDiskAccess,
    requestPermission: requestFullDiskAccessPermission,
  } = useFullDiskAccessPermission();
  const [isPreferencesOpen, setIsPreferencesOpen] = useState(false);

  const activePath =
    activeRowIndex !== null
      ? (virtualListRef.current?.getItem?.(activeRowIndex)?.path ?? null)
      : null;

  useEffect(() => {
    if (isCheckingFullDiskAccess) {
      return;
    }
    if (fullDiskAccessStatus !== 'granted') {
      return;
    }

    void invoke('start_logic');
  }, [fullDiskAccessStatus, isCheckingFullDiskAccess]);

  const focusSearchInput = useCallback(() => {
    requestAnimationFrame(() => {
      const input = searchInputRef.current;
      if (!input) return;
      input.focus();
      input.select();
    });
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
        const { scannedFiles, processedEvents } = payload;
        handleStatusUpdate(scannedFiles, processedEvents);
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
    const handleWindowFocus = () => setIsWindowFocused(true);
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
    if (activeTab === 'files') {
      return;
    }

    // Close Quick Look when leaving the files tab
    invoke('close_quicklook').catch((error) => {
      console.error('Failed to close Quick Look', error);
    });
  }, [activeTab]);

  useEffect(() => {
    if (activeTab !== 'files') {
      return;
    }

    const handleKeyDown = (event: KeyboardEvent) => {
      const isSpaceKey = event.code === 'Space' || event.key === ' ';
      if (!isSpaceKey || event.repeat) {
        return;
      }

      const target = event.target as HTMLElement | null;
      if (isEditableTarget(target)) {
        return;
      }

      if (!selectedIndices.length) {
        return;
      }

      event.preventDefault();
      toggleQuickLookPanel();
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [activeTab, toggleQuickLookPanel, selectedIndices]);

  useEffect(() => {
    if (activeTab !== 'files' || !selectedIndices.length) {
      return;
    }

    updateQuickLookPanel();
  }, [activeTab, selectedIndices, updateQuickLookPanel]);

  useEffect(() => {
    if (activeTab !== 'files') {
      return;
    }

    const handleArrowNavigation = (event: KeyboardEvent) => {
      if (event.altKey || event.metaKey || event.ctrlKey) {
        return;
      }

      if (event.key !== 'ArrowDown' && event.key !== 'ArrowUp') {
        return;
      }

      if (isEditableTarget(event.target)) {
        return;
      }

      event.preventDefault();
      const delta = event.key === 'ArrowDown' ? 1 : -1;
      moveSelection(delta);
    };

    window.addEventListener('keydown', handleArrowNavigation);
    return () => window.removeEventListener('keydown', handleArrowNavigation);
  }, [activeTab, moveSelection]);

  useEffect(() => {
    const handleGlobalShortcuts = (event: KeyboardEvent) => {
      if (!event.metaKey) {
        return;
      }

      const key = event.key.toLowerCase();

      if (key === 'f') {
        event.preventDefault();
        focusSearchInput();
        return;
      }

      if (key === 'r') {
        if (activeTab !== 'files' || !activePath) {
          return;
        }
        event.preventDefault();
        invoke('open_in_finder', { path: activePath }).catch((error) => {
          console.error('Failed to reveal file in Finder', error);
        });
        return;
      }

      if (key === 'c') {
        if (activeTab !== 'files' || !activePath) {
          return;
        }
        event.preventDefault();
        if (typeof navigator !== 'undefined' && navigator.clipboard?.writeText) {
          navigator.clipboard.writeText(activePath).catch((error) => {
            console.error('Failed to copy file path', error);
          });
        }
      }
    };

    window.addEventListener('keydown', handleGlobalShortcuts);
    return () => window.removeEventListener('keydown', handleGlobalShortcuts);
  }, [focusSearchInput, activeTab, activePath]);

  useEffect(() => {
    let unlisten: UnlistenFn | null = null;

    const setup = async () => {
      try {
        unlisten = await listen<QuickLookKeydownPayload>('quicklook-keydown', (event) => {
          if (activeTab !== 'files') {
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
            moveSelection(1);
          } else if (keyCode === QUICK_LOOK_KEYCODE_UP) {
            moveSelection(-1);
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
  }, [activeTab, moveSelection]);

  useEffect(() => {
    if (activeRowIndex == null) {
      return;
    }

    const list = virtualListRef.current;
    if (!list) {
      return;
    }

    list.scrollToRow?.(activeRowIndex, 'nearest');
  }, [activeRowIndex]);

  useEffect(() => {
    clearSelection();
  }, [results, clearSelection]);

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

  const handleRowContextMenu = useCallback(
    (event: ReactMouseEvent<HTMLDivElement>, path: string, rowIndex: number) => {
      if (!selectedIndices.includes(rowIndex)) {
        selectSingleRow(rowIndex);
      }
      if (path) {
        showFilesContextMenu(event, path);
      }
    },
    [selectedIndices, selectSingleRow, showFilesContextMenu],
  );

  const handleRowOpen = useCallback((path: string) => {
    if (!path) {
      return;
    }
    invoke('open_path', { path }).catch((error) => {
      console.error('Failed to open file', error);
    });
  }, []);

  const renderRow = useCallback(
    (rowIndex: number, item: SearchResultItem | undefined, rowStyle: CSSProperties) => {
      const path = item?.path;
      const isSelected = selectedIndices.includes(rowIndex);

      return (
        <FileRow
          key={item?.path ?? rowIndex}
          item={item}
          rowIndex={rowIndex}
          style={{ ...rowStyle, width: 'var(--columns-total)' }} // Enforce column width CSS vars for virtualization rows
          onContextMenu={(event, contextPath) => handleRowContextMenu(event, contextPath, rowIndex)}
          onSelect={handleRowSelect}
          onOpen={handleRowOpen}
          isSelected={isSelected}
          selectedPathsForDrag={selectedPaths}
          caseInsensitive={!caseSensitive}
          highlightTerms={highlightTerms}
        />
      );
    },
    [
      handleRowContextMenu,
      handleRowSelect,
      handleRowOpen,
      selectedIndices,
      selectedPaths,
      caseSensitive,
      highlightTerms,
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
    '--columns-events-total': `${eventColWidths.name + eventColWidths.path + eventColWidths.time}px`,
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

  return (
    <>
      <main className="container" aria-hidden={showFullDiskAccessOverlay || isPreferencesOpen}>
        <SearchBar
          inputRef={searchInputRef}
          placeholder={searchPlaceholder}
          onChange={onQueryChange}
          caseSensitive={caseSensitive}
          onToggleCaseSensitive={onToggleCaseSensitive}
          caseSensitiveLabel={caseSensitiveLabel}
        />
        <div className="results-container" style={containerStyle}>
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
              rowHeight={ROW_HEIGHT}
              overscan={OVERSCAN_ROW_COUNT}
              renderRow={renderRow}
              onScrollSync={handleHorizontalSync}
              sortState={sortState}
              onSortToggle={handleSortToggle}
              sortDisabled={sortButtonsDisabled}
              sortIndicatorMode="triangle"
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
        />
      </main>
      <PreferencesOverlay
        open={isPreferencesOpen}
        onClose={() => setIsPreferencesOpen(false)}
        sortThreshold={sortThreshold}
        onSortThresholdChange={setSortThreshold}
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
