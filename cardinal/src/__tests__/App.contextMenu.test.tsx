import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { forwardRef } from 'react';
import type { CSSProperties } from 'react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import App from '../App';

const mocks = vi.hoisted(() => ({
  showFilesContextMenu: vi.fn(),
  showEventsContextMenu: vi.fn(),
  selectSingleRow: vi.fn(),
  useContextMenuMock: vi.fn(),
  invoke: vi.fn(),
  buildListedFilesTsvHeader: vi.fn(),
  buildListedFilesTsvRows: vi.fn(),
  createListedFilesTsvFilename: vi.fn(),
  alert: vi.fn(),
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

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mocks.invoke(...args),
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
  default: ({
    onRequestExportListedFilesTsv,
  }: {
    onRequestExportListedFilesTsv?: () => Promise<void> | void;
  }) => (
    <button
      type="button"
      data-testid="status-export-trigger"
      onClick={() => void onRequestExportListedFilesTsv?.()}
    />
  ),
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
    isSearchFocused: false,
    handleSearchFocus: vi.fn(),
    handleSearchBlur: vi.fn(),
    eventFilterQuery: '',
    setEventFilterQuery: vi.fn(),
    onTabChange: vi.fn(),
    searchInputValue: '',
    onQueryChange: vi.fn(),
    onSearchInputKeyDown: vi.fn(),
    submitFilesQuery: vi.fn(),
  }),
}));

vi.mock('../hooks/useContextMenu', () => ({
  useContextMenu: (...args: unknown[]) => mocks.useContextMenuMock(...args),
}));

vi.mock('../hooks/useStableEvent', () => ({
  useStableEvent: <T extends (...args: any[]) => any>(handler: T): T => handler,
}));

vi.mock('../utils/exportListedFilesTsv', () => ({
  buildListedFilesTsvHeader: (...args: unknown[]) => mocks.buildListedFilesTsvHeader(...args),
  buildListedFilesTsvRows: (...args: unknown[]) => mocks.buildListedFilesTsvRows(...args),
  createListedFilesTsvFilename: (...args: unknown[]) => mocks.createListedFilesTsvFilename(...args),
}));

describe('App context menu regression', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.spyOn(window, 'alert').mockImplementation(mocks.alert);
    testState.activeTab = 'files';
    testState.selectedIndices = [0];
    testState.selectedPaths = ['/stale-a', '/stale-b'];
    mocks.buildListedFilesTsvHeader.mockReturnValue('header-cols');
    mocks.buildListedFilesTsvRows.mockReturnValue('row1\nrow2');
    mocks.createListedFilesTsvFilename.mockReturnValue('cardinal-word-list-test-2026-02-22.tsv');

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

  afterEach(() => {
    vi.restoreAllMocks();
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

  it('exports listed files as TSV using displayed result order', async () => {
    const fetchedNodes = [
      {
        path: '/tmp/alpha.txt',
        metadata: { type: 0, size: 10, mtime: 1, ctime: 2 },
      },
      {
        path: '/tmp/folder',
        metadata: { type: 1, size: 0, mtime: 3, ctime: 4 },
      },
    ];
    mocks.invoke
      .mockResolvedValueOnce('/tmp/cardinal-word-list-test-2026-02-22.tsv')
      .mockResolvedValueOnce(undefined)
      .mockResolvedValueOnce(fetchedNodes)
      .mockResolvedValueOnce(undefined);

    render(<App />);
    fireEvent.click(screen.getByTestId('status-export-trigger'));

    await waitFor(() => {
      expect(mocks.invoke).toHaveBeenCalledWith('write_listed_files_tsv', {
        path: '/tmp/cardinal-word-list-test-2026-02-22.tsv',
        content: 'header-cols\n',
      });
    });

    expect(mocks.invoke).toHaveBeenNthCalledWith(1, 'prompt_save_listed_files_tsv', {
      defaultFilename: 'cardinal-word-list-test-2026-02-22.tsv',
    });
    expect(mocks.invoke).toHaveBeenNthCalledWith(2, 'write_listed_files_tsv', {
      path: '/tmp/cardinal-word-list-test-2026-02-22.tsv',
      content: 'header-cols\n',
    });
    expect(mocks.invoke).toHaveBeenNthCalledWith(3, 'get_nodes_info', {
      results: [101, 202],
      includeIcons: false,
    });
    expect(mocks.invoke).toHaveBeenNthCalledWith(4, 'append_listed_files_tsv_chunk', {
      path: '/tmp/cardinal-word-list-test-2026-02-22.tsv',
      content: 'row1\nrow2',
    });

    expect(mocks.invoke).toHaveBeenCalledWith('get_nodes_info', {
      results: [101, 202],
      includeIcons: false,
    });
    expect(mocks.buildListedFilesTsvHeader).toHaveBeenCalledWith({
      filename: 'columns.filename',
      path: 'columns.path',
      size: 'columns.size',
      modified: 'columns.modified',
      created: 'columns.created',
    });
    expect(mocks.buildListedFilesTsvRows).toHaveBeenCalledWith(fetchedNodes);
    expect(mocks.createListedFilesTsvFilename).toHaveBeenCalledWith('');
  });

  it('does not fetch or write when save dialog is canceled', async () => {
    mocks.invoke.mockResolvedValueOnce(null);

    render(<App />);
    fireEvent.click(screen.getByTestId('status-export-trigger'));

    await waitFor(() => {
      expect(mocks.invoke).toHaveBeenCalledWith('prompt_save_listed_files_tsv', {
        defaultFilename: 'cardinal-word-list-test-2026-02-22.tsv',
      });
    });

    expect(mocks.invoke).toHaveBeenCalledTimes(1);
    expect(mocks.buildListedFilesTsvHeader).not.toHaveBeenCalled();
    expect(mocks.buildListedFilesTsvRows).not.toHaveBeenCalled();
  });

  it('cleans up partial export and shows an alert when writing fails', async () => {
    vi.spyOn(console, 'error').mockImplementation(() => {});
    mocks.invoke
      .mockResolvedValueOnce('/tmp/cardinal-word-list-test-2026-02-22.tsv')
      .mockRejectedValueOnce(new Error('write failed'))
      .mockResolvedValueOnce(undefined);

    render(<App />);
    fireEvent.click(screen.getByTestId('status-export-trigger'));

    await waitFor(() => {
      expect(mocks.invoke).toHaveBeenCalledWith('remove_listed_files_tsv', {
        path: '/tmp/cardinal-word-list-test-2026-02-22.tsv',
      });
    });

    expect(mocks.alert).toHaveBeenCalledWith(
      'Failed to export current files list. Please try again.',
    );
  });
});
