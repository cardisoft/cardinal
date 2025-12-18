import type { SlabIndex } from './slab';

export type StatusBarUpdatePayload = {
  scannedFiles: number;
  processedEvents: number;
};

export type IconUpdateWirePayload = {
  slabIndex: number;
  icon?: string;
  width?: number;
  height?: number;
};

export type IconUpdatePayload = {
  slabIndex: SlabIndex;
  icon?: string;
  width?: number;
  height?: number;
};

export type RecentEventPayload = {
  path: string;
  flagBits: number;
  eventId: number;
  timestamp: number;
};

export type AppLifecycleStatus = 'Initializing' | 'Updating' | 'Ready';

export type SearchResponsePayload = {
  results: number[];
  highlights?: string[];
};
