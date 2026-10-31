import { emit, listen } from '@tauri-apps/api/event';
import { applyThemePreference, getStoredThemePreference } from '../theme';
import { FONT_SIZE_DEFAULT, clampFontSize } from '../types/fontSize';
import { FONT_SIZE_STORAGE_KEY } from '../hooks/useFontSize';

/**
 * Keeps the UI preferences that live outside React (theme, font size) in sync between the main
 * window and the standalone bookmarks window.
 *
 * Each window is a separate webview with its own `document`, so applying a theme only painted
 * the window that changed it — the other stayed light until restarted. They share localStorage
 * but not DOM state, and the `storage` event does not fire across separate webviews, so a Tauri
 * event is the only reliable channel.
 *
 * Language is intentionally not handled here: it needs the i18n instance, which the caller
 * passes in through `onAfterApply`. That keeps this module free of `i18n/config` (importing it
 * pulls in `initReactI18next`, which unit tests of `App` mock away).
 */
export const UI_PREFERENCES_CHANGED_EVENT = 'cardinal:ui-preferences-changed';

const readStoredFontSize = (): number | null => {
  if (typeof window === 'undefined') {
    return null;
  }
  try {
    const parsed = Number.parseFloat(window.localStorage.getItem(FONT_SIZE_STORAGE_KEY) ?? '');
    return Number.isFinite(parsed) ? clampFontSize(parsed) : null;
  } catch {
    return null;
  }
};

type ApplyOptions = {
  /** Called after the theme/font have been applied; used to switch the language. */
  onAfterApply?: () => void;
};

/** Applies the persisted theme and font size to the current document. */
export const applyStoredUiPreferences = ({ onAfterApply }: ApplyOptions = {}): void => {
  applyThemePreference(getStoredThemePreference());

  if (typeof document !== 'undefined') {
    const base = readStoredFontSize() ?? FONT_SIZE_DEFAULT;
    document.documentElement.style.setProperty('--app-font-size', `${base}px`);
  }

  onAfterApply?.();
};

/** Tells the other window to re-read and re-apply the preferences. */
export const broadcastUiPreferencesChanged = (): void => {
  try {
    // `emit` returns a promise under Tauri, but not every test double does.
    void Promise.resolve(emit(UI_PREFERENCES_CHANGED_EVENT)).catch(() => {
      // No Tauri IPC (unit tests, plain browser): there is no other window to notify.
    });
  } catch {
    // Same as above: broadcasting is best-effort.
  }
};

/**
 * Re-applies the preferences whenever another window announces a change.
 * Returns an unsubscribe function.
 */
export const subscribeUiPreferencesChanged = ({
  onAfterApply,
}: ApplyOptions = {}): (() => void) => {
  let unlisten: (() => void) | undefined;
  let cancelled = false;
  void listen(UI_PREFERENCES_CHANGED_EVENT, () => {
    applyStoredUiPreferences({ onAfterApply });
  })
    .then((fn) => {
      // The listener may resolve after the caller already unsubscribed.
      if (cancelled) {
        fn();
        return;
      }
      unlisten = fn;
    })
    .catch((error) => {
      console.warn('Unable to subscribe to UI preference changes', error);
    });
  return () => {
    cancelled = true;
    unlisten?.();
  };
};
