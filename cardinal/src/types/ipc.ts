import type { SlabIndex } from './slab';

export type StatusBarUpdatePayload = {
  scannedFiles: number;
  processedEvents: number;
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
