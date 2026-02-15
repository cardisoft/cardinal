import { getName } from '@tauri-apps/api/app';
import { invoke } from '@tauri-apps/api/core';
import { Menu, MenuItem, PredefinedMenuItem, Submenu } from '@tauri-apps/api/menu';
import { openUrl } from '@tauri-apps/plugin-opener';
import i18n from './i18n/config';
import { openPreferences } from './utils/openPreferences';

const HELP_UPDATES_URL = 'https://github.com/cardisoft/cardinal/releases';

let menuInitPromise: Promise<void> | null = null;

export function initializeAppMenu(): Promise<void> {
  if (!menuInitPromise) {
    scheduleMenuBuild();
  }

  return menuInitPromise ?? Promise.resolve();
}

async function buildAppMenu(): Promise<void> {
  const name = (await getName().catch(() => null)) ?? 'Cardinal';
  const aboutItem = await PredefinedMenuItem.new({
    item: { About: null },
    text: i18n.t('menu.about', { appName: name }),
  });
  const preferencesItem = await MenuItem.new({
    id: 'menu.preferences',
    text: i18n.t('menu.preferences'),
    accelerator: 'CmdOrCtrl+,',
    action: () => {
      openPreferences();
    },
  });
  const hideItem = await MenuItem.new({
    id: 'menu.hide',
    text: i18n.t('menu.hide'),
    accelerator: 'Esc',
    action: () => {
      void invoke('hide_main_window');
    },
  });
  const appSubmenu = await Submenu.new({
    id: 'menu.application',
    text: name,
    items: [
      aboutItem,
      await PredefinedMenuItem.new({ item: 'Separator' }),
      preferencesItem,
      hideItem,
      await PredefinedMenuItem.new({ item: 'Separator' }),
      await PredefinedMenuItem.new({
        item: 'Quit',
        text: i18n.t('menu.quit', { appName: name }),
      }),
    ],
  });

  const editSubmenu = await Submenu.new({
    id: 'menu.edit',
    text: i18n.t('menu.edit'),
    items: [
      await PredefinedMenuItem.new({ item: 'Undo', text: i18n.t('menu.undo') }),
      await PredefinedMenuItem.new({ item: 'Redo', text: i18n.t('menu.redo') }),
      await PredefinedMenuItem.new({ item: 'Separator' }),
      await PredefinedMenuItem.new({ item: 'Cut', text: i18n.t('menu.cut') }),
      await PredefinedMenuItem.new({ item: 'Copy', text: i18n.t('menu.copy') }),
      await PredefinedMenuItem.new({ item: 'Paste', text: i18n.t('menu.paste') }),
      await PredefinedMenuItem.new({ item: 'SelectAll', text: i18n.t('menu.selectAll') }),
    ],
  });

  const viewSubmenu = await Submenu.new({
    id: 'menu.view',
    text: i18n.t('menu.view'),
    items: [await PredefinedMenuItem.new({ item: 'Fullscreen', text: i18n.t('menu.fullscreen') })],
  });

  const windowSubmenu = await Submenu.new({
    id: 'menu.window',
    text: i18n.t('menu.window'),
    items: [
      await PredefinedMenuItem.new({ item: 'Minimize', text: i18n.t('menu.minimize') }),
      await PredefinedMenuItem.new({ item: 'Maximize', text: i18n.t('menu.maximize') }),
      await PredefinedMenuItem.new({ item: 'Separator' }),
      await PredefinedMenuItem.new({ item: 'CloseWindow', text: i18n.t('menu.closeWindow') }),
    ],
  });

  const getUpdatesItem = await MenuItem.new({
    id: 'menu.help_updates',
    text: i18n.t('menu.getUpdates'),
    action: () => void openUpdatesPage(),
  });
  const helpSubmenu = await Submenu.new({
    id: 'menu.help-root',
    text: i18n.t('menu.help'),
    items: [getUpdatesItem],
  });

  await helpSubmenu.setAsHelpMenuForNSApp().catch(() => {});

  const menu = await Menu.new({
    items: [appSubmenu, editSubmenu, viewSubmenu, windowSubmenu, helpSubmenu],
  });
  await menu.setAsAppMenu();
}

async function openUpdatesPage(): Promise<void> {
  try {
    await openUrl(HELP_UPDATES_URL);
  } catch (error) {
    console.error('Failed to open updates page', error);
  }
}

function scheduleMenuBuild(): void {
  const start = menuInitPromise ?? Promise.resolve();

  menuInitPromise = start
    .catch(() => {})
    .then(buildAppMenu)
    .catch((error) => {
      console.error('Failed to initialize app menu', error);
      menuInitPromise = null;
    });
}

i18n.on('languageChanged', () => {
  scheduleMenuBuild();
});
