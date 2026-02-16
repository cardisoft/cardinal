import { listen } from '@tauri-apps/api/event';
import type { Event as TauriEvent, UnlistenFn } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import type { DragDropEvent } from '@tauri-apps/api/window';
import type {
  AppLifecycleStatus,
  IconUpdatePayload,
  IconUpdateWirePayload,
  RecentEventPayload,
  StatusBarUpdatePayload,
} from '../types/ipc';
import type { SlabIndex } from '../types/slab';

export type QuickLookKeydownPayload = {
  keyCode: number;
  characters?: string | null;
  modifiers: {
    shift: boolean;
    control: boolean;
    option: boolean;
    command: boolean;
  };
};

type Listener<T> = (payload: T) => void;
export type WindowDragDropEvent = TauriEvent<DragDropEvent>;

const statusBarUpdateListeners = new Set<Listener<StatusBarUpdatePayload>>();
const lifecycleStateListeners = new Set<Listener<AppLifecycleStatus>>();
const quickLaunchListeners = new Set<Listener<void>>();
const fsEventsBatchListeners = new Set<Listener<RecentEventPayload[]>>();
const iconUpdateListeners = new Set<Listener<readonly IconUpdatePayload[]>>();
const quickLookKeydownListeners = new Set<Listener<QuickLookKeydownPayload>>();
const windowDragDropListeners = new Set<Listener<WindowDragDropEvent>>();

let initPromise: Promise<void> | null = null;

const emit = <T>(listeners: Set<Listener<T>>, payload: T): void => {
  listeners.forEach((listener) => {
    listener(payload);
  });
};

const subscribe = <T>(listeners: Set<Listener<T>>, listener: Listener<T>): UnlistenFn => {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
};

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

const normalizeRecentEvents = (payload: unknown): RecentEventPayload[] => {
  if (!Array.isArray(payload)) {
    return [];
  }
  return payload.filter(isRecentEventPayload);
};

const isIconUpdateWirePayload = (value: unknown): value is IconUpdateWirePayload => {
  if (!value || typeof value !== 'object') {
    return false;
  }

  const candidate = value as Record<string, unknown>;
  return (
    typeof candidate.slabIndex === 'number' &&
    (typeof candidate.icon === 'string' || typeof candidate.icon === 'undefined')
  );
};

const normalizeIconUpdates = (payload: unknown): IconUpdatePayload[] => {
  if (!Array.isArray(payload)) {
    return [];
  }
  return payload
    .filter(isIconUpdateWirePayload)
    .map((item) => ({ slabIndex: item.slabIndex as SlabIndex, icon: item.icon }));
};

export const initializeTauriEventRuntime = (): Promise<void> => {
  if (initPromise) {
    return initPromise;
  }

  initPromise = (async () => {
    const setupTasks: Promise<unknown>[] = [
      listen<StatusBarUpdatePayload>('status_bar_update', (event) => {
        const payload = event.payload;
        if (!payload) return;
        emit(statusBarUpdateListeners, payload);
      }).catch((error) => {
        console.error('Failed to register status_bar_update listener', error);
      }),
      listen<AppLifecycleStatus>('app_lifecycle_state', (event) => {
        const status = event.payload;
        if (!status) return;
        emit(lifecycleStateListeners, status);
      }).catch((error) => {
        console.error('Failed to register app_lifecycle_state listener', error);
      }),
      listen('quick_launch', () => {
        emit(quickLaunchListeners, undefined);
      }).catch((error) => {
        console.error('Failed to register quick_launch listener', error);
      }),
      listen<RecentEventPayload[]>('fs_events_batch', (event) => {
        const payload = normalizeRecentEvents(event.payload);
        if (payload.length === 0) return;
        emit(fsEventsBatchListeners, payload);
      }).catch((error) => {
        console.error('Failed to register fs_events_batch listener', error);
      }),
      listen<readonly IconUpdateWirePayload[] | null | undefined>('icon_update', (event) => {
        const payload = normalizeIconUpdates(event.payload);
        if (payload.length === 0) return;
        emit(iconUpdateListeners, payload);
      }).catch((error) => {
        console.error('Failed to register icon_update listener', error);
      }),
      listen<QuickLookKeydownPayload>('quicklook-keydown', (event) => {
        const payload = event.payload;
        if (!payload) return;
        emit(quickLookKeydownListeners, payload);
      }).catch((error) => {
        console.error('Failed to register quicklook-keydown listener', error);
      }),
      getCurrentWindow()
        .onDragDropEvent((event) => {
          emit(windowDragDropListeners, event);
        })
        .catch((error) => {
          console.error('Failed to register drag-drop listener', error);
        }),
    ];

    await Promise.allSettled(setupTasks);
  })();

  return initPromise;
};

export const subscribeStatusBarUpdate = (
  listener: Listener<StatusBarUpdatePayload>,
): UnlistenFn => {
  void initializeTauriEventRuntime();
  return subscribe(statusBarUpdateListeners, listener);
};

export const subscribeLifecycleState = (listener: Listener<AppLifecycleStatus>): UnlistenFn => {
  void initializeTauriEventRuntime();
  return subscribe(lifecycleStateListeners, listener);
};

export const subscribeQuickLaunch = (listener: () => void): UnlistenFn => {
  void initializeTauriEventRuntime();
  return subscribe(quickLaunchListeners, listener);
};

export const subscribeFSEventsBatch = (listener: Listener<RecentEventPayload[]>): UnlistenFn => {
  void initializeTauriEventRuntime();
  return subscribe(fsEventsBatchListeners, listener);
};

export const subscribeIconUpdate = (
  listener: Listener<readonly IconUpdatePayload[]>,
): UnlistenFn => {
  void initializeTauriEventRuntime();
  return subscribe(iconUpdateListeners, listener);
};

export const subscribeQuickLookKeydown = (
  listener: Listener<QuickLookKeydownPayload>,
): UnlistenFn => {
  void initializeTauriEventRuntime();
  return subscribe(quickLookKeydownListeners, listener);
};

export const subscribeWindowDragDrop = (listener: Listener<WindowDragDropEvent>): UnlistenFn => {
  void initializeTauriEventRuntime();
  return subscribe(windowDragDropListeners, listener);
};
