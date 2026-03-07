import { invoke } from '@tauri-apps/api/core';

export const pickFolder = async (): Promise<string | undefined> => {
  try {
    const selectedPath = await invoke<string | null>('pick_folder');
    return selectedPath ?? undefined;
  } catch (error) {
    console.error('Failed to pick folder', error);
    return undefined;
  }
};
