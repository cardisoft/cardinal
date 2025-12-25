import React, {
  useRef,
  useState,
  useCallback,
  useMemo,
  useLayoutEffect,
  useEffect,
  forwardRef,
  useImperativeHandle,
} from 'react';
import type { CSSProperties, UIEvent as ReactUIEvent } from 'react';
import Scrollbar from './Scrollbar';
import { useDataLoader } from '../hooks/useDataLoader';
import type { SearchResultItem } from '../types/search';
import type { SlabIndex } from '../types/slab';
import { useIconViewport } from '../hooks/useIconViewport';
import { CONTAINER_PADDING } from '../constants';

export type VirtualGridHandle = {
  scrollToTop: () => void;
  scrollToRow: (rowIndex: number, align?: 'nearest' | 'start' | 'end' | 'center') => void;
  ensureRangeLoaded: (startIndex: number, endIndex: number) => Promise<void> | void;
  getItem: (index: number) => SearchResultItem | undefined;
  getColumnsCount: () => number;
};

type VirtualGridProps = {
  results?: SlabIndex[];
  overscanRows?: number;
  renderItem: (
    index: number,
    item: SearchResultItem | undefined,
    style: CSSProperties,
    actualItemWidth: number,
  ) => React.ReactNode;
  itemWidth: number;
  itemHeight: number;
  onBulkSelect?: (indices: number[]) => void;
  onSelect?: (
    rowIndex: number,
    options: { isShift: boolean; isMeta: boolean; isCtrl: boolean },
  ) => void;
  className?: string;
  iconSize: number;
};

export const VirtualGrid = forwardRef<VirtualGridHandle, VirtualGridProps>(function VirtualGrid(
  {
    results = [],
    overscanRows = 6,
    renderItem,
    itemWidth,
    itemHeight,
    onBulkSelect,
    onSelect,
    className = '',
    iconSize,
  },
  ref,
) {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const [scrollTop, setScrollTop] = useState(0);
  const [viewportHeight, setViewportHeight] = useState(0);
  const [viewportWidth, setViewportWidth] = useState(0);
  const [marqueeStart, setMarqueeStart] = useState<{ x: number; y: number } | null>(null);
  const [marqueeEnd, setMarqueeEnd] = useState<{ x: number; y: number } | null>(null);

  const rowCount = results.length;
  const { cache, ensureRangeLoaded } = useDataLoader(results);

  const gridPadding = 12; // Matches .scroll-area padding in App.css
  const availableWidth = Math.max(0, viewportWidth - 2 * gridPadding);
  const columnsCount = Math.max(1, Math.floor(availableWidth / itemWidth));
  const actualItemWidth = availableWidth / columnsCount;
  const leftOffset = gridPadding;

  const rowsCountTotal = Math.ceil(rowCount / columnsCount);
  const totalHeight = rowsCountTotal * itemHeight;

  const maxScrollTop = Math.max(0, totalHeight - viewportHeight);

  const startRow = Math.max(0, Math.floor(scrollTop / itemHeight) - overscanRows);
  const endRow = Math.min(
    rowsCountTotal - 1,
    Math.ceil((scrollTop + viewportHeight) / itemHeight) + overscanRows - 1,
  );

  const startIndex = startRow * columnsCount;
  const endIndex = Math.min(rowCount - 1, (endRow + 1) * columnsCount - 1);

  useIconViewport({ results, start: startIndex, end: endIndex, iconSize });

  const updateScrollAndRange = useCallback(
    (updater: (value: number) => number) => {
      setScrollTop((prev) => {
        const nextValue = updater(prev);
        const clamped = Math.max(0, Math.min(nextValue, maxScrollTop));
        return prev === clamped ? prev : clamped;
      });
    },
    [maxScrollTop],
  );

  const handleWheel = useCallback(
    (e: React.WheelEvent<HTMLDivElement>) => {
      e.preventDefault();
      updateScrollAndRange((prev) => prev + e.deltaY);
      if (marqueeStart && marqueeEnd) {
        setMarqueeEnd((prev) => (prev ? { ...prev, y: prev.y + e.deltaY } : null));
      }
    },
    [updateScrollAndRange, marqueeStart, marqueeEnd],
  );

  const handleMouseDown = useCallback(
    (e: React.MouseEvent<HTMLDivElement>) => {
      if (e.button !== 0) return;

      // Check if we clicked on an item or empty space
      const target = e.target as HTMLElement;
      const isItemInside = target.closest('.grid-item');

      if (isItemInside) return;

      const rect = containerRef.current?.getBoundingClientRect();
      if (!rect) return;

      const x = e.clientX - rect.left;
      const y = e.clientY - rect.top + scrollTop;

      setMarqueeStart({ x, y });
      setMarqueeEnd({ x, y });

      if (!e.metaKey && !e.ctrlKey && !e.shiftKey) {
        onBulkSelect?.([]);
      }
    },
    [scrollTop, onBulkSelect],
  );

  const handleMouseMove = useCallback(
    (e: React.MouseEvent<HTMLDivElement>) => {
      if (!marqueeStart) return;

      const rect = containerRef.current?.getBoundingClientRect();
      if (!rect) return;

      const x = e.clientX - rect.left;
      const y = e.clientY - rect.top + scrollTop;

      setMarqueeEnd({ x, y });

      // Calculate selected indices
      const x1 = Math.min(marqueeStart.x, x);
      const y1 = Math.min(marqueeStart.y, y);
      const x2 = Math.max(marqueeStart.x, x);
      const y2 = Math.max(marqueeStart.y, y);

      const startCol = Math.max(0, Math.floor((x1 - leftOffset) / actualItemWidth));
      const endCol = Math.min(
        columnsCount - 1,
        Math.floor((x2 - 1 - leftOffset) / actualItemWidth),
      );
      const startRow = Math.max(0, Math.floor(y1 / itemHeight));
      const endRow = Math.min(rowsCountTotal - 1, Math.floor((y2 - 1) / itemHeight));

      const selectedIndices: number[] = [];
      for (let r = startRow; r <= endRow; r++) {
        for (let c = startCol; c <= endCol; c++) {
          if (c < 0) continue;
          const index = r * columnsCount + c;
          if (index < rowCount) {
            selectedIndices.push(index);
          }
        }
      }

      onBulkSelect?.(selectedIndices);
    },
    [
      marqueeStart,
      scrollTop,
      rowCount,
      rowsCountTotal,
      columnsCount,
      actualItemWidth,
      itemHeight,
      leftOffset,
      onBulkSelect,
    ],
  );

  const handleMouseUp = useCallback(() => {
    setMarqueeStart(null);
    setMarqueeEnd(null);
  }, []);

  useEffect(() => {
    if (marqueeStart) {
      window.addEventListener('mouseup', handleMouseUp);
      return () => window.removeEventListener('mouseup', handleMouseUp);
    }
  }, [marqueeStart, handleMouseUp]);

  useEffect(() => {
    if (endIndex >= startIndex) ensureRangeLoaded(startIndex, endIndex);
  }, [startIndex, endIndex, ensureRangeLoaded, results]);

  useLayoutEffect(() => {
    const container = containerRef.current;
    if (!container) return;
    const updateViewport = () => {
      setViewportHeight(container.clientHeight);
      setViewportWidth(container.clientWidth);
    };
    const resizeObserver = new ResizeObserver(updateViewport);
    resizeObserver.observe(container);
    updateViewport();
    return () => resizeObserver.disconnect();
  }, []);

  useEffect(() => {
    setScrollTop((prev) => {
      const clamped = Math.max(0, Math.min(prev, maxScrollTop));
      return clamped === prev ? prev : clamped;
    });
  }, [maxScrollTop]);

  const scrollToRow = useCallback(
    (rowIndex: number, align: 'nearest' | 'start' | 'end' | 'center' = 'nearest') => {
      if (!Number.isFinite(rowIndex) || rowCount === 0) return;

      const targetRow = Math.floor(rowIndex / columnsCount);
      const rowTop = targetRow * itemHeight;
      const rowBottom = rowTop + itemHeight;

      updateScrollAndRange((prev) => {
        if (viewportHeight <= 0) return rowTop;

        const viewportTop = prev;
        const viewportBottom = viewportTop + viewportHeight;

        switch (align) {
          case 'start':
            return rowTop;
          case 'end':
            return rowBottom - viewportHeight;
          case 'center':
            return rowTop - Math.max(0, (viewportHeight - itemHeight) / 2);
          case 'nearest':
          default: {
            const tolerance = 40; // More forgiving tolerance to prevent jitter
            if (rowTop < viewportTop - tolerance) return rowTop;
            if (rowBottom > viewportBottom + tolerance) return rowBottom - viewportHeight;
            return prev;
          }
        }
      });
    },
    [rowCount, columnsCount, viewportHeight, itemHeight, updateScrollAndRange],
  );

  useImperativeHandle(
    ref,
    () => ({
      scrollToTop: () => updateScrollAndRange(() => 0),
      scrollToRow,
      ensureRangeLoaded,
      getItem: (index: number) => cache.get(index),
      getColumnsCount: () => columnsCount,
    }),
    [updateScrollAndRange, scrollToRow, ensureRangeLoaded, cache, columnsCount],
  );

  const renderedItems = useMemo(() => {
    if (endRow < startRow) return null;

    const items = [];
    for (let r = startRow; r <= endRow; r++) {
      for (let c = 0; c < columnsCount; c++) {
        const index = r * columnsCount + c;
        if (index >= rowCount) break;

        const item = cache.get(index);
        const style: CSSProperties = {
          position: 'absolute',
          top: r * itemHeight,
          left: leftOffset + c * actualItemWidth,
          width: actualItemWidth,
          height: itemHeight,
        };

        items.push(renderItem(index, item, style, actualItemWidth));
      }
    }
    return items;
  }, [
    startRow,
    endRow,
    columnsCount,
    rowCount,
    cache,
    renderItem,
    actualItemWidth,
    itemHeight,
    leftOffset,
  ]);

  return (
    <div
      ref={containerRef}
      className={`virtual-grid ${className}`}
      onWheel={handleWheel}
      onMouseDown={handleMouseDown}
      onMouseMove={handleMouseMove}
      role="list"
    >
      <div className="virtual-grid-viewport">
        <div
          className="virtual-grid-items"
          style={{
            height: totalHeight,
            position: 'relative',
            transform: `translate3d(0, ${-scrollTop}px, 0)`,
            willChange: 'transform',
          }}
        >
          {renderedItems}
        </div>
        {marqueeStart && marqueeEnd && (
          <div
            className="selection-marquee"
            style={{
              left: Math.min(marqueeStart.x, marqueeEnd.x),
              top: Math.min(marqueeStart.y, marqueeEnd.y) - scrollTop,
              width: Math.abs(marqueeStart.x - marqueeEnd.x),
              height: Math.abs(marqueeStart.y - marqueeEnd.y),
              zIndex: 1000,
            }}
          />
        )}
      </div>
      <Scrollbar
        totalHeight={totalHeight}
        viewportHeight={viewportHeight}
        maxScrollTop={maxScrollTop}
        scrollTop={scrollTop}
        onScrollUpdate={updateScrollAndRange}
      />
    </div>
  );
});

VirtualGrid.displayName = 'VirtualGrid';

export default VirtualGrid;
