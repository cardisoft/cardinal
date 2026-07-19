import { invoke } from '@tauri-apps/api/core';

type ResultOpenedListener = () => void;

const resultOpenedListeners = new Set<ResultOpenedListener>();

/**
 * Subscribe to successful result-open actions (double-click, Cmd+O, context menu).
 * Returns an unsubscribe function.
 */
export const subscribeResultOpened = (listener: ResultOpenedListener): (() => void) => {
  resultOpenedListeners.add(listener);
  return () => {
    resultOpenedListeners.delete(listener);
  };
};

export const openResultPath = (path: string | null | undefined): void => {
  if (!path) {
    return;
  }

  void invoke('open_path', { path });
  resultOpenedListeners.forEach((listener) => listener());
};
