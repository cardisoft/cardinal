import { renderHook, waitFor } from '@testing-library/react';
import { describe, expect, it, vi, afterEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import type { SlabIndex } from '../../types/slab';
import { useFileSearch } from '../useFileSearch';

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
        return Promise.resolve({ results: backendResults, highlights: [] });
      }
      return Promise.resolve(null);
    });

    const { result } = renderHook(() => useFileSearch());

    await waitFor(() => expect(result.current.state.initialFetchCompleted).toBe(true));

    expect(result.current.state.results).toBe(backendResults);
    expect(result.current.state.resultCount).toBe(backendResults.length);
  });
});
