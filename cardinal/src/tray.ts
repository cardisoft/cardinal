import { defaultWindowIcon } from '@tauri-apps/api/app';
import { invoke } from '@tauri-apps/api/core';
import { Menu, MenuItem, PredefinedMenuItem } from '@tauri-apps/api/menu';
import { TrayIcon, type TrayIconEvent, type TrayIconOptions } from '@tauri-apps/api/tray';
import i18n from './i18n/config';

const TRAY_ID = 'cardinal.tray';

let trayInitPromise: Promise<void> | null = null;
let trayIcon: TrayIcon | null = null;

export function initializeTray(): Promise<void> {
  if (!trayInitPromise) {
    trayInitPromise = createTray().catch((error) => {
      console.error('Failed to initialize Cardinal tray', error);
      trayInitPromise = null;
    });
  }

  return trayInitPromise;
}

async function createTray(): Promise<void> {
  const options: TrayIconOptions = {
    id: TRAY_ID,
    tooltip: 'Cardinal',
    action: handleTrayAction,
    icon: (await defaultWindowIcon()) ?? undefined,
  };

  trayIcon = await TrayIcon.new(options);
}

function handleTrayAction(event: TrayIconEvent): void {
  if (event.type === 'Click' || event.type === 'DoubleClick') {
    void activateMainWindow();
  }
}

async function activateMainWindow(): Promise<void> {
  await invoke('activate_main_window');
}
