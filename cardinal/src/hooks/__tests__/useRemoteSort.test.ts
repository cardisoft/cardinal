import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import type { SlabIndex } from '../../types/slab';
import { useRemoteSort } from '../useRemoteSort';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

const mockedInvoke = vi.mocked(invoke);

const toSlabIndices = (values: number[]): SlabIndex[] => values.map((value) => value as SlabIndex);

const createDeferred = () => {
  let resolve: (value: SlabIndex[]) => void = () => {};
  const promise = new Promise<SlabIndex[]>((res) => {
    resolve = res;
  });
  return { promise, resolve };
};

describe('useRemoteSort', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    window.localStorage.clear();

    mockedInvoke.mockImplementation((command: string) => {
      if (command === 'get_sorted_view') {
        return Promise.resolve(toSlabIndices([2, 1, 0]));
      }
      return Promise.resolve(null);
    });
  });

  it('bumps displayedResultsVersion when sort projection changes without backend version changes', async () => {
    const results = toSlabIndices([0, 1, 2]);
    const { result } = renderHook(() => useRemoteSort(results, 1, 'en-US', () => null));

    await waitFor(() => {
      expect(result.current.displayedResultsVersion).toBeGreaterThan(0);
    });
    const beforeSortVersion = result.current.displayedResultsVersion;

    act(() => {
      result.current.handleSortToggle('filename');
    });

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith('get_sorted_view', {
        results,
        sort: { key: 'filename', direction: 'asc' },
      });
    });
    await waitFor(() => {
      expect(result.current.displayedResultsVersion).toBeGreaterThan(beforeSortVersion);
    });
  });

  it('bumps displayedResultsVersion when backend resultsVersion increments', async () => {
    const first = toSlabIndices([0, 1, 2]);
    const next = toSlabIndices([10, 11, 12]);

    const { result, rerender } = renderHook(
      ({ items, version }: { items: SlabIndex[]; version: number }) =>
        useRemoteSort(items, version, 'en-US', () => null),
      {
        initialProps: {
          items: first,
          version: 1,
        },
      },
    );

    await waitFor(() => {
      expect(result.current.displayedResultsVersion).toBeGreaterThan(0);
    });
    const beforeRefreshVersion = result.current.displayedResultsVersion;

    act(() => {
      rerender({
        items: next,
        version: 2,
      });
    });

    await waitFor(() => {
      expect(result.current.displayedResultsVersion).toBeGreaterThan(beforeRefreshVersion);
    });
    expect(result.current.displayedResults).toEqual(next);
  });

  it('does not sort remotely when the result count exceeds threshold', async () => {
    window.localStorage.setItem('cardinal.sortThreshold', '2');
    const results = toSlabIndices([0, 1, 2]);
    const { result } = renderHook(() => useRemoteSort(results, 1, 'en-US', () => null));

    act(() => {
      result.current.handleSortToggle('filename');
    });

    await waitFor(() => {
      expect(result.current.sortState).toBeNull();
    });
    expect(mockedInvoke).not.toHaveBeenCalledWith(
      'get_sorted_view',
      expect.objectContaining({ results }),
    );
    expect(result.current.displayedResults).toEqual(results);
  });

  it('bumps displayedResultsVersion when switching sorted projection on then off', async () => {
    const results = toSlabIndices([0, 1, 2]);
    const { result } = renderHook(() => useRemoteSort(results, 1, 'en-US', () => null));

    await waitFor(() => {
      expect(result.current.displayedResultsVersion).toBeGreaterThan(0);
    });
    const initialVersion = result.current.displayedResultsVersion;

    act(() => {
      result.current.handleSortToggle('filename');
    });
    await waitFor(() => {
      expect(result.current.sortState).toEqual({ key: 'filename', direction: 'asc' });
    });
    await waitFor(() => {
      expect(result.current.displayedResultsVersion).toBeGreaterThan(initialVersion);
    });
    const sortedVersion = result.current.displayedResultsVersion;

    act(() => {
      result.current.handleSortToggle('filename');
    });
    await waitFor(() => {
      expect(result.current.sortState).toEqual({ key: 'filename', direction: 'desc' });
    });

    act(() => {
      result.current.handleSortToggle('filename');
    });
    await waitFor(() => {
      expect(result.current.sortState).toBeNull();
    });
    await waitFor(() => {
      expect(result.current.displayedResultsVersion).toBeGreaterThan(sortedVersion);
    });
  });

  it('ignores stale remote sort responses and applies only the latest request', async () => {
    const results = toSlabIndices([0, 1, 2]);
    const firstRequest = createDeferred();
    const secondRequest = createDeferred();

    mockedInvoke.mockReset();
    mockedInvoke
      .mockImplementationOnce((command: string) => {
        if (command === 'get_sorted_view') return firstRequest.promise;
        return Promise.resolve(null);
      })
      .mockImplementationOnce((command: string) => {
        if (command === 'get_sorted_view') return secondRequest.promise;
        return Promise.resolve(null);
      });

    const { result } = renderHook(() => useRemoteSort(results, 1, 'en-US', () => null));

    act(() => {
      result.current.handleSortToggle('filename');
    });
    act(() => {
      result.current.handleSortToggle('filename');
    });

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledTimes(2);
    });

    act(() => {
      firstRequest.resolve(toSlabIndices([2, 1, 0]));
    });
    await Promise.resolve();

    act(() => {
      secondRequest.resolve(toSlabIndices([1, 2, 0]));
    });

    await waitFor(() => {
      expect(result.current.displayedResults).toEqual(toSlabIndices([1, 2, 0]));
    });
  });
});
