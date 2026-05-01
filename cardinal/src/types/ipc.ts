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
}

export type SearchResponsePayload = {
  results: number[];
  highlights?: string[];
  statusCode: SearchStatusCode;
};
