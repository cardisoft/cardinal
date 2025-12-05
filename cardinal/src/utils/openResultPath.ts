import { invoke } from '@tauri-apps/api/core';

export const openResultPath = (path: string | null | undefined): void => {
  if (!path) {
    return;
  }

  invoke('open_path', { path }).catch((error) => {
    console.error('Failed to open file', error);
  });
};
