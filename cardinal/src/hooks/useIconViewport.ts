import { useCallback, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { SlabIndex } from '../types/slab';

type UseIconViewportProps = {
  results: SlabIndex[];
  start: number;
  end: number;
  iconSize: number;
};

// Deduplicates and throttles icon viewport updates to the backend.
export function useIconViewport({ results, start, end, iconSize }: UseIconViewportProps) {
  const requestIdRef = useRef(0);
  const lastRangeRef = useRef<{ start: number; end: number } | null>(null);
  const pendingRef = useRef<SlabIndex[] | null>(null);
  const rafRef = useRef<number | null>(null);

  const flushIconViewport = useCallback(() => {
    rafRef.current = null;
    const viewport = pendingRef.current;
    if (!viewport) return;
    pendingRef.current = null;
    requestIdRef.current += 1;
    requestIdRef.current += 1;
    void invoke('update_icon_viewport', { id: requestIdRef.current, viewport, iconSize });
  }, [iconSize]);

  const scheduleIconViewport = useCallback(
    (viewport: SlabIndex[]) => {
      pendingRef.current = viewport;
      if (rafRef.current === null) {
        rafRef.current = requestAnimationFrame(flushIconViewport);
      }
    },
    [flushIconViewport],
  );

  useEffect(() => {
    // Reset tracking when the source list identity changes.
    lastRangeRef.current = null;
  }, [results]);

  useEffect(() => {
    const clampedStart = Math.max(0, start);
    const clampedEnd = Math.min(end, results.length - 1);
    const hasRange = results.length > 0 && end >= start && clampedEnd >= clampedStart;

    // Nothing to send; emit a clear once per empty state.
    if (!hasRange) {
      if (!lastRangeRef.current || lastRangeRef.current.start !== -1) {
        lastRangeRef.current = { start: -1, end: -1 };
        scheduleIconViewport([]);
      }
      return;
    }

    const last = lastRangeRef.current;
    if (last && last.start === clampedStart && last.end === clampedEnd) {
      return; // viewport unchanged; skip IPC
    }

    lastRangeRef.current = { start: clampedStart, end: clampedEnd };
    scheduleIconViewport(results.slice(clampedStart, clampedEnd + 1));
  }, [results, start, end, iconSize, scheduleIconViewport]);

  useEffect(
    () => () => {
      if (rafRef.current !== null) {
        cancelAnimationFrame(rafRef.current);
      }
      pendingRef.current = [];
      flushIconViewport();
    },
    [flushIconViewport],
  );
}
