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

export function useColumnResize() {
  const [colWidths, setColWidths] = useState<ColumnWidths>(() => {
    const windowWidth = window.innerWidth;
    return calculateInitialColWidths(windowWidth);
  });

  // Keep the filename column filling whatever horizontal space remains after the
  // fixed columns and the scrollbar gutter.
  useEffect(() => {
    const handleResize = () => {
      setColWidths((prev) => {
        const fixedWidth = prev.path + prev.size + prev.modified + prev.created;
        const available = window.innerWidth - CONTAINER_PADDING - SCROLLBAR_WIDTH;
        const filename = Math.max(MIN_COL_WIDTH, available - fixedWidth);
        return { ...prev, filename };
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
      const startWidth = colWidths[key];
      startColumnResizeDrag({
        event: e,
        startWidth,
        clampWidth,
        applyWidth: (newWidth) => {
          setColWidths((prev) => ({ ...prev, [key]: newWidth }));
        },
      });
    },
    [clampWidth, colWidths],
  );

  const autoFitColumns = useCallback(() => {
    const windowWidth = window.innerWidth;
    const newColWidths = calculateInitialColWidths(windowWidth);
    setColWidths(newColWidths);
  }, []);

  return {
    colWidths,
    onResizeStart,
    autoFitColumns,
  };
}
