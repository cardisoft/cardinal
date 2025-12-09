import { useState, useCallback } from 'react';
import type { MouseEvent as ReactMouseEvent } from 'react';
import { calculateInitialColWidths, MAX_COL_WIDTH, MIN_COL_WIDTH } from '../constants';
import type { ColumnKey } from '../constants';

type ColumnWidths = Record<ColumnKey, number>;

export function useColumnResize() {
  const [colWidths, setColWidths] = useState<ColumnWidths>(() => {
    const windowWidth = window.innerWidth;
    return calculateInitialColWidths(windowWidth);
  });

  const onResizeStart = useCallback(
    (key: ColumnKey) => (e: ReactMouseEvent<HTMLSpanElement>) => {
      e.preventDefault();
      e.stopPropagation();

      const startX = e.clientX;
      const startWidth = colWidths[key];

      const handleMouseMove = (moveEvent: MouseEvent) => {
        moveEvent.preventDefault();
        document.body.style.cursor = 'col-resize';
        const delta = moveEvent.clientX - startX;
        const newWidth = Math.max(MIN_COL_WIDTH, Math.min(MAX_COL_WIDTH, startWidth + delta));
        setColWidths((prev) => ({ ...prev, [key]: newWidth }));
      };

      const handleMouseUp = () => {
        document.removeEventListener('mousemove', handleMouseMove);
        document.removeEventListener('mouseup', handleMouseUp);
        document.body.style.userSelect = '';
        document.body.style.cursor = '';
      };

      document.addEventListener('mousemove', handleMouseMove);
      document.addEventListener('mouseup', handleMouseUp);
      document.body.style.userSelect = 'none';
      document.body.style.cursor = 'col-resize';
    },
    [colWidths],
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
