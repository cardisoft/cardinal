import { defaultWindowIcon } from '@tauri-apps/api/app';
import { invoke } from '@tauri-apps/api/core';
import { Menu, MenuItem, PredefinedMenuItem } from '@tauri-apps/api/menu';
import { TrayIcon, type TrayIconOptions } from '@tauri-apps/api/tray';
import i18n from './i18n/config';
import { QUICK_LAUNCH_SHORTCUT } from './utils/globalShortcuts';

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

export async function setTrayEnabled(enabled: boolean): Promise<void> {
  await invoke('set_tray_activation_policy', { enabled }).catch((error) => {
    console.error('Failed to update activation policy', error);
  });

  if (enabled) {
    await initializeTray();
    return;
  }

  const pendingInit = trayInitPromise;
  trayInitPromise = null;

  await pendingInit?.catch(() => {});

  const current = trayIcon;
  trayIcon = null;

  await Promise.allSettled([current?.close(), TrayIcon.removeById(TRAY_ID)]);
}

async function createTray(): Promise<void> {
  const openItem = await MenuItem.new({
    id: 'tray.open',
    text: i18n.t('tray.open'),
    accelerator: QUICK_LAUNCH_SHORTCUT,
    action: () => {
      void activateMainWindow();
    },
  });
  const menu = await Menu.new({
    items: [
      openItem,
      await PredefinedMenuItem.new({ item: 'Separator' }),
      await PredefinedMenuItem.new({ item: 'Quit', text: i18n.t('tray.quit') }),
    ],
  });
  const options: TrayIconOptions = {
    id: TRAY_ID,
    tooltip: 'Cardinal',
    icon: (await defaultWindowIcon()) ?? undefined,
    menu,
  };

  trayIcon = await TrayIcon.new(options);
}

async function activateMainWindow(): Promise<void> {
  await invoke('activate_main_window');
}
