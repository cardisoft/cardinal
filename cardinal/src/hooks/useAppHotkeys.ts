import { useEffect, useRef } from 'react';
import type { MutableRefObject } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { StatusTabKey } from '../components/StatusBar';
import type { ShortcutId, ShortcutMap } from '../shortcuts';
import {
  subscribeQuickLookKeydown,
  type QuickLookKeydownPayload,
} from '../runtime/tauriEventRuntime';
import {
  copyFilesToClipboard,
  copyFilenamesToClipboard,
  copyPathsToClipboard,
  openPaths,
  revealPathsInFinder,
} from '../utils/fileActions';
import { openPreferences } from '../utils/openPreferences';
import { shortcutMatchesKeydown } from '../utils/shortcutCapture';
import { useStableEvent } from './useStableEvent';

type MoveSelectionOptions = {
  extend?: boolean;
};

type UseAppHotkeysOptions = {
  activeTab: StatusTabKey;
  selectedPaths: string[];
  selectedIndicesRef: MutableRefObject<number[]>;
  shortcuts: ShortcutMap;
  enabled: boolean;
  focusSearchInput: () => void;
  navigateSelection: (delta: 1 | -1, options?: MoveSelectionOptions) => void;
  triggerQuickLook: () => void;
};

type ShortcutRule = {
  id: ShortcutId;
  run: (event: KeyboardEvent) => void | boolean;
};

const QUICK_LOOK_KEYCODE_DOWN = 125;
const QUICK_LOOK_KEYCODE_UP = 126;

const isEditableTarget = (target: EventTarget | null): boolean => {
  const element = target as HTMLElement | null;
  if (!element) return false;
  const tagName = element.tagName;
  return tagName === 'INPUT' || tagName === 'TEXTAREA' || element.isContentEditable;
};

const runShortcutRules = (
  event: KeyboardEvent,
  shortcutConfig: ShortcutMap,
  rules: ShortcutRule[],
): boolean => {
  for (const rule of rules) {
    if (!shortcutMatchesKeydown(event, shortcutConfig[rule.id])) {
      continue;
    }
    return rule.run(event) !== false;
  }
  return false;
};

export function useAppHotkeys({
  activeTab,
  selectedPaths,
  selectedIndicesRef,
  shortcuts,
  enabled,
  focusSearchInput,
  navigateSelection,
  triggerQuickLook,
}: UseAppHotkeysOptions): void {
  const keyboardStateRef = useRef<{
    activeTab: StatusTabKey;
    shortcuts: ShortcutMap;
    enabled: boolean;
  }>({
    activeTab,
    shortcuts,
    enabled,
  });

  useEffect(() => {
    keyboardStateRef.current.activeTab = activeTab;
    keyboardStateRef.current.shortcuts = shortcuts;
    keyboardStateRef.current.enabled = enabled;
  }, [activeTab, enabled, shortcuts]);

  const handleWindowShortcuts = useStableEvent(
    (event: KeyboardEvent, shortcutConfig: ShortcutMap) => {
      return runShortcutRules(event, shortcutConfig, [
        {
          id: 'openPreferences',
          run: (keyboardEvent) => {
            keyboardEvent.preventDefault();
            openPreferences();
          },
        },
        {
          id: 'hideWindow',
          run: (keyboardEvent) => {
            keyboardEvent.preventDefault();
            void invoke('hide_main_window');
          },
        },
        {
          id: 'focusSearch',
          run: (keyboardEvent) => {
            keyboardEvent.preventDefault();
            focusSearchInput();
          },
        },
      ]);
    },
  );

  const handleFilesShortcuts = useStableEvent(
    (event: KeyboardEvent, shortcutConfig: ShortcutMap) => {
      const target = event.target as HTMLElement | null;
      // Preserve native copy/edit behavior when focus is inside an editable control.
      // Focus-search is handled earlier by `handleWindowShortcuts`.
      if (isEditableTarget(target)) {
        return false;
      }

      return runShortcutRules(event, shortcutConfig, [
        {
          id: 'revealInFinder',
          run: (keyboardEvent) => {
            if (selectedPaths.length === 0) {
              return false;
            }
            keyboardEvent.preventDefault();
            revealPathsInFinder(selectedPaths);
          },
        },
        {
          id: 'openResult',
          run: (keyboardEvent) => {
            if (selectedPaths.length === 0) {
              return false;
            }
            keyboardEvent.preventDefault();
            openPaths(selectedPaths);
          },
        },
        {
          id: 'copyFilenames',
          run: (keyboardEvent) => {
            if (selectedPaths.length === 0) {
              return false;
            }
            keyboardEvent.preventDefault();
            copyFilenamesToClipboard(selectedPaths);
          },
        },
        {
          id: 'copyPaths',
          run: (keyboardEvent) => {
            if (selectedPaths.length === 0) {
              return false;
            }
            keyboardEvent.preventDefault();
            copyPathsToClipboard(selectedPaths);
          },
        },
        {
          id: 'copyFiles',
          run: (keyboardEvent) => {
            if (selectedPaths.length === 0) {
              return false;
            }
            keyboardEvent.preventDefault();
            copyFilesToClipboard(selectedPaths);
          },
        },
        {
          id: 'quickLook',
          run: (keyboardEvent) => {
            if (keyboardEvent.repeat || !selectedIndicesRef.current.length) {
              return true;
            }
            keyboardEvent.preventDefault();
            triggerQuickLook();
            return true;
          },
        },
        {
          id: 'moveSelectionDown',
          run: (keyboardEvent) => {
            keyboardEvent.preventDefault();
            navigateSelection(1, { extend: false });
          },
        },
        {
          id: 'moveSelectionUp',
          run: (keyboardEvent) => {
            keyboardEvent.preventDefault();
            navigateSelection(-1, { extend: false });
          },
        },
        {
          id: 'extendSelectionDown',
          run: (keyboardEvent) => {
            keyboardEvent.preventDefault();
            navigateSelection(1, { extend: true });
          },
        },
        {
          id: 'extendSelectionUp',
          run: (keyboardEvent) => {
            keyboardEvent.preventDefault();
            navigateSelection(-1, { extend: true });
          },
        },
      ]);
    },
  );

  useEffect(() => {
    if (typeof window === 'undefined') {
      return;
    }

    const handleKeyDown = (event: KeyboardEvent) => {
      const {
        activeTab: currentTab,
        shortcuts: shortcutConfig,
        enabled: shortcutsEnabled,
      } = keyboardStateRef.current;

      if (!shortcutsEnabled) {
        return;
      }

      if (handleWindowShortcuts(event, shortcutConfig)) {
        return;
      }

      if (currentTab !== 'files') {
        return;
      }

      if (handleFilesShortcuts(event, shortcutConfig)) {
        return;
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [handleFilesShortcuts, handleWindowShortcuts]);

  const handleQuickLookKeydown = useStableEvent((payload: QuickLookKeydownPayload) => {
    if (keyboardStateRef.current.activeTab !== 'files' || !keyboardStateRef.current.enabled) {
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
