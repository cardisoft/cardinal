import { invoke } from '@tauri-apps/api/core';
import { openResultPath } from './openResultPath';
import { splitPath } from './path';

const writeClipboard = (text: string, errorMessage: string): void => {
  const writePromise = navigator.clipboard?.writeText(text);
  if (!writePromise) {
    return;
  }

  void writePromise.catch((error) => {
    console.error(errorMessage, error);
  });
};

export const openPaths = (paths: string[]): void => {
  paths.forEach((path) => openResultPath(path));
};

export const revealPathsInFinder = (paths: string[]): void => {
  paths.forEach((path) => {
    void invoke('open_in_finder', { path });
  });
};

export const copyFilenamesToClipboard = (paths: string[]): void => {
  const filenames = paths.map((path) => splitPath(path).name || path).join(' ');
  writeClipboard(filenames, 'Failed to copy file names to clipboard');
};

export const copyPathsToClipboard = (paths: string[]): void => {
  writeClipboard(paths.join('\n'), 'Failed to copy paths to clipboard');
};

export const copyFilesToClipboard = (paths: string[]): void => {
  void invoke('copy_files_to_clipboard', { paths }).catch((error) => {
    console.error('Failed to copy files to clipboard', error);
  });
};
