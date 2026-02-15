import { useCallback, useState } from 'react';
import type { MouseEvent as ReactMouseEvent } from 'react';
import { CONTAINER_PADDING, MAX_COL_WIDTH, MIN_COL_WIDTH } from '../constants';
import { startColumnResizeDrag } from './resizeDrag';

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
      const startWidth = eventColWidths[key];
      startColumnResizeDrag({
        event: e,
        startWidth,
        clampWidth,
        applyWidth: (newWidth) => {
          setEventColWidths((prev) => ({ ...prev, [key]: newWidth }));
        },
      });
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
