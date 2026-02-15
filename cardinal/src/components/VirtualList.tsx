// React & libs
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

export type VirtualListHandle = {
  scrollToTop: () => void;
  scrollToRow: (rowIndex: number, align?: 'nearest' | 'start' | 'end' | 'center') => void;
  ensureRangeLoaded: (startIndex: number, endIndex: number) => Promise<void> | void;
  getItem: (index: number) => SearchResultItem | undefined;
};

type VirtualListProps = {
  results?: SlabIndex[];
  resultsVersion: number;
  rowHeight?: number;
  overscan?: number;
  renderRow: (
    rowIndex: number,
    item: SearchResultItem | undefined,
    rowStyle: CSSProperties,
  ) => React.ReactNode;
  onScrollSync?: (scrollLeft: number) => void;
  className?: string;
};

// Virtualized list with lazy row hydration and synchronized column scrolling
export const VirtualList = forwardRef<VirtualListHandle, VirtualListProps>(function VirtualList(
  {
    results = [],
    resultsVersion,
    rowHeight = 24,
    overscan = 5,
    renderRow,
    onScrollSync,
    className = '',
  },
  ref,
) {
  // ----- refs -----
  const containerRef = useRef<HTMLDivElement | null>(null);

  // ----- state -----
  const [scrollTop, setScrollTop] = useState(0);
  const [viewportHeight, setViewportHeight] = useState(0);

  // ----- derived -----
  // Row count is inferred from the results array; explicit rowCount is no longer supported
  const resultsList = results;
  const rowCount = resultsList.length;

  // ----- data loader -----
  const { cache, ensureRangeLoaded } = useDataLoader(resultsList, resultsVersion);

  // Virtualized height powers the scrollbar math
  const totalHeight = rowCount * rowHeight;
  const maxScrollTop = Math.max(0, totalHeight - viewportHeight);

  // ----- callbacks: pure calculations first -----
  // Compute visible window (with overscan) based on the current scroll offset
  const start =
    rowCount && viewportHeight ? Math.max(0, Math.floor(scrollTop / rowHeight) - overscan) : 0;
  const end =
    rowCount && viewportHeight
      ? Math.min(rowCount - 1, Math.ceil((scrollTop + viewportHeight) / rowHeight) + overscan - 1)
      : -1;
  useIconViewport({ results: resultsList, start, end });

  // Clamp scroll updates so callers cannot push the viewport outside legal bounds
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

  // ----- event handlers -----
  // Normalise wheel deltas (line/page vs pixel) for consistent vertical scrolling
  const handleWheel = useCallback(
    (e: React.WheelEvent<HTMLDivElement>) => {
      e.preventDefault();
      const { deltaMode, deltaY } = e;
      let delta = deltaY;
      if (deltaMode === 1) {
        delta = deltaY * rowHeight;
      } else if (deltaMode === 2) {
        const pageSize = viewportHeight || rowHeight * 10;
        delta = deltaY * pageSize;
      }
      updateScrollAndRange((prev) => prev + delta);
    },
    [rowHeight, viewportHeight, updateScrollAndRange],
  );

  // Propagate horizontal scroll offset to the parent (keeps column headers aligned)
  const handleHorizontalScroll = useCallback(
    (e: ReactUIEvent<HTMLDivElement>) => {
      if (onScrollSync) onScrollSync((e.target as HTMLDivElement).scrollLeft);
    },
    [onScrollSync],
  );

  // ----- effects -----
  // Ensure the data cache stays warm for the active window
  useEffect(() => {
    if (end >= start) ensureRangeLoaded(start, end);
  }, [start, end, ensureRangeLoaded, resultsList, resultsVersion]);

  // Track container height changes so virtualization recalculates the viewport
  useLayoutEffect(() => {
    const container = containerRef.current;
    if (!container) return;
    const updateViewport = () => setViewportHeight(container.clientHeight);
    const resizeObserver = new ResizeObserver(updateViewport);
    resizeObserver.observe(container);
    updateViewport();
    return () => resizeObserver.disconnect();
  }, []);

  // Re-clamp scrollTop whenever total height shrinks (e.g. due to a narrower result set)
  useEffect(() => {
    setScrollTop((prev) => {
      const clamped = Math.max(0, Math.min(prev, maxScrollTop));
      return clamped === prev ? prev : clamped;
    });
  }, [maxScrollTop]);

  // ----- imperative API -----
  // Imperative handle used by App.jsx to drive preloading and programmatic scroll
  const scrollToRow = useCallback(
    (rowIndex: number, align: 'nearest' | 'start' | 'end' | 'center' = 'nearest') => {
      if (!Number.isFinite(rowIndex) || rowCount === 0) {
        return;
      }

      const targetIndex = Math.max(0, Math.min(rowIndex, rowCount - 1));
      const rowTop = targetIndex * rowHeight;
      const rowBottom = rowTop + rowHeight;

      updateScrollAndRange((prev) => {
        if (viewportHeight <= 0) {
          return rowTop;
        }

        const viewportTop = prev;
        const viewportBottom = viewportTop + viewportHeight;

        switch (align) {
          case 'start':
            return rowTop;
          case 'end':
            return rowBottom - viewportHeight;
          case 'center':
            return rowTop - Math.max(0, (viewportHeight - rowHeight) / 2);
          case 'nearest':
          default: {
            if (rowTop < viewportTop) {
              return rowTop;
            }
            if (rowBottom > viewportBottom) {
              return rowBottom - viewportHeight;
            }
            return prev;
          }
        }
      });
    },
    [rowCount, rowHeight, viewportHeight, updateScrollAndRange],
  );

  const getItemAt = useCallback((index: number) => cache.get(index), [cache]);

  useImperativeHandle(
    ref,
    () => ({
      scrollToTop: () => updateScrollAndRange(() => 0),
      scrollToRow,
      ensureRangeLoaded,
      getItem: getItemAt,
    }),
    [updateScrollAndRange, scrollToRow, ensureRangeLoaded, getItemAt],
  );

  // ----- rendered items memo -----
  // Memoize rendered rows so virtualization only re-renders what it must
  const renderedItems = useMemo(() => {
    if (end < start) return null;

    const baseTop = start * rowHeight - scrollTop;
    return Array.from({ length: end - start + 1 }, (_, i) => {
      const rowIndex = start + i;
      const item = cache.get(rowIndex);
      return renderRow(rowIndex, item, {
        position: 'absolute',
        top: baseTop + i * rowHeight,
        height: rowHeight,
        left: 0,
        right: 0,
      });
    });
  }, [start, end, scrollTop, rowHeight, cache, renderRow]);

  // ----- render -----
  return (
    <div
      ref={containerRef}
      className={`virtual-list ${className}`}
      onWheel={handleWheel}
      role="list"
      aria-rowcount={rowCount}
    >
      <div className="virtual-list-viewport" onScroll={handleHorizontalScroll}>
        <div className="virtual-list-items">{renderedItems}</div>
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

VirtualList.displayName = 'VirtualList';

export default VirtualList;
