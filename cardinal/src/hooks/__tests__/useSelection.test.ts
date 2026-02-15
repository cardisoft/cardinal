import { renderHook, act } from '@testing-library/react';
import type { MutableRefObject } from 'react';
import { describe, expect, it } from 'vitest';
import type { VirtualListHandle } from '../../components/VirtualList';
import type { SearchResultItem } from '../../types/search';
import type { SlabIndex } from '../../types/slab';
import { useSelection } from '../useSelection';

const createVirtualListRef = (
  overrides?: Partial<VirtualListHandle>,
): MutableRefObject<VirtualListHandle | null> => ({
  current: {
    scrollToTop: () => {},
    scrollToRow: () => {},
    ensureRangeLoaded: () => {},
    getItem: (index: number) => ({ path: `item-${index}` }) as SearchResultItem,
    ...overrides,
  },
});

type SelectOptions = {
  isShift?: boolean;
  isMeta?: boolean;
  isCtrl?: boolean;
};

const renderSelection = (initial: number[], initialVersion = 0) => {
  const virtualListRef = createVirtualListRef();
  let currentResults = initial as unknown as SlabIndex[];
  let version = initialVersion;
  const hook = renderHook(
    ({ results, version: activeVersion }: { results: SlabIndex[]; version: number }) =>
      useSelection(results, activeVersion, virtualListRef),
    { initialProps: { results: currentResults, version } },
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

  const rerenderResults = (next: number[], options?: { bumpVersion?: boolean }) => {
    currentResults = next as unknown as SlabIndex[];
    act(() => {
      if (options?.bumpVersion !== false) {
        version += 1;
      }
      hook.rerender({ results: currentResults, version });
    });
  };

  const bumpVersion = () => {
    act(() => {
      version += 1;
      hook.rerender({ results: currentResults, version });
    });
  };

  return { ...hook, selectRow, rerenderResults, bumpVersion };
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

  it('resets when only the version increments', () => {
    const { result, selectRow, bumpVersion } = renderSelection([0, 1, 2, 3, 4]);

    selectRow(1);
    expect(result.current.selectedIndices).toEqual([1]);

    bumpVersion();

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
    expect(result.current.shiftAnchorIndex).toBe(1);

    selectRow(5, { isShift: true });
    expect(result.current.selectedIndices).toEqual([1, 2, 3, 4, 5]);
    expect(result.current.shiftAnchorIndex).toBe(1);

    selectRow(6, { isCtrl: true });
    expect(result.current.selectedIndices).toEqual([1, 2, 3, 4, 5, 6]);
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

  it('extends selection when moveSelection is called with extend=true', () => {
    const { result, selectRow } = renderSelection([0, 1, 2, 3, 4, 5]);

    selectRow(2);
    expect(result.current.selectedIndices).toEqual([2]);

    act(() => {
      result.current.moveSelection(1, { extend: true });
    });
    expect(result.current.selectedIndices).toEqual([2, 3]);
    expect(result.current.shiftAnchorIndex).toBe(2);

    act(() => {
      result.current.moveSelection(1, { extend: true });
    });
    expect(result.current.selectedIndices).toEqual([2, 3, 4]);

    act(() => {
      result.current.moveSelection(-1, { extend: true });
    });
    expect(result.current.selectedIndices).toEqual([2, 3]);

    act(() => {
      result.current.moveSelection(-1, { extend: true });
    });
    expect(result.current.selectedIndices).toEqual([2]);
    expect(result.current.shiftAnchorIndex).toBe(2);
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

  describe('shift selection edge cases', () => {
    it('creates a range in reverse order (higher anchor to lower target)', () => {
      const { result, selectRow } = renderSelection([0, 1, 2, 3, 4, 5]);

      selectRow(4);
      expect(result.current.shiftAnchorIndex).toBe(4);

      selectRow(1, { isShift: true });

      expect(result.current.selectedIndices).toEqual([1, 2, 3, 4]);
      expect(result.current.shiftAnchorIndex).toBe(4);
    });

    it('selects a single-item range when shift-clicking the anchor itself', () => {
      const { result, selectRow } = renderSelection([0, 1, 2, 3]);

      selectRow(2);
      selectRow(2, { isShift: true });

      expect(result.current.selectedIndices).toEqual([2]);
      expect(result.current.shiftAnchorIndex).toBe(2);
    });

    it('extends range to the first item when shift-clicking index 0', () => {
      const { result, selectRow } = renderSelection([0, 1, 2, 3, 4]);

      selectRow(3);
      selectRow(0, { isShift: true });

      expect(result.current.selectedIndices).toEqual([0, 1, 2, 3]);
    });

    it('extends range to the last item when shift-clicking the final index', () => {
      const { result, selectRow } = renderSelection([0, 1, 2, 3, 4]);

      selectRow(1);
      selectRow(4, { isShift: true });

      expect(result.current.selectedIndices).toEqual([1, 2, 3, 4]);
    });

    it('replaces previous shift range when creating a new one from the same anchor', () => {
      const { result, selectRow } = renderSelection([0, 1, 2, 3, 4, 5]);

      selectRow(2);
      selectRow(4, { isShift: true });
      expect(result.current.selectedIndices).toEqual([2, 3, 4]);

      selectRow(0, { isShift: true });
      expect(result.current.selectedIndices).toEqual([0, 1, 2]);
      expect(result.current.shiftAnchorIndex).toBe(2);
    });
  });

  describe('cmd/ctrl toggle edge cases', () => {
    it('removes the only selected item when cmd-clicking it', () => {
      const { result, selectRow } = renderSelection([0, 1, 2]);

      selectRow(1);
      expect(result.current.selectedIndices).toEqual([1]);

      selectRow(1, { isMeta: true });
      expect(result.current.selectedIndices).toEqual([]);
      expect(result.current.shiftAnchorIndex).toBeNull();
    });

    it('supports adding multiple items with consecutive cmd-clicks', () => {
      const { result, selectRow } = renderSelection([0, 1, 2, 3, 4]);

      selectRow(0);
      selectRow(2, { isMeta: true });
      selectRow(4, { isMeta: true });

      expect(result.current.selectedIndices).toEqual([0, 2, 4]);
      expect(result.current.shiftAnchorIndex).toBe(4);
    });

    it('treats ctrl modifier the same as cmd for toggling', () => {
      const { result, selectRow } = renderSelection([0, 1, 2, 3]);

      selectRow(1);
      selectRow(3, { isCtrl: true });

      expect(result.current.selectedIndices).toEqual([1, 3]);
      expect(result.current.shiftAnchorIndex).toBe(3);
    });

    it('moves anchor when deselecting the anchor in a multi-selection', () => {
      const { result, selectRow } = renderSelection([0, 1, 2, 3, 4]);

      selectRow(1);
      selectRow(2, { isMeta: true });
      selectRow(3, { isMeta: true });
      expect(result.current.selectedIndices).toEqual([1, 2, 3]);
      expect(result.current.shiftAnchorIndex).toBe(3);

      // Deselect anchor (3), should move to next below (none), then up to 2
      selectRow(3, { isMeta: true });
      expect(result.current.selectedIndices).toEqual([1, 2]);
      expect(result.current.shiftAnchorIndex).toBe(2);
    });

    it('moves shift anchor to the nearest selected item below when deselecting the anchor', () => {
      const { result, selectRow } = renderSelection([0, 1, 2, 3, 4, 5]);

      selectRow(1);
      selectRow(3, { isMeta: true });
      selectRow(5, { isMeta: true });
      expect(result.current.selectedIndices).toEqual([1, 3, 5]);
      expect(result.current.shiftAnchorIndex).toBe(5);

      // Deselect the anchor (5), should move to next below (none), then check upward (3)
      selectRow(5, { isMeta: true });
      expect(result.current.selectedIndices).toEqual([1, 3]);
      expect(result.current.shiftAnchorIndex).toBe(3);
    });

    it('moves shift anchor downward to next selected item when available', () => {
      const { result, selectRow } = renderSelection([0, 1, 2, 3, 4, 5, 6, 7]);

      selectRow(1);
      selectRow(3, { isMeta: true });
      selectRow(5, { isMeta: true });
      selectRow(7, { isMeta: true });
      expect(result.current.selectedIndices).toEqual([1, 3, 5, 7]);
      expect(result.current.shiftAnchorIndex).toBe(7);

      // Change anchor by adding 5
      selectRow(5, { isMeta: true });
      selectRow(5, { isMeta: true });
      expect(result.current.shiftAnchorIndex).toBe(5);

      // Deselect the anchor (5), should move to next below (7), not upward
      selectRow(5, { isMeta: true });
      expect(result.current.selectedIndices).toEqual([1, 3, 7]);
      expect(result.current.shiftAnchorIndex).toBe(7);
    });

    it('moves shift anchor downward when deselecting the anchor in the middle', () => {
      const { result, selectRow } = renderSelection([0, 1, 2, 3, 4, 5, 6]);

      selectRow(1);
      selectRow(3, { isMeta: true });
      selectRow(5, { isMeta: true });
      expect(result.current.shiftAnchorIndex).toBe(5);

      // Deselect row 3, but it's not the anchor, so anchor stays at 5
      selectRow(3, { isMeta: true });
      expect(result.current.shiftAnchorIndex).toBe(5);

      // Now deselect the current anchor (5), should move to the next selected below (none), then up to 1
      selectRow(5, { isMeta: true });
      expect(result.current.selectedIndices).toEqual([1]);
      expect(result.current.shiftAnchorIndex).toBe(1);
    });

    it('clears shift anchor when deselecting the last remaining anchor', () => {
      const { result, selectRow } = renderSelection([0, 1, 2, 3, 4]);

      selectRow(2);
      expect(result.current.shiftAnchorIndex).toBe(2);

      selectRow(2, { isMeta: true });
      expect(result.current.selectedIndices).toEqual([]);
      expect(result.current.shiftAnchorIndex).toBeNull();
    });

    it('builds a shift range from the last cmd-toggled anchor', () => {
      const { result, selectRow } = renderSelection([0, 1, 2, 3, 4, 5, 6]);

      selectRow(1);
      selectRow(3, { isMeta: true });
      expect(result.current.shiftAnchorIndex).toBe(3);

      selectRow(5, { isShift: true });
      expect(result.current.selectedIndices).toEqual([3, 4, 5]);
    });

    it('uses the new anchor after deselecting the old anchor', () => {
      const { result, selectRow } = renderSelection([0, 1, 2, 3, 4, 5, 6, 7]);

      selectRow(2);
      selectRow(4, { isMeta: true });
      selectRow(6, { isMeta: true });
      expect(result.current.selectedIndices).toEqual([2, 4, 6]);
      expect(result.current.shiftAnchorIndex).toBe(6);

      // Deselect the anchor (6), anchor moves to next selected below (none), then up to 4
      selectRow(6, { isMeta: true });
      expect(result.current.shiftAnchorIndex).toBe(4);

      // Now shift-select from the new anchor (4) to 7
      selectRow(7, { isShift: true });
      expect(result.current.selectedIndices).toEqual([4, 5, 6, 7]);
    });
  });

  describe('moveSelection edge cases', () => {
    it('does nothing when moving in an empty list', () => {
      const { result } = renderSelection([]);

      act(() => {
        result.current.moveSelection(1);
      });

      expect(result.current.selectedIndices).toEqual([]);
      expect(result.current.activeRowIndex).toBeNull();
    });

    it('starts at index 0 when moving down with no active selection', () => {
      const { result } = renderSelection([0, 1, 2, 3]);

      act(() => {
        result.current.moveSelection(1);
      });

      expect(result.current.selectedIndices).toEqual([0]);
      expect(result.current.activeRowIndex).toBe(0);
    });

    it('starts at the last index when moving up with no active selection', () => {
      const { result } = renderSelection([0, 1, 2, 3, 4]);

      act(() => {
        result.current.moveSelection(-1);
      });

      expect(result.current.selectedIndices).toEqual([4]);
      expect(result.current.activeRowIndex).toBe(4);
    });

    it('clamps to the first item when moving up from index 0', () => {
      const { result, selectRow } = renderSelection([0, 1, 2, 3]);

      selectRow(0);

      act(() => {
        result.current.moveSelection(-1);
      });

      expect(result.current.selectedIndices).toEqual([0]);
      expect(result.current.activeRowIndex).toBe(0);
    });

    it('clamps to the last item when moving down from the end', () => {
      const { result, selectRow } = renderSelection([0, 1, 2, 3, 4]);

      selectRow(4);

      act(() => {
        result.current.moveSelection(1);
      });

      expect(result.current.selectedIndices).toEqual([4]);
      expect(result.current.activeRowIndex).toBe(4);
    });

    it('replaces multi-selection with single selection when navigating', () => {
      const { result, selectRow } = renderSelection([0, 1, 2, 3, 4]);

      selectRow(1);
      selectRow(3, { isShift: true });
      expect(result.current.selectedIndices).toEqual([1, 2, 3]);

      act(() => {
        result.current.moveSelection(1);
      });

      expect(result.current.selectedIndices).toEqual([4]);
      expect(result.current.activeRowIndex).toBe(4);
    });

    it('does nothing when getItem returns undefined', () => {
      const virtualListRef = createVirtualListRef({ getItem: () => undefined });

      const hook = renderHook(() =>
        useSelection([0, 1, 2] as unknown as SlabIndex[], 0, virtualListRef),
      );

      act(() => {
        hook.result.current.moveSelection(1);
      });

      expect(hook.result.current.selectedIndices).toEqual([]);
      expect(hook.result.current.activeRowIndex).toBeNull();
    });
  });

  describe('version changes and selection reset', () => {
    it('does not reset when version changes but selection is already empty', () => {
      const { result, bumpVersion } = renderSelection([0, 1, 2, 3]);

      expect(result.current.selectedIndices).toEqual([]);
      expect(result.current.activeRowIndex).toBeNull();

      bumpVersion();

      expect(result.current.selectedIndices).toEqual([]);
      expect(result.current.activeRowIndex).toBeNull();
    });

    it('resets selection when version changes even with no result changes', () => {
      const { result, selectRow, rerenderResults } = renderSelection([0, 1, 2, 3]);

      selectRow(2);
      expect(result.current.selectedIndices).toEqual([2]);

      rerenderResults([0, 1, 2, 3], { bumpVersion: true });

      expect(result.current.selectedIndices).toEqual([]);
      expect(result.current.activeRowIndex).toBeNull();
      expect(result.current.shiftAnchorIndex).toBeNull();
    });

    it('preserves selection when rerendering without version bump', () => {
      const { result, selectRow, rerenderResults } = renderSelection([0, 1, 2, 3]);

      selectRow(2);
      expect(result.current.selectedIndices).toEqual([2]);

      rerenderResults([0, 1, 2, 3], { bumpVersion: false });

      expect(result.current.selectedIndices).toEqual([2]);
      expect(result.current.activeRowIndex).toBe(2);
      expect(result.current.shiftAnchorIndex).toBe(2);
    });

    it('resets multi-selection and anchor state on version bump', () => {
      const { result, selectRow, bumpVersion } = renderSelection([0, 1, 2, 3, 4, 5]);

      selectRow(1);
      selectRow(4, { isShift: true });
      expect(result.current.selectedIndices).toEqual([1, 2, 3, 4]);
      expect(result.current.shiftAnchorIndex).toBe(1);

      bumpVersion();

      expect(result.current.selectedIndices).toEqual([]);
      expect(result.current.activeRowIndex).toBeNull();
      expect(result.current.shiftAnchorIndex).toBeNull();
    });
  });

  describe('selectedPaths computation', () => {
    it('returns an empty array when no items are selected', () => {
      const { result } = renderSelection([0, 1, 2]);

      expect(result.current.selectedPaths).toEqual([]);
    });

    it('returns paths for all selected indices', () => {
      const { result, selectRow } = renderSelection([0, 1, 2, 3, 4]);

      selectRow(1);
      selectRow(3, { isMeta: true });

      expect(result.current.selectedPaths).toEqual(['item-1', 'item-3']);
    });

    it('returns an empty array when virtualListRef is null', () => {
      const virtualListRef: MutableRefObject<VirtualListHandle | null> = { current: null };

      const hook = renderHook(() =>
        useSelection([0, 1, 2] as unknown as SlabIndex[], 0, virtualListRef),
      );

      act(() => {
        hook.result.current.selectSingleRow(1);
      });

      expect(hook.result.current.selectedPaths).toEqual([]);
    });

    it('skips items without paths when computing selectedPaths', () => {
      const virtualListRef = createVirtualListRef({
        getItem: (index: number) => {
          if (index === 1) {
            return {} as SearchResultItem;
          }
          return { path: `item-${index}` } as SearchResultItem;
        },
      });

      const hook = renderHook(() =>
        useSelection([0, 1, 2, 3] as unknown as SlabIndex[], 0, virtualListRef),
      );

      act(() => {
        hook.result.current.selectSingleRow(0);
      });
      act(() => {
        hook.result.current.handleRowSelect(1, {
          isShift: false,
          isMeta: true,
          isCtrl: false,
        });
      });
      act(() => {
        hook.result.current.handleRowSelect(2, {
          isShift: false,
          isMeta: true,
          isCtrl: false,
        });
      });

      expect(hook.result.current.selectedPaths).toEqual(['item-0', 'item-2']);
    });

    it('updates selectedPaths when selection changes', () => {
      const { result, selectRow } = renderSelection([0, 1, 2, 3]);

      selectRow(0);
      expect(result.current.selectedPaths).toEqual(['item-0']);

      selectRow(2, { isMeta: true });
      expect(result.current.selectedPaths).toEqual(['item-0', 'item-2']);

      selectRow(0, { isMeta: true });
      expect(result.current.selectedPaths).toEqual(['item-2']);
    });
  });

  describe('selectedIndicesRef synchronization', () => {
    it('keeps selectedIndicesRef in sync with selectedIndices state', () => {
      const { result, selectRow } = renderSelection([0, 1, 2, 3]);

      expect(result.current.selectedIndicesRef.current).toEqual([]);

      selectRow(1);
      expect(result.current.selectedIndicesRef.current).toEqual([1]);

      selectRow(3, { isMeta: true });
      expect(result.current.selectedIndicesRef.current).toEqual([1, 3]);

      act(() => {
        result.current.clearSelection();
      });
      expect(result.current.selectedIndicesRef.current).toEqual([]);
    });
  });

  describe('complex interaction sequences', () => {
    it('handles cmd-toggle -> shift-extend -> normal-click sequence', () => {
      const { result, selectRow } = renderSelection([0, 1, 2, 3, 4, 5, 6, 7]);

      selectRow(2);
      selectRow(4, { isMeta: true });
      expect(result.current.selectedIndices).toEqual([2, 4]);

      selectRow(6, { isShift: true });
      expect(result.current.selectedIndices).toEqual([4, 5, 6]);

      selectRow(1);
      expect(result.current.selectedIndices).toEqual([1]);
      expect(result.current.shiftAnchorIndex).toBe(1);
    });

    it('handles shift-extend -> cmd-toggle -> shift-extend from new anchor', () => {
      const { result, selectRow } = renderSelection([0, 1, 2, 3, 4, 5, 6, 7, 8]);

      selectRow(2);
      selectRow(4, { isShift: true });
      expect(result.current.selectedIndices).toEqual([2, 3, 4]);

      selectRow(6, { isMeta: true });
      expect(result.current.selectedIndices).toEqual([2, 3, 4, 6]);
      expect(result.current.shiftAnchorIndex).toBe(6);

      selectRow(8, { isShift: true });
      expect(result.current.selectedIndices).toEqual([6, 7, 8]);
    });

    it('handles moveSelection after cmd-toggle multi-selection', () => {
      const { result, selectRow } = renderSelection([0, 1, 2, 3, 4, 5, 6]);

      selectRow(1);
      selectRow(3, { isMeta: true });
      selectRow(5, { isMeta: true });
      expect(result.current.selectedIndices).toEqual([1, 3, 5]);
      expect(result.current.activeRowIndex).toBe(5);

      act(() => {
        result.current.moveSelection(1);
      });

      expect(result.current.selectedIndices).toEqual([6]);
      expect(result.current.activeRowIndex).toBe(6);
    });

    it('preserves activeRowIndex through cmd toggles', () => {
      const { result, selectRow } = renderSelection([0, 1, 2, 3, 4]);

      selectRow(1);
      expect(result.current.activeRowIndex).toBe(1);

      selectRow(3, { isMeta: true });
      expect(result.current.activeRowIndex).toBe(3);

      selectRow(3, { isMeta: true });
      expect(result.current.activeRowIndex).toBe(3);
    });

    it('handles selectSingleRow replacing existing multi-selection', () => {
      const { result, selectRow } = renderSelection([0, 1, 2, 3, 4]);

      selectRow(1);
      selectRow(3, { isShift: true });
      expect(result.current.selectedIndices).toEqual([1, 2, 3]);

      act(() => {
        result.current.selectSingleRow(4);
      });

      expect(result.current.selectedIndices).toEqual([4]);
      expect(result.current.activeRowIndex).toBe(4);
      expect(result.current.shiftAnchorIndex).toBe(4);
    });
  });

  describe('single-item list edge cases', () => {
    it('handles selection in a single-item list', () => {
      const { result, selectRow } = renderSelection([0]);

      selectRow(0);
      expect(result.current.selectedIndices).toEqual([0]);
      expect(result.current.activeRowIndex).toBe(0);
    });

    it('handles moveSelection down in a single-item list', () => {
      const { result, selectRow } = renderSelection([0]);

      selectRow(0);

      act(() => {
        result.current.moveSelection(1);
      });

      expect(result.current.selectedIndices).toEqual([0]);
      expect(result.current.activeRowIndex).toBe(0);
    });

    it('handles moveSelection up in a single-item list', () => {
      const { result, selectRow } = renderSelection([0]);

      selectRow(0);

      act(() => {
        result.current.moveSelection(-1);
      });

      expect(result.current.selectedIndices).toEqual([0]);
      expect(result.current.activeRowIndex).toBe(0);
    });

    it('handles shift-click in a single-item list', () => {
      const { result, selectRow } = renderSelection([0]);

      selectRow(0);
      selectRow(0, { isShift: true });

      expect(result.current.selectedIndices).toEqual([0]);
    });
  });
});
