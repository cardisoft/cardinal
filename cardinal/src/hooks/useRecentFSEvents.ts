import { useEffect, useMemo, useRef, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import type { UnlistenFn } from '@tauri-apps/api/event';
import type { FileSystemEvent } from '../components/FSEventsPanel';

// Listen to batched file-system events and expose filtered projections for the UI.
const MAX_EVENTS = 10000;

// Accept multiple backend shapes and normalise to a consistent camelCase form.
const normalizeEvent = (rawEvent: FileSystemEvent | undefined): FileSystemEvent | undefined => {
  if (!rawEvent || typeof rawEvent !== 'object') {
    return rawEvent;
  }

  if (typeof rawEvent.eventId === 'number') {
    return rawEvent;
  }

  const eventId =
    typeof (rawEvent as { event_id?: number }).event_id === 'number'
      ? (rawEvent as { event_id: number }).event_id
      : typeof (rawEvent as { eventID?: number }).eventID === 'number'
        ? (rawEvent as { eventID: number }).eventID
        : undefined;

  return eventId === undefined ? rawEvent : { ...rawEvent, eventId };
};

const toComparable = (value: string, caseSensitive: boolean): string =>
  caseSensitive ? value : value.toLowerCase();

type RecentEventsOptions = {
  caseSensitive: boolean;
  useRegex: boolean;
};

export function useRecentFSEvents({ caseSensitive, useRegex }: RecentEventsOptions) {
  const [recentEvents, setRecentEvents] = useState<FileSystemEvent[]>([]);
  const [eventFilterQuery, setEventFilterQuery] = useState('');
  const isMountedRef = useRef(false);

  useEffect(() => {
    isMountedRef.current = true;
    let unlistenEvents: UnlistenFn | undefined;

    // Capture streamed events from the Rust side and keep only the latest N entries.
    const setupListener = async () => {
      try {
        unlistenEvents = await listen<FileSystemEvent[]>('fs_events_batch', (event) => {
          if (!isMountedRef.current) return;
          const newEvents = Array.isArray(event?.payload) ? event.payload : [];
          if (newEvents.length === 0) return;

          setRecentEvents((prev) => {
            const normalizedIncoming = newEvents.map(normalizeEvent).filter(Boolean) as FileSystemEvent[];
            let updated = [...prev, ...normalizedIncoming];
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

    if (useRegex) {
      try {
        const flags = caseSensitive ? '' : 'i';
        const regex = new RegExp(query, flags);
        // Regex search hits either the full path or just the leaf name.
        return recentEvents.filter((event) => {
          const path = event.path || '';
          const name = path.split('/').pop() || '';
          return regex.test(path) || regex.test(name);
        });
      } catch {
        return recentEvents;
      }
    }

    const searchQuery = toComparable(query, caseSensitive);
    return recentEvents.filter((event) => {
      const path = event.path || '';
      const name = path.split('/').pop() || '';
      const testPath = toComparable(path, caseSensitive);
      const testName = toComparable(name, caseSensitive);
      // Perform basic substring matching when regex is disabled.
      return testPath.includes(searchQuery) || testName.includes(searchQuery);
    });
  }, [recentEvents, eventFilterQuery, caseSensitive, useRegex]);

  return {
    recentEvents,
    filteredEvents,
    eventFilterQuery,
    setEventFilterQuery,
  };
}
