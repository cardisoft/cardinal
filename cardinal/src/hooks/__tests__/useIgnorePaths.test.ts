import { act, renderHook } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { useIgnorePaths } from '../useIgnorePaths';

const STORAGE_KEY = 'cardinal.ignorePaths';

const flushEffects = async () => {
  await act(async () => {});
};

describe('useIgnorePaths', () => {
  beforeEach(() => {
    window.localStorage.clear();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('hydrates from stored values and filters invalid entries', async () => {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify([' /tmp ', '', 42, '   ', '/var']));
    const setItemSpy = vi.spyOn(Storage.prototype, 'setItem');

    const { result } = renderHook(() => useIgnorePaths());

    expect(result.current.ignorePaths).toEqual(['/tmp', '/var']);

    await flushEffects();

    expect(setItemSpy).not.toHaveBeenCalled();
  });

  it('uses defaults and persists when no stored value exists', async () => {
    const setItemSpy = vi.spyOn(Storage.prototype, 'setItem');

    const { result } = renderHook(() => useIgnorePaths());

    expect(result.current.ignorePaths).toEqual(result.current.defaultIgnorePaths);

    await flushEffects();

    expect(setItemSpy).toHaveBeenCalledWith(
      STORAGE_KEY,
      JSON.stringify(result.current.defaultIgnorePaths),
    );
  });

  it('includes CloudStorage in default ignore paths', () => {
    const { result } = renderHook(() => useIgnorePaths());

    expect(result.current.defaultIgnorePaths).toContain('~/Library/CloudStorage');
  });

  it('keeps an empty stored array without writing defaults', async () => {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(['', '   ']));
    const setItemSpy = vi.spyOn(Storage.prototype, 'setItem');

    const { result } = renderHook(() => useIgnorePaths());

    expect(result.current.ignorePaths).toEqual([]);

    await flushEffects();

    expect(setItemSpy).not.toHaveBeenCalled();
  });

  it('cleans and persists updates', async () => {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify([]));
    const setItemSpy = vi.spyOn(Storage.prototype, 'setItem');

    const { result } = renderHook(() => useIgnorePaths());

    await flushEffects();

    act(() => {
      result.current.setIgnorePaths([' /tmp ', '', '/var', '   ']);
    });

    expect(result.current.ignorePaths).toEqual(['/tmp', '/var']);
    expect(setItemSpy).toHaveBeenCalledWith(STORAGE_KEY, JSON.stringify(['/tmp', '/var']));
  });

  it('falls back to defaults when stored JSON is invalid', async () => {
    const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});
    window.localStorage.setItem(STORAGE_KEY, '{');
    const setItemSpy = vi.spyOn(Storage.prototype, 'setItem');

    const { result } = renderHook(() => useIgnorePaths());

    expect(result.current.ignorePaths).toEqual(result.current.defaultIgnorePaths);

    await flushEffects();

    expect(setItemSpy).toHaveBeenCalledWith(
      STORAGE_KEY,
      JSON.stringify(result.current.defaultIgnorePaths),
    );
    warnSpy.mockRestore();
  });
});
