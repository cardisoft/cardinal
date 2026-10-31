import React, {
  forwardRef,
  useCallback,
  useEffect,
  useImperativeHandle,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from 'react';
import type { CSSProperties } from 'react';
import Scrollbar from './Scrollbar';
import { useDataLoader, GRID_ICON_SIZE } from '../hooks/useDataLoader';
import { useIconViewport } from '../hooks/useIconViewport';
import type { SearchResultItem } from '../types/search';
import type { SlabIndex } from '../types/slab';

export type IconGridHandle = {
  scrollToTop: () => void;
  scrollToRow: (rowIndex: number, align?: 'nearest' | 'start' | 'end' | 'center') => void;
  ensureRangeLoaded: (startIndex: number, endIndex: number) => Promise<void> | void;
  getItem: (index: number) => SearchResultItem | undefined;
};

export type IconSizePreset = 'small' | 'medium' | 'large' | 'xlarge';

export const ICON_SIZE_PRESETS: Record<IconSizePreset, number> = {
  small: 72,
  medium: 96,
  large: 128,
  xlarge: 192,
};

const CELL_GAP = 8;
/** Max label lines in the tiled view; matches `-webkit-line-clamp` in App.css. */
const LABEL_MAX_LINES = 3;
/** Label font size as a multiple of the base UI font size; matches `.icon-grid-item__label`. */
const LABEL_FONT_SCALE = 0.78;
/** Label line height (unitless); matches `.icon-grid-item__label`. */
const LABEL_LINE_HEIGHT = 1.2;
/**
 * Widest a label line may get, in CJK glyph widths. A full-width CJK glyph is one `em`, so
 * six glyphs are the budget; Latin letters are narrower and fit more of themselves in it.
 */
const LABEL_MAX_CJK_CHARS = 6;

/**
 * Vertical space the name label may occupy. Derived from the base font size so the tile grid
 * tracks the global font-size setting; keep in sync with the label rules in App.css.
 */
export const gridLabelAreaHeight = (fontSize: number): number =>
  Math.round(fontSize * LABEL_FONT_SCALE * LABEL_LINE_HEIGHT * LABEL_MAX_LINES);

/**
 * Pixel width the name label may occupy: six CJK characters at the label font size. Computed
 * here rather than in CSS so the value follows the global font-size setting exactly.
 */
export const gridLabelWidth = (fontSize: number): number =>
  Math.round(fontSize * LABEL_FONT_SCALE * LABEL_MAX_CJK_CHARS);

const DEFAULT_FONT_SIZE = 13;

type IconGridProps = {
  results: SlabIndex[];
  dataResultsVersion: number;
  displayedResultsVersion: number;
  iconSize: IconSizePreset;
  /** Base UI font size in px, used to reserve room for up to three label lines. */
  fontSize?: number;
  renderItem: (
    rowIndex: number,
    item: SearchResultItem | undefined,
    style: CSSProperties,
  ) => React.ReactNode;
  /** Called once per approach to the end of the grid (used to page the launch list). */
  onNearEnd?: () => void;
};

const clamp = (value: number, min: number, max: number): number =>
  Math.max(min, Math.min(value, max));

// Grid virtualization for the "tiled" results view (Windows-Explorer style).
// Mirrors VirtualList's lazy hydration + custom scrollbar approach but lays
// items out in a fixed number of columns derived from the container width.
export const IconGrid = forwardRef<IconGridHandle, IconGridProps>(function IconGrid(
  {
    results,
    dataResultsVersion,
    displayedResultsVersion,
    iconSize,
    fontSize = DEFAULT_FONT_SIZE,
    renderItem,
    onNearEnd,
  },
  ref,
) {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const viewportRef = useRef<HTMLDivElement | null>(null);
  const [scrollTop, setScrollTop] = useState(0);
  const [viewportWidth, setViewportWidth] = useState(0);
  const [viewportHeight, setViewportHeight] = useState(0);

  // Ask the backend for a 2x raster of the current preset: the artwork is drawn at
  // ICON_SIZE_PRESETS[iconSize] CSS px, so a same-size bitmap would look soft on Retina.
  const gridIconRasterSize = Math.min(GRID_ICON_SIZE, Math.round(ICON_SIZE_PRESETS[iconSize] * 2));

  const { cache, ensureRangeLoaded } = useDataLoader(results, dataResultsVersion, {
    iconSize: gridIconRasterSize,
  });

  const iconPixels = ICON_SIZE_PRESETS[iconSize];
  const labelAreaHeight = gridLabelAreaHeight(fontSize);
  const labelWidth = gridLabelWidth(fontSize);
  const cellWidth = iconPixels + CELL_GAP * 2;
  // The cell always reserves room for three label lines, so rows stay evenly spaced and the
  // last text line of every tile lands on the same baseline. The icon keeps its preset size;
  // the text grows downward into the reserved area instead of pushing the icon around.
  const cellHeight = iconPixels + labelAreaHeight + CELL_GAP * 2;

  const columnCount = useMemo(
    () => (viewportWidth > 0 ? Math.max(1, Math.floor(viewportWidth / cellWidth)) : 1),
    [viewportWidth, cellWidth],
  );
  const rowCount = Math.ceil(results.length / columnCount);
  const totalHeight = rowCount * cellHeight;
  const maxScrollTop = Math.max(0, totalHeight - viewportHeight);

  const firstVisibleRow = viewportHeight > 0 ? Math.floor(scrollTop / cellHeight) : 0;
  const visibleRowCount = viewportHeight > 0 ? Math.ceil(viewportHeight / cellHeight) + 1 : 0;
  const startIndex = Math.max(0, firstVisibleRow * columnCount - columnCount);
  const endIndex = Math.min(
    results.length - 1,
    (firstVisibleRow + visibleRowCount + 1) * columnCount - 1,
  );

  // Ordering/projection changes should still refresh icon hydration.
  useIconViewport({
    results,
    resultsVersion: displayedResultsVersion,
    start: results.length > 0 ? Math.max(0, startIndex) : 0,
    end: results.length > 0 ? Math.max(0, endIndex) : -1,
    iconSize: gridIconRasterSize,
  });

  const updateScroll = useCallback(
    (updater: (value: number) => number) => {
      setScrollTop((prev) => clamp(updater(prev), 0, maxScrollTop));
    },
    [maxScrollTop],
  );

  const handleWheel = useCallback(
    (event: React.WheelEvent<HTMLDivElement>) => {
      event.preventDefault();
      const { deltaMode, deltaY } = event;
      let delta = deltaY;
      if (deltaMode === 1) {
        delta = deltaY * cellHeight;
      } else if (deltaMode === 2) {
        delta = deltaY * (viewportHeight || cellHeight * 4);
      }
      updateScroll((prev) => prev + delta);
    },
    [cellHeight, viewportHeight, updateScroll],
  );

  // Request the next page when the visible rows approach the end of the grid.
  useEffect(() => {
    if (!onNearEnd || totalHeight <= 0 || viewportHeight <= 0 || rowCount === 0) {
      return;
    }
    if (scrollTop + viewportHeight >= totalHeight - cellHeight * 2) {
      onNearEnd();
    }
  }, [cellHeight, onNearEnd, rowCount, scrollTop, totalHeight, viewportHeight]);

  // Keep data warm for the active window.
  useEffect(() => {
    if (results.length === 0) {
      return;
    }
    ensureRangeLoaded(Math.max(0, startIndex), Math.max(0, endIndex));
  }, [startIndex, endIndex, ensureRangeLoaded, results.length, displayedResultsVersion, cache]);

  useLayoutEffect(() => {
    const container = containerRef.current;
    const viewport = viewportRef.current;
    if (!container || !viewport) return;
    const updateViewport = () => {
      // Measure the scrolling element, not the outer wrapper: its `clientHeight` already
      // excludes the horizontal scrollbar, which otherwise covers the last row.
      setViewportWidth(viewport.clientWidth);
      setViewportHeight(viewport.clientHeight);
    };
    const observer = new ResizeObserver(updateViewport);
    observer.observe(container);
    observer.observe(viewport);
    updateViewport();
    return () => observer.disconnect();
  }, []);

  useEffect(() => {
    setScrollTop((prev) => clamp(prev, 0, maxScrollTop));
  }, [maxScrollTop]);

  const getItem = useCallback(
    (index: number) => {
      const slabIndex = results[index];
      return slabIndex === undefined ? undefined : cache.get(slabIndex);
    },
    [cache, results],
  );

  const scrollToRow = useCallback(
    (index: number, align: 'nearest' | 'start' | 'end' | 'center' = 'nearest') => {
      if (!Number.isFinite(index) || results.length === 0 || viewportHeight <= 0) {
        return;
      }

      // Translate a result index into a grid row, then align that row like VirtualList does.
      const clampedIndex = clamp(index, 0, Math.max(0, results.length - 1));
      const row = Math.floor(clampedIndex / columnCount);
      const rowTop = row * cellHeight;
      const rowBottom = rowTop + cellHeight;

      updateScroll((prev) => {
        const viewportTop = prev;
        const viewportBottom = viewportTop + viewportHeight;

        switch (align) {
          case 'start':
            return rowTop;
          case 'end':
            return rowBottom - viewportHeight;
          case 'center':
            return rowTop - Math.max(0, (viewportHeight - cellHeight) / 2);
          case 'nearest':
          default: {
            // Keep the current scroll offset when the row is already fully visible; this is
            // what prevents selecting an item (e.g. via the context menu) from jumping.
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
    [columnCount, cellHeight, results.length, updateScroll, viewportHeight],
  );

  useImperativeHandle(
    ref,
    () => ({
      scrollToTop: () => updateScroll(() => 0),
      scrollToRow,
      ensureRangeLoaded,
      getItem,
    }),
    [updateScroll, scrollToRow, ensureRangeLoaded, getItem],
  );

  const renderedItems = useMemo(() => {
    if (results.length === 0) {
      return [];
    }
    const items: React.ReactNode[] = [];
    const first = Math.max(0, startIndex);
    const last = Math.max(first, endIndex);
    for (let i = first; i <= last && i < results.length; i += 1) {
      const row = Math.floor(i / columnCount);
      const column = i % columnCount;
      const item = getItem(i);
      items.push(
        renderItem(i, item, {
          position: 'absolute',
          // Convert the absolute row offset into viewport space; without subtracting
          // `scrollTop` rows pushed past the clipped viewport disappear.
          top: row * cellHeight + CELL_GAP - scrollTop,
          left: column * cellWidth + CELL_GAP,
          width: iconPixels,
          height: iconPixels + labelAreaHeight,
          // `.icon-grid-item__icon` caps itself at this size and shrinks when the label wraps.
          '--icon-size': `${iconPixels}px`,
          '--label-width': `${labelWidth}px`,
          // Fixed height so a three-line name never takes space from the icon.
          '--label-height': `${labelAreaHeight}px`,
        } as CSSProperties),
      );
    }
    return items;
  }, [
    results.length,
    startIndex,
    endIndex,
    columnCount,
    cellHeight,
    cellWidth,
    iconPixels,
    labelAreaHeight,
    labelWidth,
    scrollTop,
    getItem,
    renderItem,
  ]);

  return (
    <div ref={containerRef} className="virtual-list icon-grid" onWheel={handleWheel} role="list">
      <div ref={viewportRef} className="virtual-list-viewport">
        <div className="virtual-list-items">{renderedItems}</div>
      </div>
      <Scrollbar
        totalHeight={totalHeight}
        viewportHeight={viewportHeight}
        maxScrollTop={maxScrollTop}
        scrollTop={scrollTop}
        onScrollUpdate={updateScroll}
      />
    </div>
  );
});

IconGrid.displayName = 'IconGrid';

export default IconGrid;

export type { IconGridProps };
