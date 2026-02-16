import { act, renderHook, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { subscribeIconUpdate } from '../../runtime/tauriEventRuntime';
import type { SlabIndex } from '../../types/slab';
import { useDataLoader } from '../useDataLoader';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

vi.mock('../../runtime/tauriEventRuntime', () => ({
  subscribeIconUpdate: vi.fn(),
}));

const mockedInvoke = vi.mocked(invoke);
const mockedSubscribeIconUpdate = vi.mocked(subscribeIconUpdate);
type HookProps = { results: SlabIndex[]; version: number };

const buildNodeInfo = (slabIndex: SlabIndex) => ({
  path: `/tmp/file-${slabIndex}`,
  icon: null,
  metadata: null,
  size: null,
  mtime: null,
  ctime: null,
});

const renderDataLoader = (initialProps: HookProps) =>
  renderHook(({ results, version }: HookProps) => useDataLoader(results, version), {
    initialProps,
  });

describe('useDataLoader', () => {
  const iconUpdateUnlisten = vi.fn();

  beforeEach(() => {
    mockedSubscribeIconUpdate.mockImplementation(() => iconUpdateUnlisten);
    mockedInvoke.mockImplementation((command: string, payload?: unknown) => {
      if (command !== 'get_nodes_info') {
        return Promise.resolve(null);
      }

      const slabIndices = (payload as { results: SlabIndex[] }).results;
      return Promise.resolve(slabIndices.map((slabIndex) => buildNodeInfo(slabIndex)));
    });
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it('does not clear cache when only results reference changes', async () => {
    const slab11 = 11 as SlabIndex;
    const slab22 = 22 as SlabIndex;
    const first = [slab11, slab22];
    const { result, rerender } = renderDataLoader({ results: first, version: 1 });

    await act(async () => {
      await result.current.ensureRangeLoaded(0, 1);
    });

    await waitFor(() => {
      expect(result.current.cache.get(slab11)?.path).toBe('/tmp/file-11');
      expect(result.current.cache.get(slab22)?.path).toBe('/tmp/file-22');
    });
    expect(mockedInvoke).toHaveBeenCalledTimes(1);

    rerender({ results: [...first], version: 1 });

    await act(async () => {
      await result.current.ensureRangeLoaded(0, 1);
    });

    expect(result.current.cache.get(slab11)?.path).toBe('/tmp/file-11');
    expect(result.current.cache.get(slab22)?.path).toBe('/tmp/file-22');
    expect(mockedInvoke).toHaveBeenCalledTimes(1);
  });

  it('resets cache when results version changes', async () => {
    const first = [33 as SlabIndex, 44 as SlabIndex];
    const { result, rerender } = renderDataLoader({ results: first, version: 1 });

    await act(async () => {
      await result.current.ensureRangeLoaded(0, 1);
    });

    await waitFor(() => expect(result.current.cache.size).toBe(2));
    expect(mockedInvoke).toHaveBeenCalledTimes(1);

    rerender({ results: first, version: 2 });

    await waitFor(() => expect(result.current.cache.size).toBe(0));

    await act(async () => {
      await result.current.ensureRangeLoaded(0, 1);
    });

    await waitFor(() => expect(result.current.cache.size).toBe(2));
    expect(mockedInvoke).toHaveBeenCalledTimes(2);
  });

  it('cleans up icon update subscription on unmount', async () => {
    const { unmount } = renderDataLoader({ results: [11 as SlabIndex], version: 1 });
    unmount();

    expect(iconUpdateUnlisten).toHaveBeenCalled();
  });
});
