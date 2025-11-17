import { useRef, useCallback, useEffect, useState } from 'react';
import type { ChangeEvent, CSSProperties, MouseEvent as ReactMouseEvent } from 'react';
import './App.css';
import { FileRow } from './components/FileRow';
import { SearchBar } from './components/SearchBar';
import { FilesTabContent } from './components/FilesTabContent';
import { PermissionOverlay } from './components/PermissionOverlay';
import StatusBar from './components/StatusBar';
import type { StatusTabKey } from './components/StatusBar';
import type { SearchResultItem } from './types/search';
import type { AppLifecycleStatus, StatusBarUpdatePayload } from './types/ipc';
import { useColumnResize } from './hooks/useColumnResize';
import { useContextMenu } from './hooks/useContextMenu';
import { useFileSearch } from './hooks/useFileSearch';
import { useEventColumnWidths } from './hooks/useEventColumnWidths';
import { useRecentFSEvents } from './hooks/useRecentFSEvents';
import { ROW_HEIGHT, OVERSCAN_ROW_COUNT } from './constants';
import type { VirtualListHandle } from './components/VirtualList';
import FSEventsPanel from './components/FSEventsPanel';
import type { FSEventsPanelHandle } from './components/FSEventsPanel';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { UnlistenFn } from '@tauri-apps/api/event';
import { useTranslation } from 'react-i18next';
import type { SlabIndex } from './types/slab';
import { useFullDiskAccessPermission } from './hooks/useFullDiskAccessPermission';

type ActiveTab = StatusTabKey;

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
  type SelectedRow = {
    index: number;
    slab: SlabIndex | null;
    path: string | null;
  };
  const [selectedRow, setSelectedRow] = useState<SelectedRow | null>(null);
  const [isWindowFocused, setIsWindowFocused] = useState<boolean>(() => {
    if (typeof document === 'undefined') {
      return true;
    }
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
  const { t } = useTranslation();

  const selectRow = useCallback(
    (rowIndex: number, pathOverride?: string | null) => {
      if (rowIndex < 0 || rowIndex >= results.length) {
        return;
      }

      const resolvedPath =
        pathOverride ?? virtualListRef.current?.getItem?.(rowIndex)?.path ?? null;
      const nextSlab = results[rowIndex] ?? null;

      setSelectedRow((prev) => {
        if (
          prev &&
          prev.index === rowIndex &&
          prev.slab === nextSlab &&
          prev.path === resolvedPath
        ) {
          return prev;
        }
        return {
          index: rowIndex,
          slab: nextSlab,
          path: resolvedPath,
        };
      });
    },
    [results],
  );

  const handleRowSelect = useCallback(
    (path: string, rowIndex: number) => {
      selectRow(rowIndex, path);
    },
    [selectRow],
  );

  const {
    showContextMenu: showFilesContextMenu,
    showHeaderContextMenu: showFilesHeaderContextMenu,
  } = useContextMenu(autoFitColumns);

  const {
    showContextMenu: showEventsContextMenu,
    showHeaderContextMenu: showEventsHeaderContextMenu,
  } = useContextMenu(autoFitEventColumns);

  const {
    status: fullDiskAccessStatus,
    isChecking: isCheckingFullDiskAccess,
    requestPermission: requestFullDiskAccessPermission,
  } = useFullDiskAccessPermission();
  const selectedIndex = selectedRow?.index ?? null;
  const selectedPath = selectedRow?.path ?? null;

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
      setSelectedRow(null);
    }
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
      if (target) {
        const tagName = target.tagName;
        if (tagName === 'INPUT' || tagName === 'TEXTAREA' || target.isContentEditable) {
          return;
        }
      }

      if (!selectedPath) {
        return;
      }

      event.preventDefault();
      invoke('preview_with_quicklook', { path: selectedPath }).catch((error) => {
        console.error('Failed to preview file with Quick Look', error);
      });
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [activeTab, selectedPath]);

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

      const target = event.target as HTMLElement | null;
      if (target) {
        const tagName = target.tagName;
        if (tagName === 'INPUT' || tagName === 'TEXTAREA' || target.isContentEditable) {
          return;
        }
      }

      if (!results.length) {
        return;
      }

      event.preventDefault();

      const delta = event.key === 'ArrowDown' ? 1 : -1;
      const fallbackIndex = delta > 0 ? -1 : results.length;
      const baseIndex = selectedIndex ?? fallbackIndex;
      const nextIndex = Math.min(Math.max(baseIndex + delta, 0), results.length - 1);

      if (nextIndex === selectedIndex) {
        return;
      }

      selectRow(nextIndex);
    };

    window.addEventListener('keydown', handleArrowNavigation);
    return () => window.removeEventListener('keydown', handleArrowNavigation);
  }, [activeTab, results, selectRow, selectedIndex]);

  useEffect(() => {
    const handleGlobalShortcuts = (event: KeyboardEvent) => {
      if (!(event.metaKey || event.ctrlKey) || event.altKey || event.shiftKey) {
        return;
      }

      const key = event.key.toLowerCase();

      if (key === 'f') {
        event.preventDefault();
        focusSearchInput();
        return;
      }

      if (key === 'r') {
        if (activeTab !== 'files' || !selectedPath) {
          return;
        }
        event.preventDefault();
        invoke('open_in_finder', { path: selectedPath }).catch((error) => {
          console.error('Failed to reveal file in Finder', error);
        });
        return;
      }

      if (key === 'c') {
        if (activeTab !== 'files' || !selectedPath) {
          return;
        }
        event.preventDefault();
        if (typeof navigator !== 'undefined' && navigator.clipboard?.writeText) {
          navigator.clipboard.writeText(selectedPath).catch((error) => {
            console.error('Failed to copy file path', error);
          });
        }
      }
    };

    window.addEventListener('keydown', handleGlobalShortcuts);
    return () => window.removeEventListener('keydown', handleGlobalShortcuts);
  }, [focusSearchInput, activeTab, selectedPath]);

  useEffect(() => {
    if (selectedIndex == null) {
      return;
    }

    const list = virtualListRef.current;
    if (!list) {
      return;
    }

    list.scrollToRow?.(selectedIndex, 'nearest');

    let cancelled = false;
    const ensureResult = list.ensureRangeLoaded?.(selectedIndex, selectedIndex);
    const ensurePromise = ensureResult instanceof Promise ? ensureResult : Promise.resolve();

    void ensurePromise.then(() => {
      if (cancelled) {
        return;
      }

      const item = list.getItem?.(selectedIndex);
      if (!item?.path) {
        return;
      }

      setSelectedRow((prev) => {
        if (!prev || prev.index !== selectedIndex || prev.path === item.path) {
          return prev;
        }
        return { ...prev, path: item.path };
      });
    });

    return () => {
      cancelled = true;
    };
  }, [selectedIndex]);

  useEffect(() => {
    if (!results.length) {
      setSelectedRow(null);
      return;
    }

    setSelectedRow((prev) => {
      if (!prev) {
        return prev;
      }

      const { slab, index, path } = prev;
      let nextIndex = -1;

      if (slab != null) {
        nextIndex = results.findIndex((candidate) => candidate === slab);
      }

      if (nextIndex === -1) {
        nextIndex = Math.min(index, results.length - 1);
      }

      if (nextIndex < 0) {
        return null;
      }

      const nextSlab = results[nextIndex] ?? null;
      if (nextIndex === index && nextSlab === slab) {
        return prev;
      }

      const list = virtualListRef.current;
      const resolvedPath = list?.getItem?.(nextIndex)?.path ?? null;

      return {
        index: nextIndex,
        slab: nextSlab,
        path: resolvedPath ?? (nextIndex === index ? path : null),
      };
    });
  }, [results]);

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
      handleRowSelect(path, rowIndex);
      showFilesContextMenu(event, path);
    },
    [handleRowSelect, showFilesContextMenu],
  );

  const renderRow = useCallback(
    (rowIndex: number, item: SearchResultItem | undefined, rowStyle: CSSProperties) => (
      <FileRow
        key={rowIndex}
        item={item}
        rowIndex={rowIndex}
        style={{ ...rowStyle, width: 'var(--columns-total)' }} // Enforce column width CSS vars for virtualization rows
        onContextMenu={(event, path) => handleRowContextMenu(event, path, rowIndex)}
        onSelect={(path) => handleRowSelect(path, rowIndex)}
        isSelected={selectedIndex === rowIndex}
        caseInsensitive={!caseSensitive}
        highlightTerms={highlightTerms}
      />
    ),
    [handleRowContextMenu, handleRowSelect, selectedIndex, caseSensitive, highlightTerms],
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
    '--columns-events-total': `${
      eventColWidths.name + eventColWidths.path + eventColWidths.time
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

  return (
    <>
      <main className="container" aria-hidden={showFullDiskAccessOverlay}>
        <SearchBar
          inputRef={searchInputRef}
          value={searchInputValue}
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
              results={results}
              rowHeight={ROW_HEIGHT}
              overscan={OVERSCAN_ROW_COUNT}
              renderRow={renderRow}
              onScrollSync={handleHorizontalSync}
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
