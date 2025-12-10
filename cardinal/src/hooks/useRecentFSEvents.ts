import { useEffect, useReducer, useRef, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import type { UnlistenFn } from '@tauri-apps/api/event';
import type { RecentEventPayload } from '../types/ipc';

// Listen to batched file-system events and expose filtered projections for the UI.
const MAX_EVENTS = 10000;

const isRecentEventPayload = (value: unknown): value is RecentEventPayload => {
  if (!value || typeof value !== 'object') {
    return false;
  }

  const candidate = value as Record<string, unknown>;
  return (
    typeof candidate.path === 'string' &&
    typeof candidate.eventId === 'number' &&
    typeof candidate.timestamp === 'number' &&
    typeof candidate.flagBits === 'number'
  );
};

type RecentEventsOptions = {
  caseSensitive: boolean;
  isActive: boolean;
};

export function useRecentFSEvents({ caseSensitive, isActive }: RecentEventsOptions) {
  const eventsRef = useRef<RecentEventPayload[]>([]);
  const [eventFilterQuery, setEventFilterQuery] = useState('');
  const [, triggerRender] = useReducer((count) => count + 1, 0);
  const isMountedRef = useRef(false);
  const isActiveRef = useRef(isActive);

  useEffect(() => {
    isActiveRef.current = isActive;
    if (isActive) {
      triggerRender();
    }
  }, [isActive]);

  useEffect(() => {
    isMountedRef.current = true;
    let unlistenEvents: UnlistenFn | undefined;

    // Capture streamed events from the Rust side and keep only the latest N entries.
    const setupListener = async () => {
      try {
        unlistenEvents = await listen<RecentEventPayload[]>('fs_events_batch', (event) => {
          if (!isMountedRef.current) return;
          const payload = event?.payload;
          if (!Array.isArray(payload) || payload.length === 0) return;

          const validEvents = payload.filter(isRecentEventPayload);
          if (validEvents.length === 0) {
            return;
          }

          const previous = eventsRef.current;
          let updated = [...previous, ...validEvents];
          if (updated.length > MAX_EVENTS) {
            updated = updated.slice(updated.length - MAX_EVENTS);
          }
          eventsRef.current = updated;

          if (isActiveRef.current) {
            triggerRender();
          }
        });
      } catch (error) {
        console.error('Failed to listen for file events', error);
      }
    };

    void setupListener();

    return () => {
      isMountedRef.current = false;
      unlistenEvents?.();
    };
  }, [triggerRender]);

  const filteredEvents = filterBuffer(eventsRef.current, eventFilterQuery, caseSensitive);

  return {
    filteredEvents,
    eventFilterQuery,
    setEventFilterQuery,
  };
}

const normalize = (value: string, caseSensitive: boolean): string =>
  caseSensitive ? value : value.toLowerCase();

const filterBuffer = (
  events: RecentEventPayload[],
  query: string,
  caseSensitive: boolean,
): RecentEventPayload[] => {
  const trimmed = query.trim();
  if (!trimmed) {
    return events;
  }
  const comparable = normalize(trimmed, caseSensitive);
  return events.filter((event) => {
    const path = event.path || '';
    const name = path.split('/').pop() || '';
    const normalizedPath = normalize(path, caseSensitive);
    const normalizedName = normalize(name, caseSensitive);
    return normalizedPath.includes(comparable) || normalizedName.includes(comparable);
  });
};
