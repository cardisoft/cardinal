import { useState, useCallback, useEffect } from 'react';
import type { MouseEvent as ReactMouseEvent } from 'react';
import {
  calculateInitialColWidths,
  MAX_COL_WIDTH,
  MIN_COL_WIDTH,
  CONTAINER_PADDING,
  SCROLLBAR_WIDTH,
} from '../constants';
import type { ColumnKey } from '../constants';
import { startColumnResizeDrag } from './resizeDrag';

type ColumnWidths = Record<ColumnKey, number>;

// Column layout has two intentionally different modes:
//
// - autoFilename:
//   Finder-style layout. The non-filename columns keep their current widths, and
//   filename is derived from the remaining viewport width:
//     filename = max(MIN_COL_WIDTH, availableWidth - fixedColumnsWidth)
//   This keeps the full table aligned to the window while the filename column
//   absorbs horizontal window resizes. If the fixed columns alone cannot fit,
//   filename stops at MIN_COL_WIDTH and horizontal overflow is expected.
//
// - manual:
//   User-controlled layout. Dragging any column switches to this mode, and every
//   column width becomes independent. The total column width may be smaller than,
//   equal to, or larger than the viewport. Window resizes do not change the
//   user's chosen widths unless the window is resized back near the current
//   total column width, which is treated as an intentional return to the
//   Finder-style fitted layout.
type ColumnLayoutMode = 'autoFilename' | 'manual';

type ColumnResizeState = {
  mode: ColumnLayoutMode;
  widths: ColumnWidths;
};

// Window resize events and CSS/grid math do not always land on exact integer
// equality, so use a small snap band when deciding that a manual layout has been
// resized back to "fits the viewport" and can return to autoFilename mode.
const AUTO_SNAP_THRESHOLD = 5;

const getAvailableWidth = (windowWidth: number) =>
  windowWidth - CONTAINER_PADDING - SCROLLBAR_WIDTH;

const getFixedWidth = (widths: ColumnWidths) =>
  widths.path + widths.size + widths.modified + widths.created;

const getTotalWidth = (widths: ColumnWidths) => getFixedWidth(widths) + widths.filename;

const withAutoFilenameWidth = (widths: ColumnWidths, available: number): ColumnWidths => {
  const filename = Math.max(MIN_COL_WIDTH, available - getFixedWidth(widths));
  return widths.filename === filename ? widths : { ...widths, filename };
};

const calculateAutoColumnWidths = (windowWidth: number): ColumnWidths => {
  const widths = calculateInitialColWidths(windowWidth);
  return withAutoFilenameWidth(widths, getAvailableWidth(windowWidth));
};

export function useColumnResize() {
  const [state, setState] = useState<ColumnResizeState>(() => ({
    mode: 'autoFilename',
    widths: calculateAutoColumnWidths(window.innerWidth),
  }));

  // In autoFilename mode, every window resize re-derives filename from the
  // current fixed columns. In manual mode, preserve the user's independent
  // column widths until the viewport width is within AUTO_SNAP_THRESHOLD of the
  // current column total, then switch back to autoFilename.
  useEffect(() => {
    const handleResize = () => {
      setState((prev) => {
        const available = getAvailableWidth(window.innerWidth);

        if (
          prev.mode === 'manual' &&
          Math.abs(available - getTotalWidth(prev.widths)) > AUTO_SNAP_THRESHOLD
        ) {
          return prev;
        }

        const widths = withAutoFilenameWidth(prev.widths, available);
        if (prev.mode === 'autoFilename' && widths === prev.widths) {
          return prev;
        }

        return { mode: 'autoFilename', widths };
      });
    };
    window.addEventListener('resize', handleResize);
    return () => window.removeEventListener('resize', handleResize);
  }, []);

  const clampWidth = useCallback(
    (value: number) => Math.max(MIN_COL_WIDTH, Math.min(MAX_COL_WIDTH, value)),
    [],
  );

  const onResizeStart = useCallback(
    (key: ColumnKey) => (e: ReactMouseEvent<HTMLSpanElement>) => {
      const startWidth = state.widths[key];
      startColumnResizeDrag({
        event: e,
        startWidth,
        clampWidth,
        applyWidth: (newWidth) => {
          setState((prev) => {
            const widths =
              prev.widths[key] === newWidth ? prev.widths : { ...prev.widths, [key]: newWidth };
            if (prev.mode === 'manual' && widths === prev.widths) {
              return prev;
            }

            return { mode: 'manual', widths };
          });
        },
      });
    },
    [clampWidth, state.widths],
  );

  const autoFitColumns = useCallback(() => {
    setState({
      mode: 'autoFilename',
      widths: calculateAutoColumnWidths(window.innerWidth),
    });
  }, []);

  return {
    colWidths: state.widths,
    onResizeStart,
    autoFitColumns,
  };
}
