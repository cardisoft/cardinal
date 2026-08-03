import { act, renderHook } from '@testing-library/react';
import { beforeEach, describe, expect, it } from 'vitest';
import { COLUMN_ORDER_STORAGE_KEY, DEFAULT_COLUMN_ORDER, useColumnOrder } from '../useColumnOrder';

beforeEach(() => {
  window.localStorage.clear();
});

describe('useColumnOrder', () => {
  it('starts with the name first and the snippet right after it', () => {
    const { result } = renderHook(() => useColumnOrder());
    expect(result.current.columnOrder.slice(0, 2)).toEqual(['filename', 'context']);
  });

  it('moves a column to where the drop target sits, in both directions', () => {
    const { result } = renderHook(() => useColumnOrder());

    act(() => result.current.moveColumn('size', 'filename'));
    expect(result.current.columnOrder[0]).toBe('size');

    act(() => result.current.moveColumn('size', 'created'));
    expect(result.current.columnOrder[result.current.columnOrder.length - 1]).toBe('size');
  });

  it('persists the order and repairs a stored one that is missing a column', () => {
    window.localStorage.setItem(COLUMN_ORDER_STORAGE_KEY, JSON.stringify(['size', 'filename']));
    const { result } = renderHook(() => useColumnOrder());

    // Missing columns are appended rather than dropped, so a column added in a later version
    // still shows up for someone who already saved an order.
    expect(result.current.columnOrder).toHaveLength(DEFAULT_COLUMN_ORDER.length);
    expect(result.current.columnOrder.slice(0, 2)).toEqual(['size', 'filename']);
  });

  it('ignores a stored order containing junk', () => {
    window.localStorage.setItem(COLUMN_ORDER_STORAGE_KEY, JSON.stringify(['nope', 'filename']));
    const { result } = renderHook(() => useColumnOrder());
    expect(result.current.columnOrder).not.toContain('nope');
    expect(result.current.columnOrder).toHaveLength(DEFAULT_COLUMN_ORDER.length);
  });
});
