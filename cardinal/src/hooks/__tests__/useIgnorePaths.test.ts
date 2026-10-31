import { act, renderHook } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { migrateStoredIgnorePaths, useIgnorePaths } from '../useIgnorePaths';

const STORAGE_KEY = 'cardinal.ignorePaths';
const APPLIED_DEFAULTS_KEY = 'cardinal.ignorePaths.defaultsApplied';
const EXPECTED_DEFAULT_IGNORE_PATHS = [
  '/Volumes',
  '~/Library/CloudStorage',
  '~/Library/Biome',
  '~/Library/Caches',
  '~/Library/Logs',
  '~/Library/Metadata',
  '/Library/Caches',
  '/System/Library/Caches',
  '/private/var',
  '/private/tmp',
];

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
    // Every default already applied, so the migration is a no-op and this test stays focused
    // on parsing/URL filtering.
    window.localStorage.setItem(
      APPLIED_DEFAULTS_KEY,
      JSON.stringify(EXPECTED_DEFAULT_IGNORE_PATHS),
    );
    const setItemSpy = vi.spyOn(Storage.prototype, 'setItem');

    const { result } = renderHook(() => useIgnorePaths());

    expect(result.current.ignorePaths).toEqual(['/tmp', '/var']);

    await flushEffects();

    expect(setItemSpy).not.toHaveBeenCalled();
  });

  it('uses defaults and persists when no stored value exists', async () => {
    const setItemSpy = vi.spyOn(Storage.prototype, 'setItem');

    const { result } = renderHook(() => useIgnorePaths());

    expect(result.current.ignorePaths).toEqual(EXPECTED_DEFAULT_IGNORE_PATHS);
    expect(result.current.defaultIgnorePaths).toEqual(EXPECTED_DEFAULT_IGNORE_PATHS);

    await flushEffects();

    expect(setItemSpy).toHaveBeenCalledWith(
      STORAGE_KEY,
      JSON.stringify(EXPECTED_DEFAULT_IGNORE_PATHS),
    );
  });

  it('includes cache, biome, logs, metadata, and system runtime folders in default ignore paths', () => {
    const { result } = renderHook(() => useIgnorePaths());

    expect(result.current.defaultIgnorePaths).toContain('~/Library/CloudStorage');
    expect(result.current.defaultIgnorePaths).toContain('~/Library/Caches');
    expect(result.current.defaultIgnorePaths).toContain('/Library/Caches');
    expect(result.current.defaultIgnorePaths).toContain('/System/Library/Caches');
    expect(result.current.defaultIgnorePaths).toContain('~/Library/Biome');
    expect(result.current.defaultIgnorePaths).toContain('~/Library/Logs');
    expect(result.current.defaultIgnorePaths).toContain('~/Library/Metadata');
    expect(result.current.defaultIgnorePaths).toContain('/private/var');
    expect(result.current.defaultIgnorePaths).toContain('/private/tmp');
  });

  it('keeps an empty stored array without writing defaults', async () => {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(['', '   ']));
    const setItemSpy = vi.spyOn(Storage.prototype, 'setItem');

    const { result } = renderHook(() => useIgnorePaths());

    expect(result.current.ignorePaths).toEqual([]);

    await flushEffects();

    // The migration bookkeeping key may be created; the user's list must not be rewritten.
    expect(setItemSpy).not.toHaveBeenCalledWith(STORAGE_KEY, expect.anything());
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

  it('adds /Volumes to a stored list saved by an older release', async () => {
    // Exactly the v1 defaults: `/Volumes` (network/external mounts) was missing, so the whole
    // disk walk descended into every NFS share.
    const legacyDefaults = EXPECTED_DEFAULT_IGNORE_PATHS.filter((path) => path !== '/Volumes');
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(legacyDefaults));

    const { result } = renderHook(() => useIgnorePaths());

    expect(result.current.ignorePaths).toEqual([...legacyDefaults, '/Volumes']);
    expect(window.localStorage.getItem(STORAGE_KEY)).toBe(
      JSON.stringify([...legacyDefaults, '/Volumes']),
    );
  });

  it('does not re-add a default the user deliberately deleted', async () => {
    // Regression: the previous implementation used a single "user edited" boolean, so one
    // Preferences save disabled migrations forever and `/Volumes` was never added — which made
    // the app index every NFS/SMB share.
    const current = EXPECTED_DEFAULT_IGNORE_PATHS.filter((path) => path !== '/Volumes');
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(current));
    window.localStorage.setItem(
      APPLIED_DEFAULTS_KEY,
      JSON.stringify(EXPECTED_DEFAULT_IGNORE_PATHS),
    );

    const { result } = renderHook(() => useIgnorePaths());

    expect(result.current.ignorePaths).toEqual(current);
    expect(window.localStorage.getItem(STORAGE_KEY)).toBe(JSON.stringify(current));
  });

  it('merges a default that was never applied, even into a customised list', () => {
    const legacyDefaults = EXPECTED_DEFAULT_IGNORE_PATHS.filter((path) => path !== '/Volumes');
    const baseline = legacyDefaults; // older profile: `/Volumes` was not shipped yet

    // New defaults are appended so the user's existing order is preserved.
    expect(migrateStoredIgnorePaths(legacyDefaults, baseline)).toEqual([
      ...legacyDefaults,
      '/Volumes',
    ]);
    // Custom entries survive and nothing is duplicated.
    expect(migrateStoredIgnorePaths([...legacyDefaults, '/data/tmp'], baseline)).toEqual([
      ...legacyDefaults,
      '/data/tmp',
      '/Volumes',
    ]);
    // An empty list is a deliberate "ignore nothing".
    expect(migrateStoredIgnorePaths([], baseline)).toBeNull();
    // Already current: no rewrite.
    expect(
      migrateStoredIgnorePaths(EXPECTED_DEFAULT_IGNORE_PATHS, EXPECTED_DEFAULT_IGNORE_PATHS),
    ).toBeNull();
    // Applied before: a later deletion is respected.
    expect(migrateStoredIgnorePaths(legacyDefaults, EXPECTED_DEFAULT_IGNORE_PATHS)).toBeNull();
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
