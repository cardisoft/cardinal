import { useEffect, useMemo, useRef, useState } from 'react';
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

const toComparable = (value: string, caseSensitive: boolean): string =>
  caseSensitive ? value : value.toLowerCase();

type RecentEventsOptions = {
  caseSensitive: boolean;
};

export function useRecentFSEvents({ caseSensitive }: RecentEventsOptions) {
  const [recentEvents, setRecentEvents] = useState<RecentEventPayload[]>([]);
  const [eventFilterQuery, setEventFilterQuery] = useState('');
  const isMountedRef = useRef(false);

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

          setRecentEvents((prev) => {
            let updated = [...prev, ...validEvents];
            if (updated.length > MAX_EVENTS) {
              updated = updated.slice(updated.length - MAX_EVENTS);
            }
            return updated;
          });
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
  }, []);

  const filteredEvents = useMemo(() => {
    const query = eventFilterQuery.trim();
    if (!query) {
      return recentEvents;
    }

    const searchQuery = toComparable(query, caseSensitive);
    return recentEvents.filter((event) => {
      const path = event.path || '';
      const name = path.split('/').pop() || '';
      const testPath = toComparable(path, caseSensitive);
      const testName = toComparable(name, caseSensitive);
      // Perform basic substring matching across path and filename.
      return testPath.includes(searchQuery) || testName.includes(searchQuery);
    });
  }, [recentEvents, eventFilterQuery, caseSensitive]);

  return {
    recentEvents,
    filteredEvents,
    eventFilterQuery,
    setEventFilterQuery,
  };
}
