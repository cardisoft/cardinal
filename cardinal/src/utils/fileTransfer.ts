import { invoke } from '@tauri-apps/api/core';
import { open, save } from '@tauri-apps/plugin-dialog';

/**
 * Asks the user for a destination path and writes `contents` there.
 * Returns false when the dialog was cancelled.
 */
export const exportTextFile = async (options: {
  defaultPath: string;
  contents: string;
  filterName: string;
}): Promise<boolean> => {
  const path = await save({
    defaultPath: options.defaultPath,
    filters: [{ name: options.filterName, extensions: ['csv'] }],
  });
  if (!path) {
    return false;
  }
  await invoke('write_text_file', { path, contents: options.contents });
  return true;
};

/**
 * Asks the user for a text file and returns its contents.
 * Returns null when the dialog was cancelled.
 */
export const importTextFile = async (filterName: string): Promise<string | null> => {
  const path = await open({
    multiple: false,
    directory: false,
    filters: [{ name: filterName, extensions: ['csv'] }],
  });
  if (!path || Array.isArray(path)) {
    return null;
  }
  return invoke<string>('read_text_file', { path });
};
