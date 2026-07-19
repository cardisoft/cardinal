import { act, renderHook } from '@testing-library/react';
import { beforeEach, describe, expect, it } from 'vitest';
import { useRecentSearches } from '../useRecentSearches';

const STORAGE_KEY = 'cardinal.recentSearches';

describe('useRecentSearches', () => {
  beforeEach(() => {
    window.localStorage.clear();
  });

  it('records queries newest-first and persists them', () => {
    const { result } = renderHook(() => useRecentSearches());

    act(() => {
      result.current.recordSearch('first');
    });
    act(() => {
      result.current.recordSearch('second');
    });

    expect(result.current.recentSearches).toEqual(['second', 'first']);
    expect(JSON.parse(window.localStorage.getItem(STORAGE_KEY) ?? '[]')).toEqual([
      'second',
      'first',
    ]);
  });

  it('ignores blank queries and trims whitespace', () => {
    const { result } = renderHook(() => useRecentSearches());

    act(() => {
      result.current.recordSearch('   ');
    });
    act(() => {
      result.current.recordSearch('  report.pdf  ');
    });

    expect(result.current.recentSearches).toEqual(['report.pdf']);
  });

  it('deduplicates by moving repeated queries to the front', () => {
    const { result } = renderHook(() => useRecentSearches());

    act(() => {
      result.current.recordSearch('alpha');
    });
    act(() => {
      result.current.recordSearch('beta');
    });
    act(() => {
      result.current.recordSearch('alpha');
    });

    expect(result.current.recentSearches).toEqual(['alpha', 'beta']);
  });

  it('caps stored entries at the configured maximum', () => {
    const { result } = renderHook(() => useRecentSearches({ maxEntries: 3 }));

    act(() => {
      result.current.recordSearch('one');
    });
    act(() => {
      result.current.recordSearch('two');
    });
    act(() => {
      result.current.recordSearch('three');
    });
    act(() => {
      result.current.recordSearch('four');
    });

    expect(result.current.recentSearches).toEqual(['four', 'three', 'two']);
  });

  it('restores persisted entries on mount and clears them', () => {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(['stored', 'earlier']));

    const { result } = renderHook(() => useRecentSearches());
    expect(result.current.recentSearches).toEqual(['stored', 'earlier']);

    act(() => {
      result.current.clearRecentSearches();
    });

    expect(result.current.recentSearches).toEqual([]);
    expect(JSON.parse(window.localStorage.getItem(STORAGE_KEY) ?? 'null')).toEqual([]);
  });

  it('ignores malformed persisted payloads', () => {
    window.localStorage.setItem(STORAGE_KEY, '{"not":"an array"}');

    const { result } = renderHook(() => useRecentSearches());
    expect(result.current.recentSearches).toEqual([]);
  });
});
