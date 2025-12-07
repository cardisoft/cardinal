import { useCallback, useState } from 'react';
import type { MouseEvent as ReactMouseEvent } from 'react';
import { CONTAINER_PADDING, MAX_COL_WIDTH, MIN_COL_WIDTH } from '../constants';

const clampWidth = (value: number): number =>
  Math.max(MIN_COL_WIDTH, Math.min(MAX_COL_WIDTH, value));

export type EventColumnKey = 'time' | 'event' | 'name' | 'path';
type EventColumnWidths = Record<EventColumnKey, number>;

export function useEventColumnWidths() {
  // Compute initial column sizes using viewport width so the events table feels balanced.
  const calculateEventColWidths = useCallback((): EventColumnWidths => {
    const totalWidth = window.innerWidth - CONTAINER_PADDING * 2;
    return {
      time: clampWidth(Math.floor(totalWidth * 0.18)),
      event: clampWidth(Math.floor(totalWidth * 0.24)),
      name: clampWidth(Math.floor(totalWidth * 0.24)),
      path: clampWidth(Math.floor(totalWidth * 0.34)),
    };
  }, []);

  const [eventColWidths, setEventColWidths] = useState<EventColumnWidths>(calculateEventColWidths);

  const onEventResizeStart = useCallback(
    (e: ReactMouseEvent<HTMLSpanElement>, key: EventColumnKey) => {
      // Reuse the existing DOM drag pattern from the files table to keep UX consistent.
      e.preventDefault();
      e.stopPropagation();

      const startX = e.clientX;
      const startWidth = eventColWidths[key];

      const handleMouseMove = (moveEvent: MouseEvent) => {
        const delta = moveEvent.clientX - startX;
        const newWidth = clampWidth(startWidth + delta);
        setEventColWidths((prev) => ({ ...prev, [key]: newWidth }));
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
    [eventColWidths],
  );

  const autoFitEventColumns = useCallback(() => {
    // Snap columns back to their original ratios (invoked from the context menu).
    setEventColWidths(calculateEventColWidths());
  }, [calculateEventColWidths]);

  return {
    eventColWidths,
    onEventResizeStart,
    autoFitEventColumns,
  };
}
