import { useState, useCallback } from 'react';
import type { MouseEvent as ReactMouseEvent } from 'react';
import { calculateInitialColWidths, MAX_COL_WIDTH, MIN_COL_WIDTH } from '../constants';
import type { ColumnKey } from '../constants';
import { startColumnResizeDrag } from './resizeDrag';

type ColumnWidths = Record<ColumnKey, number>;

export function useColumnResize() {
  const [colWidths, setColWidths] = useState<ColumnWidths>(() => {
    const windowWidth = window.innerWidth;
    return calculateInitialColWidths(windowWidth);
  });

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
