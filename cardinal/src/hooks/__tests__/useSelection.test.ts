import { renderHook, act } from '@testing-library/react';
import { createRef } from 'react';
import { describe, expect, it } from 'vitest';
import type { VirtualListHandle } from '../../components/VirtualList';
import type { SearchResultItem } from '../../types/search';
import { toSlabIndexArray } from '../../types/slab';
import { useSelection } from '../useSelection';

const createVirtualListRef = () => {
  const ref = createRef<VirtualListHandle>();
  ref.current = {
    scrollToTop: () => {},
    scrollToRow: () => {},
    ensureRangeLoaded: () => {},
    getItem: (index) => ({ path: `item-${index}` }) as SearchResultItem,
  };
  return ref;
};

type SelectOptions = {
  isShift?: boolean;
  isMeta?: boolean;
  isCtrl?: boolean;
};

const renderSelection = (initial: number[]) => {
  const virtualListRef = createVirtualListRef();
  const hook = renderHook(
    ({ results }: { results: ReturnType<typeof toSlabIndexArray> }) =>
      useSelection(results, virtualListRef),
    { initialProps: { results: toSlabIndexArray(initial) } },
  );

  const selectRow = (rowIndex: number, options: SelectOptions = {}) => {
    act(() => {
      hook.result.current.handleRowSelect(rowIndex, {
        isShift: false,
        isMeta: false,
        isCtrl: false,
        ...options,
      });
    });
  };

  const rerenderResults = (next: number[]) => {
    act(() => {
      hook.rerender({ results: toSlabIndexArray(next) });
    });
  };

  return { ...hook, selectRow, rerenderResults };
};

describe('useSelection', () => {
  it('keeps the original anchor when extending the selection with shift-click', () => {
    const { result, selectRow } = renderSelection([0, 1, 2, 3, 4, 5]);

    selectRow(2);
    expect(result.current.shiftAnchorIndex).toBe(2);

    selectRow(4, { isShift: true });

    expect(result.current.selectedIndices).toEqual([2, 3, 4]);
    expect(result.current.shiftAnchorIndex).toBe(2);
  });

  it('clears the selection state when displayed results refresh', () => {
    const { result, selectRow, rerenderResults } = renderSelection([0, 1, 2, 3, 4, 5]);

    selectRow(3);
    expect(result.current.shiftAnchorIndex).toBe(3);

    rerenderResults([9, 3, 0, 1, 2, 4, 5]);

    expect(result.current.selectedIndices).toEqual([]);
    expect(result.current.activeRowIndex).toBeNull();
    expect(result.current.shiftAnchorIndex).toBeNull();
  });

  it('supports cmd/ctrl toggles and mixing with shift selection', () => {
    const { result, selectRow } = renderSelection([0, 1, 2, 3, 4, 5, 6]);

    selectRow(1);
    expect(result.current.selectedIndices).toEqual([1]);

    selectRow(3, { isMeta: true });
    expect(result.current.selectedIndices).toEqual([1, 3]);
    expect(result.current.shiftAnchorIndex).toBe(3);

    selectRow(3, { isMeta: true });
    expect(result.current.selectedIndices).toEqual([1]);
    expect(result.current.shiftAnchorIndex).toBe(3);

    selectRow(5, { isShift: true });
    expect(result.current.selectedIndices).toEqual([3, 4, 5]);
    expect(result.current.shiftAnchorIndex).toBe(3);

    selectRow(6, { isCtrl: true });
    expect(result.current.selectedIndices).toEqual([3, 4, 5, 6]);
    expect(result.current.shiftAnchorIndex).toBe(6);
  });

  it('treats the first shift click as a normal selection when no anchor exists', () => {
    const { result, selectRow } = renderSelection([0, 1, 2, 3]);

    expect(result.current.shiftAnchorIndex).toBeNull();

    selectRow(3, { isShift: true });

    expect(result.current.selectedIndices).toEqual([3]);
    expect(result.current.shiftAnchorIndex).toBe(3);
  });

  it('resets selection and anchor when cleared', () => {
    const { result, selectRow } = renderSelection([0, 1, 2, 3, 4]);

    selectRow(2);
    expect(result.current.shiftAnchorIndex).toBe(2);

    act(() => {
      result.current.clearSelection();
    });

    expect(result.current.selectedIndices).toEqual([]);
    expect(result.current.shiftAnchorIndex).toBeNull();

    selectRow(4, { isShift: true });
    expect(result.current.selectedIndices).toEqual([4]);
    expect(result.current.shiftAnchorIndex).toBe(4);
  });

  it('navigates with moveSelection and updates anchor accordingly', () => {
    const { result } = renderSelection([0, 1, 2, 3, 4]);

    act(() => {
      result.current.moveSelection(1);
    });
    expect(result.current.selectedIndices).toEqual([0]);
    expect(result.current.shiftAnchorIndex).toBe(0);

    act(() => {
      result.current.moveSelection(1);
    });
    expect(result.current.selectedIndices).toEqual([1]);
    expect(result.current.shiftAnchorIndex).toBe(1);

    act(() => {
      result.current.moveSelection(-1);
    });
    expect(result.current.selectedIndices).toEqual([0]);
    expect(result.current.shiftAnchorIndex).toBe(0);
  });

  it('drops any active shift range when a refresh arrives mid-selection', () => {
    const { result, selectRow, rerenderResults } = renderSelection([0, 1, 2, 3, 4]);

    selectRow(1);
    selectRow(3, { isShift: true });
    expect(result.current.selectedIndices).toEqual([1, 2, 3]);

    rerenderResults([4, 5, 6, 7]);

    expect(result.current.selectedIndices).toEqual([]);
    expect(result.current.activeRowIndex).toBeNull();
    expect(result.current.shiftAnchorIndex).toBeNull();
  });

  it('selects the requested row via selectSingleRow helper', () => {
    const { result } = renderSelection([0, 1, 2, 3]);

    act(() => {
      result.current.selectSingleRow(1);
    });

    expect(result.current.selectedIndices).toEqual([1]);
    expect(result.current.shiftAnchorIndex).toBe(1);

    act(() => {
      result.current.selectSingleRow(3);
    });

    expect(result.current.selectedIndices).toEqual([3]);
    expect(result.current.shiftAnchorIndex).toBe(3);
  });
});
