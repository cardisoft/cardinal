import { useEffect, useMemo, useReducer, useRef, useState } from 'react';
import { subscribeFSEventsBatch } from '../runtime/tauriEventRuntime';
import type { RecentEventPayload } from '../types/ipc';

// Listen to batched file-system events and expose filtered projections for the UI.
const MAX_EVENTS = 10000;

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
  const isActiveRef = useRef(isActive);
  const [bufferVersion, bumpBufferVersion] = useReducer((count: number) => count + 1, 0);

  useEffect(() => {
    isActiveRef.current = isActive;
    if (isActive) {
      bumpBufferVersion();
    }
  }, [isActive]);

  useEffect(() => {
    const unlistenEvents = subscribeFSEventsBatch((payload: RecentEventPayload[]) => {
      if (payload.length === 0) return;

      const previous = eventsRef.current;
      const normalizedBatch = payload.map(toEventRecord);
      let updated = [...previous, ...normalizedBatch];
      if (updated.length > MAX_EVENTS) {
        updated = updated.slice(updated.length - MAX_EVENTS);
      }
      eventsRef.current = updated;

      if (isActiveRef.current) {
        bumpBufferVersion();
      }
    });
    return unlistenEvents;
  }, []);

  const filteredEvents = useMemo(
    () => filterBuffer(eventsRef.current, eventFilterQuery, caseSensitive),
    // bufferVersion acts as the signal that the mutable eventsRef contents changed.
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
