import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { MutableRefObject, RefObject } from 'react';
import type { VirtualListHandle } from '../components/VirtualList';
import type { SlabIndex } from '../types/slab';
import type { SearchResultItem } from '../types/search';

type RowSelectOptions = {
  isShift: boolean;
  isMeta: boolean;
  isCtrl: boolean;
};

export type SelectionController = {
  selectedIndices: number[];
  selectedIndicesRef: MutableRefObject<number[]>;
  activeRowIndex: number | null;
  shiftAnchorIndex: number | null;
  selectedPaths: string[];
  handleRowSelect: (rowIndex: number, options: RowSelectOptions) => void;
  selectSingleRow: (rowIndex: number) => void;
  clearSelection: () => void;
  moveSelection: (delta: 1 | -1) => void;
};

/**
 * Manages virtualized row selection. Keeps indexes/anchors in sync with the rendered list,
 * exposes helpers for shift/meta selection, and remaps selections when the backing data changes.
 * The hook also tracks the concrete paths backing the selection so consumers can interact with
 * context menus, Quick Look, etc. without reimplementing bookkeeping.
 */
export const useSelection = (
  displayedResults: SlabIndex[],
  virtualListRef: RefObject<VirtualListHandle | null>,
): SelectionController => {
  const [selectedIndices, setSelectedIndices] = useState<number[]>([]);
  const [activeRowIndex, setActiveRowIndex] = useState<number | null>(null);
  const [shiftAnchorIndex, setShiftAnchorIndex] = useState<number | null>(null);
  const selectedIndicesRef = useRef<number[]>([]);
  const displayedResultsRef = useRef(displayedResults);

  const handleRowSelect = useCallback(
    (rowIndex: number, options: RowSelectOptions) => {
      const { isShift, isMeta, isCtrl } = options;
      const isCmdOrCtrl = isMeta || isCtrl;

      if (isShift && shiftAnchorIndex !== null) {
        const start = Math.min(shiftAnchorIndex, rowIndex);
        const end = Math.max(shiftAnchorIndex, rowIndex);
        const range: number[] = [];
        for (let i = start; i <= end; i += 1) {
          range.push(i);
        }
        setSelectedIndices(range);
      } else if (isCmdOrCtrl) {
        setSelectedIndices((prevIndices) => {
          if (prevIndices.includes(rowIndex)) {
            return prevIndices.filter((index) => index !== rowIndex);
          }
          return [...prevIndices, rowIndex];
        });
        setShiftAnchorIndex(rowIndex);
      } else {
        setSelectedIndices([rowIndex]);
        setShiftAnchorIndex(rowIndex);
      }

      setActiveRowIndex(rowIndex);
    },
    [displayedResults, shiftAnchorIndex],
  );

  const selectSingleRow = useCallback((rowIndex: number) => {
    setSelectedIndices([rowIndex]);
    setActiveRowIndex(rowIndex);
    setShiftAnchorIndex(rowIndex);
  }, [displayedResults]);

  const clearSelection = useCallback(() => {
    setSelectedIndices([]);
    setActiveRowIndex(null);
    setShiftAnchorIndex(null);
  }, []);

  const moveSelection = useCallback(
    (delta: 1 | -1) => {
      if (displayedResults.length === 0) {
        return;
      }

      const fallbackIndex = delta > 0 ? -1 : displayedResults.length;
      const baseIndex = activeRowIndex ?? fallbackIndex;
      const nextIndex = Math.min(Math.max(baseIndex + delta, 0), displayedResults.length - 1);

      if (nextIndex === activeRowIndex) {
        return;
      }

      const item = virtualListRef.current?.getItem?.(nextIndex);
      if (!item) {
        return;
      }

      handleRowSelect(nextIndex, {
        isShift: false,
        isMeta: false,
        isCtrl: false,
      });
    },
    [activeRowIndex, displayedResults.length, handleRowSelect],
  );

  useEffect(() => {
    selectedIndicesRef.current = selectedIndices;
  }, [selectedIndices]);

  useEffect(() => {
    const prev = displayedResultsRef.current;
    if (prev === displayedResults) {
      return;
    }
    displayedResultsRef.current = displayedResults;

    if (
      selectedIndicesRef.current.length === 0 &&
      activeRowIndex === null &&
      shiftAnchorIndex === null
    ) {
      return;
    }

    setSelectedIndices([]);
    setActiveRowIndex(null);
    setShiftAnchorIndex(null);
  }, [displayedResults, activeRowIndex, shiftAnchorIndex]);

  const selectedPaths = useMemo(() => {
    const list = virtualListRef.current;
    if (!list) {
      return [];
    }
    const paths: string[] = [];
    selectedIndices.forEach((index) => {
      const item = list.getItem?.(index) as SearchResultItem | undefined;
      if (item?.path) {
        paths.push(item.path);
      }
    });
    return paths;
  }, [selectedIndices, virtualListRef]);

  return {
    selectedIndices,
    selectedIndicesRef,
    activeRowIndex,
    shiftAnchorIndex,
    selectedPaths,
    handleRowSelect,
    selectSingleRow,
    clearSelection,
    moveSelection,
  };
};
