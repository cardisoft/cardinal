import { useEffect, useState } from 'react';
import type { RefObject } from 'react';
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
};

type UseAppWindowListenersOptions = {
  activeTab: StatusTabKey;
  searchInputRef: RefObject<HTMLInputElement>;
  focusAndSelectSearchInput: () => void;
  handleStatusUpdate: (scannedFiles: number, processedEvents: number, rescanErrors: number) => void;
  setLifecycleState: (status: AppLifecycleStatus) => void;
  submitFilesQuery: (query: string, options?: QueueSearchOptions) => void;
  setEventFilterQuery: (value: string) => void;
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
  searchInputRef,
  focusAndSelectSearchInput,
  handleStatusUpdate,
  setLifecycleState,
  submitFilesQuery,
  setEventFilterQuery,
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
      focusAndSelectSearchInput();
    });
    return unlistenQuickLaunch;
  }, [focusAndSelectSearchInput]);

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
    if (typeof document === 'undefined') {
      return;
    }
    const searchInput = searchInputRef.current;
    if (!searchInput) {
      return;
    }
    const scale = window.devicePixelRatio || 1;
    const dropTarget = document.elementFromPoint(
      payload.position.x / scale,
      payload.position.y / scale,
    );
    if (dropTarget !== searchInput) {
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
    submitFilesQuery(query, { immediate: true });
  });

  useEffect(() => {
    const unlistenDragDrop = subscribeWindowDragDrop(handleWindowDragDrop);
    return unlistenDragDrop;
  }, [handleWindowDragDrop]);

  return { isWindowFocused };
}
