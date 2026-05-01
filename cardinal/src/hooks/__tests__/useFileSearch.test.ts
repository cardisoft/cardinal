import { renderHook, waitFor } from '@testing-library/react';
import { describe, expect, it, vi, afterEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import type { SlabIndex } from '../../types/slab';
import { useFileSearch } from '../useFileSearch';
import { SearchStatusCode } from '../../types/ipc';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

const mockedInvoke = vi.mocked(invoke);

describe('useFileSearch', () => {
  afterEach(() => {
    vi.clearAllMocks();
  });

  it('reuses backend results array without copying', async () => {
    const backendResults = [1, 2, 3] as SlabIndex[];

    mockedInvoke.mockImplementation((command: string) => {
      if (command === 'get_app_status') {
        return Promise.resolve('Ready');
      }
      if (command === 'search') {
        return Promise.resolve({
          results: backendResults,
          highlights: [],
          statusCode: SearchStatusCode.OK,
        });
      }
      return Promise.resolve(null);
    });

    const { result } = renderHook(() => useFileSearch());

    await waitFor(() => expect(result.current.state.initialFetchCompleted).toBe(true));

    expect(result.current.state.results).toBe(backendResults);
    expect(result.current.state.resultCount).toBe(backendResults.length);
  });

  it('ignores results when backend returns CANCELLED status', async () => {
    const initialResults = [1, 2, 3] as SlabIndex[];

    mockedInvoke.mockImplementation((command: string) => {
      if (command === 'get_app_status') {
        return Promise.resolve('Ready');
      }
      if (command === 'search') {
        // First call (initial search) returns results
        return Promise.resolve({
          results: initialResults,
          highlights: [],
          statusCode: SearchStatusCode.OK,
        });
      }
      return Promise.resolve(null);
    });

    const { result } = renderHook(() => useFileSearch());

    // Wait for initial search to complete
    await waitFor(() => expect(result.current.state.initialFetchCompleted).toBe(true));
    expect(result.current.state.results).toBe(initialResults);

    // Mock search to return CANCELLED status for the next call
    mockedInvoke.mockImplementation((command: string) => {
      if (command === 'search') {
        return Promise.resolve({
          results: [],
          highlights: [],
          statusCode: SearchStatusCode.CANCELLED,
        });
      }
      return Promise.resolve('Ready');
    });

    // Trigger a new search
    result.current.queueSearch('new query', { immediate: true });

    // The results should still be the initial results, not updated or cleared
    // We wait a bit to ensure it had time to process if it were going to
    await new Promise((resolve) => setTimeout(resolve, 50));
    expect(result.current.state.results).toBe(initialResults);
    expect(result.current.state.currentQuery).toBe(''); // Query doesn't update on cancelled search
  });
});
