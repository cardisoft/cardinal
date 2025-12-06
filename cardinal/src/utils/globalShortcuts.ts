import { invoke } from '@tauri-apps/api/core';
import { register } from '@tauri-apps/plugin-global-shortcut';

const QUICK_LAUNCH_SHORTCUT = 'CommandOrControl+Shift+Space';
export async function initializeGlobalShortcuts(): Promise<void> {
  try {
    await register(QUICK_LAUNCH_SHORTCUT, (event) => {
      if (event.state === 'Released') {
        void invoke('toggle_main_window');
      }
    });
  } catch (error) {
    console.error('Failed to register global shortcuts', error);
  }
}
