import { defaultWindowIcon } from '@tauri-apps/api/app';
import { invoke } from '@tauri-apps/api/core';
import { Menu } from '@tauri-apps/api/menu';
import { TrayIcon, type TrayIconEvent, type TrayIconOptions } from '@tauri-apps/api/tray';
import { WebviewWindow } from '@tauri-apps/api/webviewWindow';
import i18n from './i18n/config';

const TRAY_ID = 'cardinal.tray';
const TRAY_MENU_QUIT_ID = 'tray.quit_cardinal';

let trayInitPromise: Promise<void> | null = null;
let trayIcon: TrayIcon | null = null;
let unsubscribeLanguageChange: (() => void) | null = null;

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
  const menu = await buildTrayMenu();
  const options: TrayIconOptions = {
    id: TRAY_ID,
    menu,
    tooltip: 'Cardinal',
    action: handleTrayAction,
    icon: (await defaultWindowIcon()) ?? undefined,
  };

  trayIcon = await TrayIcon.new(options);
  ensureLanguageWatcher();
}

async function buildTrayMenu(): Promise<Menu> {
  return Menu.new({
    items: [
      {
        id: TRAY_MENU_QUIT_ID,
        text: i18n.t('tray.quit', { defaultValue: 'Quit Cardinal' }),
        accelerator: 'CmdOrCtrl+Q',
        action: () => {
          void handleQuitCardinal();
        },
      },
    ],
  });
}

async function refreshTrayMenu(): Promise<void> {
  if (!trayIcon) {
    return;
  }

  const menu = await buildTrayMenu();
  await trayIcon.setMenu(menu);
}

function ensureLanguageWatcher(): void {
  if (unsubscribeLanguageChange) {
    return;
  }

  const handler = () => {
    void refreshTrayMenu();
  };

  i18n.on('languageChanged', handler);
  unsubscribeLanguageChange = () => {
    i18n.off('languageChanged', handler);
    unsubscribeLanguageChange = null;
  };
}

async function handleQuitCardinal(): Promise<void> {
  try {
    await invoke('request_app_exit');
  } catch (error) {
    console.error('Failed to quit Cardinal from tray menu', error);
  }
}

function handleTrayAction(event: TrayIconEvent): void {
  if (event.type !== 'Click' || event.button !== 'Left' || event.buttonState !== 'Up') {
    return;
  }

  void activateMainWindow();
}

async function activateMainWindow(): Promise<void> {
  const window = await WebviewWindow.getByLabel('main');
  if (!window) {
    return;
  }

  try {
    if (await window.isMinimized()) {
      await window.unminimize();
    }

    if (!(await window.isVisible())) {
      await window.show();
    }

    await window.setFocus();
  } catch (error) {
    console.error('Failed to activate Cardinal window from tray', error);
  }
}
