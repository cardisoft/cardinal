import { invoke } from '@tauri-apps/api/core';

export const openResultPath = (path: string | null | undefined): void => {
  if (!path) {
    return;
  }

  void invoke('open_path', { path });
};
