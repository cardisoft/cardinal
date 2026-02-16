import { useEffect, useRef } from 'react';
import type { MutableRefObject } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { StatusTabKey } from '../components/StatusBar';
import {
  subscribeQuickLookKeydown,
  type QuickLookKeydownPayload,
} from '../runtime/tauriEventRuntime';
import { openResultPath } from '../utils/openResultPath';
import { useStableEvent } from './useStableEvent';

type MoveSelectionOptions = {
  extend?: boolean;
};

type UseAppHotkeysOptions = {
  activeTab: StatusTabKey;
  selectedPaths: string[];
  selectedIndicesRef: MutableRefObject<number[]>;
  focusSearchInput: () => void;
  navigateSelection: (delta: 1 | -1, options?: MoveSelectionOptions) => void;
  triggerQuickLook: () => void;
};

const QUICK_LOOK_KEYCODE_DOWN = 125;
const QUICK_LOOK_KEYCODE_UP = 126;

const isEditableTarget = (target: EventTarget | null): boolean => {
  const element = target as HTMLElement | null;
  if (!element) return false;
  const tagName = element.tagName;
  return tagName === 'INPUT' || tagName === 'TEXTAREA' || element.isContentEditable;
};

export function useAppHotkeys({
  activeTab,
  selectedPaths,
  selectedIndicesRef,
  focusSearchInput,
  navigateSelection,
  triggerQuickLook,
}: UseAppHotkeysOptions): void {
  const keyboardStateRef = useRef<{ activeTab: StatusTabKey }>({
    activeTab,
  });

  useEffect(() => {
    keyboardStateRef.current.activeTab = activeTab;
  }, [activeTab]);

  const handleMetaShortcut = useStableEvent((event: KeyboardEvent, currentTab: StatusTabKey) => {
    const key = event.key.toLowerCase();
    if (key === 'f') {
      event.preventDefault();
      focusSearchInput();
      return true;
    }

    if (currentTab !== 'files') {
      return false;
    }

    if (key === 'r' && selectedPaths.length > 0) {
      event.preventDefault();
      selectedPaths.forEach((path) => {
        void invoke('open_in_finder', { path });
      });
      return true;
    }

    if (key === 'o' && selectedPaths.length > 0) {
      event.preventDefault();
      selectedPaths.forEach((path) => openResultPath(path));
      return true;
    }

    if (key === 'c' && selectedPaths.length > 0) {
      event.preventDefault();
      void invoke('copy_files_to_clipboard', { paths: selectedPaths }).catch((error) => {
        console.error('Failed to copy files to clipboard', error);
      });
      return true;
    }

    return false;
  });

  const handleFilesNavigation = useStableEvent((event: KeyboardEvent) => {
    const target = event.target as HTMLElement | null;
    if (isEditableTarget(target)) {
      return false;
    }

    const isSpaceKey = event.code === 'Space' || event.key === ' ';
    if (isSpaceKey) {
      if (event.repeat || !selectedIndicesRef.current.length) {
        return true;
      }
      event.preventDefault();
      triggerQuickLook();
      return true;
    }

    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      if (event.altKey || event.ctrlKey || event.metaKey) {
        return true;
      }
      event.preventDefault();
      const delta = event.key === 'ArrowDown' ? 1 : -1;
      navigateSelection(delta, { extend: event.shiftKey });
      return true;
    }

    return false;
  });

  useEffect(() => {
    if (typeof window === 'undefined') {
      return;
    }

    const handleKeyDown = (event: KeyboardEvent) => {
      const { activeTab: currentTab } = keyboardStateRef.current;

      if (event.metaKey && handleMetaShortcut(event, currentTab)) {
        return;
      }

      if (currentTab !== 'files') {
        return;
      }

      if (handleFilesNavigation(event)) {
        return;
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [handleMetaShortcut, handleFilesNavigation]);

  const handleQuickLookKeydown = useStableEvent((payload: QuickLookKeydownPayload) => {
    if (keyboardStateRef.current.activeTab !== 'files') {
      return;
    }

    if (!payload || !selectedIndicesRef.current.length) {
      return;
    }

    const { keyCode, modifiers } = payload;
    if (modifiers.command || modifiers.option || modifiers.control) {
      return;
    }

    if (keyCode === QUICK_LOOK_KEYCODE_DOWN) {
      navigateSelection(1, { extend: modifiers.shift });
    } else if (keyCode === QUICK_LOOK_KEYCODE_UP) {
      navigateSelection(-1, { extend: modifiers.shift });
    }
  });

  useEffect(() => {
    const unlistenQuickLook = subscribeQuickLookKeydown(handleQuickLookKeydown);
    return unlistenQuickLook;
  }, [handleQuickLookKeydown]);
}
