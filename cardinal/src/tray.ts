import { defaultWindowIcon } from '@tauri-apps/api/app';
import { invoke } from '@tauri-apps/api/core';
import { Menu, MenuItem, PredefinedMenuItem } from '@tauri-apps/api/menu';
import { TrayIcon, type TrayIconOptions } from '@tauri-apps/api/tray';
import i18n from './i18n/config';
import { getStoredShortcutAccelerators } from './shortcuts';

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

export async function refreshTrayMenu(): Promise<void> {
  if (!trayIcon) {
    return;
  }

  const menu = await createTrayMenu();
  await trayIcon.setMenu(menu).catch((error) => {
    console.error('Failed to refresh tray menu', error);
  });
}

async function createTray(): Promise<void> {
  const menu = await createTrayMenu();
  const options: TrayIconOptions = {
    id: TRAY_ID,
    tooltip: 'Cardinal',
    icon: (await defaultWindowIcon()) ?? undefined,
    menu,
  };

  trayIcon = await TrayIcon.new(options);
}

async function createTrayMenu(): Promise<Menu> {
  const shortcuts = getStoredShortcutAccelerators();
  const openItem = await MenuItem.new({
    id: 'tray.open',
    text: i18n.t('tray.open'),
    accelerator: shortcuts.quickLaunch,
    action: () => {
      void activateMainWindow();
    },
  });
  return Menu.new({
    items: [
      openItem,
      await PredefinedMenuItem.new({ item: 'Separator' }),
      await PredefinedMenuItem.new({ item: 'Quit', text: i18n.t('tray.quit') }),
    ],
  });
}

async function activateMainWindow(): Promise<void> {
  await invoke('activate_main_window');
}
