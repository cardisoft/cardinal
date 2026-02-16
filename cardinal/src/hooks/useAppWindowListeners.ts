import { useEffect, useState } from 'react';
import type { StatusTabKey } from '../components/StatusBar';
import {
  subscribeLifecycleState,
  subscribeQuickLaunch,
  subscribeStatusBarUpdate,
  subscribeWindowDragDrop,
  type WindowDragDropEvent,
} from '../runtime/tauriEventRuntime';
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
  useEffect(() => {
    const unlistenStatus = subscribeStatusBarUpdate((payload: StatusBarUpdatePayload) => {
      const { scannedFiles, processedEvents, rescanErrors } = payload;
      handleStatusUpdate(scannedFiles, processedEvents, rescanErrors);
    });
    return unlistenStatus;
  }, [handleStatusUpdate]);

  useEffect(() => {
    const unlistenLifecycle = subscribeLifecycleState((status: AppLifecycleStatus) => {
      setLifecycleState(status);
    });
    return unlistenLifecycle;
  }, [setLifecycleState]);

  useEffect(() => {
    const unlistenQuickLaunch = subscribeQuickLaunch(() => {
      focusSearchInput();
    });
    return unlistenQuickLaunch;
  }, [focusSearchInput]);

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

  const handleWindowDragDrop = useStableEvent((event: WindowDragDropEvent) => {
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
    const unlistenDragDrop = subscribeWindowDragDrop(handleWindowDragDrop);
    return unlistenDragDrop;
  }, [handleWindowDragDrop]);

  return { isWindowFocused };
}
