import { act, renderHook, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import type { SlabIndex } from '../../types/slab';
import { DIRECTORY_SCOPE_OPEN_STORAGE_KEY, useFileSearch } from '../useFileSearch';
import { SearchStatusCode } from '../../types/ipc';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

const mockedInvoke = vi.mocked(invoke);

const searchResponse = (results: SlabIndex[] = []) => ({
  results,
  highlights: [],
  statusCode: SearchStatusCode.OK,
});

const mockSearchSuccess = (results: SlabIndex[] = []) => {
  mockedInvoke.mockImplementation((command: string) => {
    if (command === 'get_app_status') {
      return Promise.resolve('Ready');
    }
    if (command === 'search') {
      return Promise.resolve(searchResponse(results));
    }
    return Promise.resolve(null);
  });
};

const mockSearchCancelled = () => {
  mockedInvoke.mockImplementation((command: string) => {
    if (command === 'get_app_status') {
      return Promise.resolve('Ready');
    }
    if (command === 'search') {
      return Promise.resolve({
        results: [],
        highlights: [],
        statusCode: SearchStatusCode.CANCELLED,
      });
    }
    return Promise.resolve(null);
  });
};

const renderReadySearchHook = async () => {
  const rendered = renderHook(() => useFileSearch());
  await waitFor(() => expect(rendered.result.current.state.initialFetchCompleted).toBe(true));
  return rendered;
};

describe('useFileSearch', () => {
  beforeEach(() => {
    window.localStorage.setItem(DIRECTORY_SCOPE_OPEN_STORAGE_KEY, 'false');
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it('reuses backend results array without copying', async () => {
    const backendResults = [1, 2, 3] as SlabIndex[];
    mockSearchSuccess(backendResults);
    const { result } = await renderReadySearchHook();

    expect(result.current.state.results).toBe(backendResults);
    expect(result.current.state.resultCount).toBe(backendResults.length);
  });

  it('ignores results when backend returns CANCELLED status', async () => {
    const initialResults = [1, 2, 3] as SlabIndex[];
    mockSearchSuccess(initialResults);
    const { result } = await renderReadySearchHook();
    expect(result.current.state.results).toBe(initialResults);

    mockSearchCancelled();

    act(() => {
      result.current.queueSearch('new query', { immediate: true });
    });

    // Cancelled results should not overwrite state, and loading should settle.
    await waitFor(() => {
      expect(result.current.state.results).toBe(initialResults);
      expect(result.current.state.currentQuery).toBe(''); // Query doesn't update on cancelled search
      expect(result.current.state.showLoadingUI).toBe(false);
      expect(result.current.state.initialFetchCompleted).toBe(true);
    });
  });

  it('reports indexing instead of waiting when the backend is still building its index', async () => {
    mockedInvoke.mockImplementation((command: string) => {
      if (command === 'get_app_status') {
        return Promise.resolve('Initializing');
      }
      if (command === 'search') {
        return Promise.resolve({
          results: [],
          highlights: [],
          statusCode: SearchStatusCode.INDEXING,
        });
      }
      return Promise.resolve(null);
    });

    const { result } = renderHook(() => useFileSearch());

    await waitFor(() => expect(result.current.state.indexing).toBe(true));
    expect(result.current.state.results).toEqual([]);
    expect(result.current.state.showLoadingUI).toBe(false);
    expect(result.current.state.lifecycleState).toBe('Initializing');
  });

  it('retries the current query once the index is ready', async () => {
    const backendResults = [7] as SlabIndex[];
    let searchCalls = 0;
    mockedInvoke.mockImplementation((command: string) => {
      if (command === 'get_app_status') {
        return Promise.resolve('Initializing');
      }
      if (command === 'search') {
        searchCalls += 1;
        return Promise.resolve(
          searchCalls === 1
            ? { results: [], highlights: [], statusCode: SearchStatusCode.INDEXING }
            : searchResponse(backendResults),
        );
      }
      return Promise.resolve(null);
    });

    const { result } = renderHook(() => useFileSearch());
    await waitFor(() => expect(result.current.state.indexing).toBe(true));

    act(() => {
      result.current.setLifecycleState('Updating');
    });

    await waitFor(() => expect(result.current.state.results).toEqual(backendResults));
    expect(result.current.state.indexing).toBe(false);
    expect(searchCalls).toBeGreaterThanOrEqual(2);
  });

  it('re-runs the query when a temporary folder-scope result set is replaced by the real index', async () => {
    // Slab indices are owned by the cache that produced them, so results answered from the
    // temporary scope index must be refreshed once the full index is installed.
    const scopedResults = [1] as SlabIndex[];
    const fullResults = [9] as SlabIndex[];
    let searchCalls = 0;
    mockedInvoke.mockImplementation((command: string) => {
      if (command === 'get_app_status') {
        return Promise.resolve('Initializing');
      }
      if (command === 'search') {
        searchCalls += 1;
        return Promise.resolve(searchResponse(searchCalls === 1 ? scopedResults : fullResults));
      }
      return Promise.resolve(null);
    });

    const { result } = renderHook(() => useFileSearch());
    await waitFor(() => expect(result.current.state.results).toEqual(scopedResults));

    act(() => {
      result.current.setLifecycleState('Updating');
    });

    await waitFor(() => expect(result.current.state.results).toEqual(fullResults));
  });

  it('polls until the temporary folder-scope index is ready', async () => {
    const backendResults = [42] as SlabIndex[];
    let searchCalls = 0;
    mockedInvoke.mockImplementation((command: string) => {
      if (command === 'get_app_status') {
        return Promise.resolve('Initializing');
      }
      if (command === 'search') {
        searchCalls += 1;
        return Promise.resolve(
          searchCalls === 1
            ? {
                results: [],
                highlights: [],
                statusCode: SearchStatusCode.SCOPED_INDEXING,
                scope: '/Volumes/nfs_D',
              }
            : searchResponse(backendResults),
        );
      }
      return Promise.resolve(null);
    });

    const { result } = renderHook(() => useFileSearch());
    await waitFor(() => expect(result.current.state.indexingScope).toBe('/Volumes/nfs_D'));
    expect(result.current.state.indexing).toBe(true);

    // The 800 ms poll retries the same query until the scoped walk finishes.
    await waitFor(() => expect(result.current.state.results).toEqual(backendResults), {
      timeout: 4000,
    });
    expect(result.current.state.indexing).toBe(false);
    expect(result.current.state.indexingScope).toBeNull();
    expect(searchCalls).toBeGreaterThanOrEqual(2);
  });

  it('keeps polling while the scope index is still building (more than one retry)', async () => {
    // Regression: the reducer used to return the identical state object on retries, so the
    // poll effect's dependencies never changed and polling stopped after a single retry.
    const backendResults = [43] as SlabIndex[];
    let searchCalls = 0;
    mockedInvoke.mockImplementation((command: string) => {
      if (command === 'get_app_status') {
        return Promise.resolve('Initializing');
      }
      if (command === 'search') {
        searchCalls += 1;
        return Promise.resolve(
          searchCalls < 4
            ? {
                results: [],
                highlights: [],
                statusCode: SearchStatusCode.SCOPED_INDEXING,
                scope: '/Volumes/nfs_D',
              }
            : searchResponse(backendResults),
        );
      }
      return Promise.resolve(null);
    });

    const { result } = renderHook(() => useFileSearch());
    await waitFor(() => expect(result.current.state.indexingScope).toBe('/Volumes/nfs_D'));

    await waitFor(() => expect(result.current.state.results).toEqual(backendResults), {
      timeout: 6000,
    });
    expect(searchCalls).toBeGreaterThanOrEqual(4);
  });

  it('stops polling and reports a folder scope that is not a directory', async () => {
    let searchCalls = 0;
    mockedInvoke.mockImplementation((command: string) => {
      if (command === 'get_app_status') {
        return Promise.resolve('Ready');
      }
      if (command === 'search') {
        searchCalls += 1;
        return Promise.resolve({
          results: [],
          highlights: [],
          statusCode: SearchStatusCode.SCOPE_UNAVAILABLE,
          scope: '/Volumes/not-a-dir',
        });
      }
      return Promise.resolve(null);
    });

    const { result } = renderHook(() => useFileSearch());
    await waitFor(() => expect(result.current.state.unavailableScope).toBe('/Volumes/not-a-dir'));

    expect(result.current.state.indexing).toBe(false);
    expect(result.current.state.indexingScope).toBeNull();

    // An unusable scope must not keep retrying: the poll effect keys off `indexing`.
    const callsAfterError = searchCalls;
    await new Promise((resolve) => setTimeout(resolve, 1200));
    expect(searchCalls).toBe(callsAfterError);
  });

  it('does not send directory scope while the scope input is inactive', async () => {
    mockSearchSuccess();
    const { result } = await renderReadySearchHook();
    mockedInvoke.mockClear();

    act(() => {
      result.current.queueDirectorySearch('Projects', { immediate: true });
    });

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith('search', {
        query: null,
        directoryQuery: null,
        options: {
          caseInsensitive: true,
        },
      });
    });
  });

  it('re-runs search when directory scope is toggled and controls the directory payload', async () => {
    mockSearchSuccess();
    const { result } = await renderReadySearchHook();

    act(() => {
      result.current.queueDirectorySearch('Projects', { immediate: true });
    });
    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenLastCalledWith('search', {
        query: null,
        directoryQuery: null,
        options: {
          caseInsensitive: true,
        },
      });
    });

    mockedInvoke.mockClear();
    act(() => {
      result.current.queueDirectoryScopeOpen(true);
    });
    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenLastCalledWith('search', {
        query: null,
        directoryQuery: 'Projects',
        options: {
          caseInsensitive: true,
        },
      });
      expect(result.current.state.currentDirectoryQuery).toBe('Projects');
      expect(window.localStorage.getItem(DIRECTORY_SCOPE_OPEN_STORAGE_KEY)).toBe('true');
    });

    mockedInvoke.mockClear();
    act(() => {
      result.current.queueDirectoryScopeOpen(false);
    });
    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenLastCalledWith('search', {
        query: null,
        directoryQuery: null,
        options: {
          caseInsensitive: true,
        },
      });
      expect(result.current.state.currentDirectoryQuery).toBe('');
      expect(window.localStorage.getItem(DIRECTORY_SCOPE_OPEN_STORAGE_KEY)).toBe('false');
    });
  });

  it('hydrates persisted directory scope open state', async () => {
    window.localStorage.setItem(DIRECTORY_SCOPE_OPEN_STORAGE_KEY, 'true');
    mockSearchSuccess();
    const { result } = await renderReadySearchHook();

    expect(result.current.searchParams.directoryScopeOpen).toBe(true);

    act(() => {
      result.current.queueDirectorySearch('Projects', { immediate: true });
    });

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenLastCalledWith('search', {
        query: null,
        directoryQuery: 'Projects',
        options: {
          caseInsensitive: true,
        },
      });
    });
  });

  it('passes whitespace directory scope through when the scope is active', async () => {
    mockSearchSuccess();
    const { result } = await renderReadySearchHook();

    act(() => {
      result.current.queueDirectorySearch('   ', { immediate: true });
    });
    act(() => {
      result.current.queueDirectoryScopeOpen(true);
    });

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenLastCalledWith('search', {
        query: null,
        directoryQuery: '   ',
        options: {
          caseInsensitive: true,
        },
      });
      expect(result.current.state.currentDirectoryQuery).toBe('   ');
    });
  });

  it('passes whitespace query through to search', async () => {
    mockSearchSuccess();
    const { result } = await renderReadySearchHook();
    mockedInvoke.mockClear();

    act(() => {
      result.current.queueSearch('   ', { immediate: true });
    });

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenLastCalledWith('search', {
        query: '   ',
        directoryQuery: null,
        options: {
          caseInsensitive: true,
        },
      });
      expect(result.current.state.currentQuery).toBe('   ');
    });
  });
});

describe('useFileSearch launch-list paging', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    window.localStorage.clear();
    mockSearchSuccess([1, 2, 3] as SlabIndex[]);
  });

  it('appends a page without resetting the result version', async () => {
    // `resultsVersion` resetting would wipe the row cache and the scroll position, undoing
    // the scroll that requested the next page.
    const { result } = renderHook(() => useFileSearch());
    await waitFor(() => expect(result.current.state.results).toHaveLength(3));
    const versionBefore = result.current.state.resultsVersion;

    act(() => {
      result.current.appendResults([4, 5] as SlabIndex[]);
    });

    expect(result.current.state.results).toEqual([1, 2, 3, 4, 5]);
    expect(result.current.state.resultsVersion).toBe(versionBefore);
    expect(result.current.state.resultCount).toBe(5);
  });

  it('ignores an empty page', async () => {
    const { result } = renderHook(() => useFileSearch());
    await waitFor(() => expect(result.current.state.results).toHaveLength(3));

    act(() => {
      result.current.appendResults([]);
    });

    expect(result.current.state.results).toEqual([1, 2, 3]);
  });
});
