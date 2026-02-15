import { act, renderHook } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { useStoredState } from '../useStoredState';

const STORAGE_KEY = 'cardinal.test.storedState';
const READ_ERROR_MESSAGE = 'read error';
const WRITE_ERROR_MESSAGE = 'write error';

type HookOptions = {
  normalize?: (value: string) => string;
};

const renderStoredStateHook = ({ normalize }: HookOptions = {}) =>
  renderHook(() =>
    useStoredState<string>({
      key: STORAGE_KEY,
      defaultValue: 'default',
      read: (raw) => (raw.startsWith('v:') ? raw.slice(2) : null),
      write: (value) => `v:${value}`,
      normalize,
      readErrorMessage: READ_ERROR_MESSAGE,
      writeErrorMessage: WRITE_ERROR_MESSAGE,
    }),
  );

describe('useStoredState', () => {
  beforeEach(() => {
    window.localStorage.clear();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('hydrates from stored value without rewriting defaults', () => {
    window.localStorage.setItem(STORAGE_KEY, 'v:saved');
    const setItemSpy = vi.spyOn(Storage.prototype, 'setItem');

    const { result } = renderStoredStateHook();

    expect(result.current[0]).toBe('saved');
    expect(setItemSpy).not.toHaveBeenCalled();
  });

  it('returns default and warns when reading storage throws', () => {
    const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});
    vi.spyOn(Storage.prototype, 'getItem').mockImplementation(() => {
      throw new Error('boom');
    });
    const setItemSpy = vi.spyOn(Storage.prototype, 'setItem');

    const { result } = renderStoredStateHook();

    expect(result.current[0]).toBe('default');
    expect(setItemSpy).toHaveBeenCalledWith(STORAGE_KEY, 'v:default');
    expect(warnSpy).toHaveBeenCalledWith(READ_ERROR_MESSAGE, expect.any(Error));
  });

  it('returns default and persists when stored value is invalid', () => {
    window.localStorage.setItem(STORAGE_KEY, 'invalid');
    const setItemSpy = vi.spyOn(Storage.prototype, 'setItem');

    const { result } = renderStoredStateHook();

    expect(result.current[0]).toBe('default');
    expect(setItemSpy).toHaveBeenCalledWith(STORAGE_KEY, 'v:default');
  });

  it('applies normalize before state update and persistence', () => {
    const setItemSpy = vi.spyOn(Storage.prototype, 'setItem');
    const { result } = renderStoredStateHook({
      normalize: (value) => value.trim().toUpperCase(),
    });

    act(() => {
      result.current[1]('  next  ');
    });

    expect(result.current[0]).toBe('NEXT');
    expect(setItemSpy).toHaveBeenLastCalledWith(STORAGE_KEY, 'v:NEXT');
  });

  it('updates state and warns when writing storage throws', () => {
    window.localStorage.setItem(STORAGE_KEY, 'v:seed');
    const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});
    vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
      throw new Error('boom');
    });

    const { result } = renderStoredStateHook();

    act(() => {
      result.current[1]('next');
    });

    expect(result.current[0]).toBe('next');
    expect(warnSpy).toHaveBeenCalledWith(WRITE_ERROR_MESSAGE, expect.any(Error));
  });
});
