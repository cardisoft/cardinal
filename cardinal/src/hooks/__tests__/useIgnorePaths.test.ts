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

  it('hydrates from stored values and preserves blank entries', async () => {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify([' /tmp ', '', 42, '   ', '/var']));
    const setItemSpy = vi.spyOn(Storage.prototype, 'setItem');

    const { result } = renderHook(() => useIgnorePaths());

    expect(result.current.ignorePaths).toEqual([' /tmp ', '', '   ', '/var']);

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

  it('ships data-driven defaults with per-pattern comments and blank separators', () => {
    const { result } = renderHook(() => useIgnorePaths());
    const defaults = result.current.defaultIgnorePaths;

    expect(defaults[0]).toContain('0 files in the 2026-02-24 sample');
    expect(defaults).toContain('');
    expect(defaults).toContain('**/node_modules/');
    expect(defaults).toContain('**/.pnpm/');
    expect(defaults).toContain('**/.bun/install/cache/');
    expect(defaults).toContain('**/Library/Caches/');
    expect(defaults).toContain('**/.cache/');
    expect(defaults).toContain('**/Library/Application Support/*/Partitions/*/Cache/');
    expect(defaults).toContain('**/Library/Application Support/*/Cache/');
    expect(defaults).toContain('**/Library/pnpm/store/');
    expect(defaults).toContain('**/target/');
    expect(defaults).toContain('**/Code Cache/');
    expect(defaults).toContain('**/Service Worker/CacheStorage/');
    expect(defaults).toContain('**/IndexedDB/');
    expect(defaults).toContain('**/WebStorage/');
    expect(defaults).toContain('**/.next/');
    expect(defaults).toContain('**/.git/');
    expect(defaults).toContain('**/__pycache__/');
    expect(defaults).toContain('**/.cocoapods/');
    expect(defaults).toContain('**/.opam/');
    expect(defaults).toContain('**/*.pyc');
    expect(defaults).toContain('**/.DS_Store');
    expect(defaults).toContain('**/Google/Chrome*/Cache/');
    expect(defaults).toContain('/System/Volumes/');
    expect(defaults).toContain('~/Library/CloudStorage');
  });

  it('keeps a whitespace-only stored array without writing defaults', async () => {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(['', '   ']));
    const setItemSpy = vi.spyOn(Storage.prototype, 'setItem');

    const { result } = renderHook(() => useIgnorePaths());

    expect(result.current.ignorePaths).toEqual(['', '   ']);

    await flushEffects();

    expect(setItemSpy).not.toHaveBeenCalled();
  });

  it('preserves pattern text including blank entries and persists updates', async () => {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify([]));
    const setItemSpy = vi.spyOn(Storage.prototype, 'setItem');

    const { result } = renderHook(() => useIgnorePaths());

    await flushEffects();

    act(() => {
      result.current.setIgnorePaths([' /tmp ', '', '/var', '   ']);
    });

    expect(result.current.ignorePaths).toEqual([' /tmp ', '', '/var', '   ']);
    expect(setItemSpy).toHaveBeenCalledWith(
      STORAGE_KEY,
      JSON.stringify([' /tmp ', '', '/var', '   ']),
    );
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
