import { renderHook, act } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { useSearchHistory } from '../useSearchHistory';

const enterValue = (update: (value: string) => void, value: string) => {
  act(() => {
    update(value);
  });
};

const navigateHistory = (
  navigate: (direction: 'older' | 'newer') => string | null,
  direction: 'older' | 'newer',
): string | null => {
  let result: string | null = null;
  act(() => {
    result = navigate(direction);
  });
  return result;
};

describe('useSearchHistory', () => {
  it('returns to the latest buffer after editing a historical entry', () => {
    const { result } = renderHook(() => useSearchHistory({ maxEntries: 10 }));

    const type = (value: string) => enterValue(result.current.handleInputChange, value);

    // Build history entries: a -> b -> c -> d -> e
    type('a');
    type('');
    type('b');
    type('');
    type('c');
    type('');
    type('d');
    type('');
    type('e');

    expect(navigateHistory(result.current.navigate, 'older')).toBe('d');
    expect(navigateHistory(result.current.navigate, 'newer')).toBe('e');
    expect(navigateHistory(result.current.navigate, 'newer')).toBeNull();

    expect(navigateHistory(result.current.navigate, 'older')).toBe('d');

    type('da');

    expect(navigateHistory(result.current.navigate, 'older')).toBe('e');
    expect(navigateHistory(result.current.navigate, 'newer')).toBe('da');
    expect(navigateHistory(result.current.navigate, 'newer')).toBeNull();
  });

  it('preserves the previous entry when the first letter changes', () => {
    const { result } = renderHook(() => useSearchHistory({ maxEntries: 10 }));
    const type = (value: string) => enterValue(result.current.handleInputChange, value);

    type('foo');
    type('b');
    type('ba');
    type('bar');

    expect(navigateHistory(result.current.navigate, 'older')).toBe('foo');
    expect(navigateHistory(result.current.navigate, 'newer')).toBe('bar');
  });
});
