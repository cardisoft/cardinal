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
