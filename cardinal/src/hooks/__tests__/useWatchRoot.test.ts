import { act, renderHook } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { useWatchRoot } from '../useWatchRoot';

const STORAGE_KEY = 'cardinal.watchRoot';

const flushEffects = async () => {
  await act(async () => {});
};

describe('useWatchRoot', () => {
  beforeEach(() => {
    window.localStorage.clear();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('hydrates from stored values and does not persist defaults', async () => {
    window.localStorage.setItem(STORAGE_KEY, '/Users/example');
    const setItemSpy = vi.spyOn(Storage.prototype, 'setItem');

    const { result } = renderHook(() => useWatchRoot());

    expect(result.current.watchRoot).toBe('/Users/example');

    await flushEffects();

    expect(setItemSpy).not.toHaveBeenCalled();
  });

  it('uses defaults and persists when no stored value exists', async () => {
    const setItemSpy = vi.spyOn(Storage.prototype, 'setItem');

    const { result } = renderHook(() => useWatchRoot());

    expect(result.current.watchRoot).toBe(result.current.defaultWatchRoot);

    await flushEffects();

    expect(setItemSpy).toHaveBeenCalledWith(STORAGE_KEY, result.current.defaultWatchRoot);
  });

  it('trims and persists updates', async () => {
    window.localStorage.setItem(STORAGE_KEY, '/');
    const setItemSpy = vi.spyOn(Storage.prototype, 'setItem');

    const { result } = renderHook(() => useWatchRoot());

    await flushEffects();

    act(() => {
      result.current.setWatchRoot(' /Users/example ');
    });

    expect(result.current.watchRoot).toBe('/Users/example');
    expect(setItemSpy).toHaveBeenCalledWith(STORAGE_KEY, '/Users/example');
  });

  it('falls back to default for empty updates', async () => {
    window.localStorage.setItem(STORAGE_KEY, '/');
    const setItemSpy = vi.spyOn(Storage.prototype, 'setItem');

    const { result } = renderHook(() => useWatchRoot());

    await flushEffects();

    act(() => {
      result.current.setWatchRoot('   ');
    });

    expect(result.current.watchRoot).toBe(result.current.defaultWatchRoot);
    expect(setItemSpy).toHaveBeenCalledWith(STORAGE_KEY, result.current.defaultWatchRoot);
  });

  it('warns when persisting defaults fails', async () => {
    const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});
    const setItemSpy = vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
      throw new Error('boom');
    });

    const { result } = renderHook(() => useWatchRoot());

    expect(result.current.watchRoot).toBe(result.current.defaultWatchRoot);

    await flushEffects();

    expect(setItemSpy).toHaveBeenCalledWith(STORAGE_KEY, result.current.defaultWatchRoot);
    expect(warnSpy).toHaveBeenCalled();
  });
});
