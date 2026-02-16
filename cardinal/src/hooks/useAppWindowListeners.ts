import { useEffect, useRef, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import type { Event as TauriEvent, UnlistenFn } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import type { DragDropEvent } from '@tauri-apps/api/window';
import type { StatusTabKey } from '../components/StatusBar';
import type { AppLifecycleStatus, StatusBarUpdatePayload } from '../types/ipc';
import { useStableEvent } from './useStableEvent';

type QueueSearchOptions = {
  immediate?: boolean;
  onSearchCommitted?: (query: string) => void;
};

type UseAppWindowListenersOptions = {
  activeTab: StatusTabKey;
  focusSearchInput: () => void;
  handleStatusUpdate: (scannedFiles: number, processedEvents: number, rescanErrors: number) => void;
  setLifecycleState: (status: AppLifecycleStatus) => void;
  queueSearch: (query: string, options?: QueueSearchOptions) => void;
  setEventFilterQuery: (value: string) => void;
  updateHistoryFromInput: (query: string) => void;
};

type UseAppWindowListenersResult = {
  isWindowFocused: boolean;
};

/**
 * Manages window-level listeners for Tauri IPC events and browser window events.
 * Keeps the DOM focus attribute in sync and routes drag-drop queries by active tab.
 */
export function useAppWindowListeners({
  activeTab,
  focusSearchInput,
  handleStatusUpdate,
  setLifecycleState,
  queueSearch,
  setEventFilterQuery,
  updateHistoryFromInput,
}: UseAppWindowListenersOptions): UseAppWindowListenersResult {
  const [isWindowFocused, setIsWindowFocused] = useState<boolean>(() => {
    if (typeof document === 'undefined') {
      return true;
    }
    return document.hasFocus();
  });
  const isMountedRef = useRef(false);

  useEffect(() => {
    isMountedRef.current = true;
    let unlistenStatus: UnlistenFn | undefined;
    let unlistenLifecycle: UnlistenFn | undefined;
    let unlistenQuickLaunch: UnlistenFn | undefined;

    const setupListeners = async (): Promise<void> => {
      unlistenStatus = await listen<StatusBarUpdatePayload>('status_bar_update', (event) => {
        if (!isMountedRef.current) return;
        const payload = event.payload;
        if (!payload) return;
        const { scannedFiles, processedEvents, rescanErrors } = payload;
        handleStatusUpdate(scannedFiles, processedEvents, rescanErrors);
      });

      unlistenLifecycle = await listen<AppLifecycleStatus>('app_lifecycle_state', (event) => {
        if (!isMountedRef.current) return;
        const status = event.payload;
        if (!status) return;
        setLifecycleState(status);
      });

      unlistenQuickLaunch = await listen('quick_launch', () => {
        if (!isMountedRef.current) return;
        focusSearchInput();
      });
    };

    void setupListeners();

    return () => {
      isMountedRef.current = false;
      unlistenStatus?.();
      unlistenLifecycle?.();
      unlistenQuickLaunch?.();
    };
  }, [focusSearchInput, handleStatusUpdate, setLifecycleState]);

  useEffect(() => {
    if (typeof window === 'undefined') {
      return;
    }
    const handleWindowFocus = () => {
      setIsWindowFocused(true);
    };
    const handleWindowBlur = () => setIsWindowFocused(false);
    window.addEventListener('focus', handleWindowFocus);
    window.addEventListener('blur', handleWindowBlur);
    return () => {
      window.removeEventListener('focus', handleWindowFocus);
      window.removeEventListener('blur', handleWindowBlur);
    };
  }, []);

  useEffect(() => {
    if (typeof document === 'undefined') {
      return;
    }
    document.documentElement.dataset.windowFocused = isWindowFocused ? 'true' : 'false';
  }, [isWindowFocused]);

  const handleWindowDragDrop = useStableEvent((event: TauriEvent<DragDropEvent>) => {
    const payload = event.payload;
    if (payload.type !== 'drop') {
      return;
    }
    const nextValue = payload.paths[0]?.trim();
    if (!nextValue) {
      return;
    }
    const query = `"${nextValue}"`;
    if (activeTab === 'events') {
      setEventFilterQuery(query);
      return;
    }
    queueSearch(query, {
      immediate: true,
      onSearchCommitted: updateHistoryFromInput,
    });
  });

  useEffect(() => {
    let unlisten: UnlistenFn | null = null;
    getCurrentWindow()
      .onDragDropEvent(handleWindowDragDrop)
      .then((unsubscribe) => {
        unlisten = unsubscribe;
      })
      .catch((error) => {
        console.error('Failed to register drag-drop listener', error);
      });

    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, [handleWindowDragDrop]);

  return { isWindowFocused };
}
