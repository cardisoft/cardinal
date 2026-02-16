import { fireEvent, render, screen } from '@testing-library/react';
import { forwardRef } from 'react';
import type { CSSProperties } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import App from '../App';

const mocks = vi.hoisted(() => ({
  showFilesContextMenu: vi.fn(),
  showEventsContextMenu: vi.fn(),
  selectSingleRow: vi.fn(),
  useContextMenuMock: vi.fn(),
}));

const testState = vi.hoisted(() => ({
  activeTab: 'files' as 'files' | 'events',
  selectedIndices: [0] as number[],
  selectedPaths: ['/stale-a', '/stale-b'] as string[],
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => key,
    i18n: {
      language: 'en-US',
      changeLanguage: vi.fn().mockResolvedValue(undefined),
    },
  }),
}));

vi.mock('../components/SearchBar', () => ({
  SearchBar: ({ inputRef }: { inputRef: React.Ref<HTMLInputElement> }) => (
    <input data-testid="search-input" ref={inputRef} />
  ),
}));

vi.mock('../components/FileRow', () => ({
  FileRow: ({
    item,
    rowIndex,
    onContextMenu,
  }: {
    item: { path: string };
    rowIndex: number;
    onContextMenu?: (
      event: React.MouseEvent<HTMLDivElement>,
      path: string,
      rowIndex: number,
    ) => void;
  }) => (
    <div
      data-testid="file-row"
      onContextMenu={(event) => onContextMenu?.(event, item.path, rowIndex)}
    />
  ),
}));

vi.mock('../components/FilesTabContent', () => ({
  FilesTabContent: ({
    renderRow,
  }: {
    renderRow: (
      rowIndex: number,
      item: { path: string } | undefined,
      rowStyle: CSSProperties,
    ) => React.ReactNode;
  }) => (
    <div data-testid="files-tab-content">
      {renderRow(1, { path: '/clicked' }, {} as CSSProperties)}
    </div>
  ),
}));

vi.mock('../components/PermissionOverlay', () => ({
  PermissionOverlay: () => null,
}));

vi.mock('../components/PreferencesOverlay', () => ({
  default: () => null,
}));

vi.mock('../components/StatusBar', () => ({
  default: () => null,
}));

vi.mock('../components/FSEventsPanel', () => ({
  default: forwardRef(function MockFSEventsPanel(
    {
      onContextMenu,
    }: {
      onContextMenu: (event: React.MouseEvent<HTMLDivElement>, path: string) => void;
    },
    _ref: React.ForwardedRef<unknown>,
  ) {
    return (
      <div
        data-testid="event-row"
        onContextMenu={(event) => {
          onContextMenu(event, '/event-path');
        }}
      />
    );
  }),
}));

vi.mock('../hooks/useFileSearch', () => ({
  useFileSearch: () => ({
    state: {
      results: [101, 202],
      resultsVersion: 1,
      scannedFiles: 0,
      processedEvents: 0,
      rescanErrors: 0,
      currentQuery: '',
      highlightTerms: [],
      showLoadingUI: false,
      initialFetchCompleted: true,
      durationMs: 0,
      resultCount: 2,
      searchError: null,
      lifecycleState: 'Ready',
    },
    searchParams: {
      query: '',
      caseSensitive: false,
    },
    updateSearchParams: vi.fn(),
    queueSearch: vi.fn(),
    handleStatusUpdate: vi.fn(),
    setLifecycleState: vi.fn(),
    requestRescan: vi.fn(),
  }),
}));

vi.mock('../hooks/useColumnResize', () => ({
  useColumnResize: () => ({
    colWidths: {
      filename: 200,
      path: 300,
      size: 100,
      modified: 120,
      created: 120,
    },
    onResizeStart: vi.fn(() => vi.fn()),
    autoFitColumns: vi.fn(),
  }),
}));

vi.mock('../hooks/useEventColumnWidths', () => ({
  useEventColumnWidths: () => ({
    eventColWidths: {
      time: 120,
      event: 180,
      name: 180,
      path: 260,
    },
    onEventResizeStart: vi.fn(),
    autoFitEventColumns: vi.fn(),
  }),
}));

vi.mock('../hooks/useRecentFSEvents', () => ({
  useRecentFSEvents: () => ({
    filteredEvents: [],
    eventFilterQuery: '',
    setEventFilterQuery: vi.fn(),
  }),
}));

vi.mock('../hooks/useRemoteSort', () => ({
  DEFAULT_SORTABLE_RESULT_THRESHOLD: 20000,
  useRemoteSort: () => ({
    sortState: null,
    displayedResults: [101, 202],
    displayedResultsVersion: 1,
    sortThreshold: 20000,
    setSortThreshold: vi.fn(),
    canSort: true,
    isSorting: false,
    sortDisabledTooltip: null,
    sortButtonsDisabled: false,
    handleSortToggle: vi.fn(),
  }),
}));

vi.mock('../hooks/useSelection', () => ({
  useSelection: () => ({
    selectedIndices: testState.selectedIndices,
    selectedIndicesRef: { current: testState.selectedIndices },
    activeRowIndex: null,
    selectedPaths: testState.selectedPaths,
    handleRowSelect: vi.fn(),
    selectSingleRow: mocks.selectSingleRow,
    clearSelection: vi.fn(),
    moveSelection: vi.fn(),
  }),
}));

vi.mock('../hooks/useQuickLook', () => ({
  useQuickLook: () => ({
    toggleQuickLook: vi.fn(),
    updateQuickLook: vi.fn(),
    closeQuickLook: vi.fn(),
  }),
}));

vi.mock('../hooks/useSearchHistory', () => ({
  useSearchHistory: () => ({
    handleInputChange: vi.fn(),
    navigate: vi.fn(),
    ensureTailValue: vi.fn(),
    resetCursorToTail: vi.fn(),
  }),
}));

vi.mock('../hooks/useFullDiskAccessPermission', () => ({
  useFullDiskAccessPermission: () => ({
    status: 'granted',
    isChecking: false,
    requestPermission: vi.fn(),
  }),
}));

vi.mock('../hooks/useAppPreferences', () => ({
  useAppPreferences: () => ({
    isPreferencesOpen: false,
    closePreferences: vi.fn(),
    trayIconEnabled: false,
    setTrayIconEnabled: vi.fn(),
    watchRoot: '/',
    defaultWatchRoot: '/',
    ignorePaths: ['/Volumes'],
    defaultIgnorePaths: ['/Volumes'],
    preferencesResetToken: 0,
    handleWatchConfigChange: vi.fn(),
    handleResetPreferences: vi.fn(),
  }),
}));

vi.mock('../hooks/useAppWindowListeners', () => ({
  useAppWindowListeners: () => ({ isWindowFocused: true }),
}));

vi.mock('../hooks/useAppHotkeys', () => ({
  useAppHotkeys: () => undefined,
}));

vi.mock('../hooks/useFilesTabState', () => ({
  useFilesTabState: () => ({
    activeTab: testState.activeTab,
    setActiveTab: vi.fn(),
    isSearchFocused: false,
    handleSearchFocus: vi.fn(),
    handleSearchBlur: vi.fn(),
  }),
}));

vi.mock('../hooks/useContextMenu', () => ({
  useContextMenu: (...args: unknown[]) => mocks.useContextMenuMock(...args),
}));

vi.mock('../hooks/useStableEvent', () => ({
  useStableEvent: <T extends (...args: any[]) => any>(handler: T): T => handler,
}));

describe('App context menu regression', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    testState.activeTab = 'files';
    testState.selectedIndices = [0];
    testState.selectedPaths = ['/stale-a', '/stale-b'];

    mocks.useContextMenuMock
      .mockReturnValueOnce({
        showContextMenu: mocks.showFilesContextMenu,
        showHeaderContextMenu: vi.fn(),
      })
      .mockReturnValueOnce({
        showContextMenu: mocks.showEventsContextMenu,
        showHeaderContextMenu: vi.fn(),
      });
  });

  it('uses clicked row path for context menu when row is not already selected', () => {
    render(<App />);

    fireEvent.contextMenu(screen.getByTestId('file-row'));

    expect(mocks.selectSingleRow).toHaveBeenCalledWith(1);
    expect(mocks.showFilesContextMenu).toHaveBeenCalledTimes(1);
    expect(mocks.showFilesContextMenu.mock.calls[0][1]).toEqual(['/clicked']);
  });

  it('uses selected paths for context menu when clicked row is already selected', () => {
    testState.selectedIndices = [1];
    testState.selectedPaths = ['/selected-a', '/selected-b'];

    render(<App />);

    fireEvent.contextMenu(screen.getByTestId('file-row'));

    expect(mocks.selectSingleRow).not.toHaveBeenCalled();
    expect(mocks.showFilesContextMenu).toHaveBeenCalledTimes(1);
    expect(mocks.showFilesContextMenu.mock.calls[0][1]).toEqual(['/selected-a', '/selected-b']);
  });

  it('falls back to clicked row path when selected row has no selected paths snapshot', () => {
    testState.selectedIndices = [1];
    testState.selectedPaths = [];

    render(<App />);

    fireEvent.contextMenu(screen.getByTestId('file-row'));

    expect(mocks.selectSingleRow).not.toHaveBeenCalled();
    expect(mocks.showFilesContextMenu).toHaveBeenCalledTimes(1);
    expect(mocks.showFilesContextMenu.mock.calls[0][1]).toEqual(['/clicked']);
  });

  it('passes event path to events context menu as a single target item', () => {
    testState.activeTab = 'events';

    render(<App />);

    fireEvent.contextMenu(screen.getByTestId('event-row'));

    expect(mocks.showEventsContextMenu).toHaveBeenCalledTimes(1);
    expect(mocks.showEventsContextMenu.mock.calls[0][1]).toEqual(['/event-path']);
  });
});
