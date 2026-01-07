import { invoke } from '@tauri-apps/api/core';

export type WatchConfigPayload = {
  watchRoot: string;
  ignorePaths: string[];
};

export const setWatchConfig = (payload: WatchConfigPayload): Promise<void> => {
  return invoke('set_watch_config', payload);
};
