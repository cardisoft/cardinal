import { useEffect, useMemo, useReducer, useRef, useState } from 'react';
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

type RecentEventRecord = {
  payload: RecentEventPayload;
  path: string;
  name: string;
  lowerPath: string;
  lowerName: string;
};

type RecentEventsOptions = {
  caseSensitive: boolean;
  isActive: boolean;
};

export function useRecentFSEvents({ caseSensitive, isActive }: RecentEventsOptions) {
  const eventsRef = useRef<RecentEventRecord[]>([]);
  const [eventFilterQuery, setEventFilterQuery] = useState('');
  const isMountedRef = useRef(false);
  const isActiveRef = useRef(isActive);
  const [bufferVersion, bumpBufferVersion] = useReducer((count: number) => count + 1, 0);

  useEffect(() => {
    isActiveRef.current = isActive;
    if (isActive) {
      bumpBufferVersion();
    }
  }, [isActive, bumpBufferVersion]);

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
          const normalizedBatch = validEvents.map(toEventRecord);
          let updated = [...previous, ...normalizedBatch];
          if (updated.length > MAX_EVENTS) {
            updated = updated.slice(updated.length - MAX_EVENTS);
          }
          eventsRef.current = updated;

          if (isActiveRef.current) {
            bumpBufferVersion();
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
  }, [bumpBufferVersion]);

  const filteredEvents = useMemo(
    () => filterBuffer(eventsRef.current, eventFilterQuery, caseSensitive),
    [eventFilterQuery, caseSensitive, bufferVersion],
  );

  return {
    filteredEvents,
    eventFilterQuery,
    setEventFilterQuery,
  };
}

const normalize = (value: string, caseSensitive: boolean): string =>
  caseSensitive ? value : value.toLowerCase();

const filterBuffer = (
  events: RecentEventRecord[],
  query: string,
  caseSensitive: boolean,
): RecentEventPayload[] => {
  const trimmed = query.trim();
  if (!trimmed) {
    return events.map((record) => record.payload);
  }
  const comparable = normalize(trimmed, caseSensitive);
  return events
    .filter((record) => {
      const haystackPath = caseSensitive ? record.path : record.lowerPath;
      const haystackName = caseSensitive ? record.name : record.lowerName;
      return haystackPath.includes(comparable) || haystackName.includes(comparable);
    })
    .map((record) => record.payload);
};

const toEventRecord = (payload: RecentEventPayload): RecentEventRecord => {
  const path = payload.path || '';
  const name = path.split('/').pop() || '';
  return {
    payload,
    path,
    name,
    lowerPath: path.toLowerCase(),
    lowerName: name.toLowerCase(),
  };
};
