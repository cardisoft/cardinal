import { invoke } from '@tauri-apps/api/core';
import { register, unregister } from '@tauri-apps/plugin-global-shortcut';
import { DEFAULT_QUICK_LAUNCH_SHORTCUT, getStoredShortcuts, persistShortcuts } from '../shortcuts';

let registeredQuickLaunchShortcut: string | null = null;
let globalShortcutsPaused = false;

const handleQuickLaunchShortcut = (event: { state: string }): void => {
  if (event.state === 'Released') {
    void invoke('toggle_main_window');
  }
};

const registerQuickLaunchShortcut = async (shortcut: string): Promise<void> => {
  await register(shortcut, handleQuickLaunchShortcut);
  registeredQuickLaunchShortcut = shortcut;
};

const saveQuickLaunchShortcut = (shortcut: string): void => {
  persistShortcuts({
    ...getStoredShortcuts(),
    quickLaunch: shortcut,
  });
};

export async function initializeGlobalShortcuts(): Promise<void> {
  const preferredShortcut = getStoredShortcuts().quickLaunch;

  try {
    await registerQuickLaunchShortcut(preferredShortcut);
  } catch (error) {
    console.error('Failed to register global shortcuts', error);
    if (preferredShortcut !== DEFAULT_QUICK_LAUNCH_SHORTCUT) {
      try {
        await registerQuickLaunchShortcut(DEFAULT_QUICK_LAUNCH_SHORTCUT);
        saveQuickLaunchShortcut(DEFAULT_QUICK_LAUNCH_SHORTCUT);
      } catch (fallbackError) {
        console.error('Failed to register fallback global shortcut', fallbackError);
      }
    }
  }
}

export async function updateQuickLaunchShortcut(shortcut: string): Promise<void> {
  const nextShortcut = shortcut.trim();
  if (!nextShortcut) {
    throw new Error('Shortcut cannot be empty');
  }

  const previousShortcut = registeredQuickLaunchShortcut;
  if (previousShortcut === nextShortcut) {
    return;
  }

  if (globalShortcutsPaused) {
    registeredQuickLaunchShortcut = nextShortcut;
    saveQuickLaunchShortcut(nextShortcut);
    return;
  }

  if (previousShortcut) {
    await unregister(previousShortcut).catch(() => {});
  }

  try {
    await registerQuickLaunchShortcut(nextShortcut);
    saveQuickLaunchShortcut(nextShortcut);
  } catch (error) {
    if (previousShortcut) {
      try {
        await registerQuickLaunchShortcut(previousShortcut);
      } catch (restoreError) {
        console.error('Failed to restore previous global shortcut', restoreError);
      }
    } else {
      registeredQuickLaunchShortcut = null;
    }
    throw error;
  }
}

export async function setGlobalShortcutsPaused(paused: boolean): Promise<void> {
  if (globalShortcutsPaused === paused) {
    return;
  }

  globalShortcutsPaused = paused;

  if (paused) {
    if (registeredQuickLaunchShortcut) {
      await unregister(registeredQuickLaunchShortcut).catch(() => {});
    }
    return;
  }

  const shortcutToRegister = registeredQuickLaunchShortcut ?? getStoredShortcuts().quickLaunch;
  await registerQuickLaunchShortcut(shortcutToRegister).catch((error) => {
    console.error('Failed to resume global shortcuts', error);
  });
}
