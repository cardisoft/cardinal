import { invoke } from '@tauri-apps/api/core';

export const openResultPath = (path: string | null | undefined): void => {
  if (!path) {
    return;
  }

  void invoke('open_path', { path });
};

// Opens the containing folder in Finder with the item selected.
export const revealResultPath = (path: string | null | undefined): void => {
  if (!path) {
    return;
  }

  void invoke('open_in_finder', { path });
};
