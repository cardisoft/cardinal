import type { SlabIndex } from './slab';

export type StatusBarUpdatePayload = {
  scannedFiles: number;
  processedEvents: number;
  rescanErrors: number;
};

export type IconUpdateWirePayload = {
  slabIndex: number;
  icon?: string;
};

export type IconUpdatePayload = {
  slabIndex: SlabIndex;
  icon?: string;
};

export type RecentEventPayload = {
  path: string;
  flagBits: number;
  eventId: number;
  timestamp: number;
};

export type AppLifecycleStatus = 'Initializing' | 'Updating' | 'Ready';

export enum SearchStatusCode {
  OK = 0,
  CANCELLED = 1,
  /** The backend is still building its index; retry once the lifecycle leaves Initializing. */
  INDEXING = 2,
  /** A temporary index for the requested folder scope is being built; retry shortly. */
  SCOPED_INDEXING = 3,
  /** The folder scope is not a directory; retrying cannot help. */
  SCOPE_UNAVAILABLE = 4,
}

export type SearchResponsePayload = {
  results: number[];
  highlights?: string[];
  /** Folder scope being indexed when `statusCode` is `SCOPED_INDEXING`. */
  scope?: string | null;
  statusCode: SearchStatusCode;
};
