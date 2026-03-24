import { normalizeShortcut, toMenuAccelerator } from './utils/shortcutCapture';

export const SHORTCUTS_STORAGE_KEY = 'cardinal.shortcuts';
export const DEFAULT_QUICK_LAUNCH_SHORTCUT = 'Command+Shift+Space';

export type ShortcutId =
  | 'quickLaunch'
  | 'openPreferences'
  | 'hideWindow'
  | 'focusSearch'
  | 'openResult'
  | 'revealInFinder'
  | 'copyFilenames'
  | 'copyFiles'
  | 'copyPaths'
  | 'quickLook'
  | 'moveSelectionUp'
  | 'moveSelectionDown'
  | 'extendSelectionUp'
  | 'extendSelectionDown'
  | 'searchHistoryUp'
  | 'searchHistoryDown';

export type ShortcutMap = Record<ShortcutId, string>;

export const DEFAULT_SHORTCUTS: ShortcutMap = {
  quickLaunch: DEFAULT_QUICK_LAUNCH_SHORTCUT,
  openPreferences: 'Command+Comma',
  hideWindow: 'Esc',
  focusSearch: 'Command+F',
  openResult: 'Command+O',
  revealInFinder: 'Command+R',
  copyFilenames: 'Command+Shift+F',
  copyFiles: 'Command+C',
  copyPaths: 'Command+Shift+C',
  quickLook: 'Space',
  moveSelectionUp: 'Up',
  moveSelectionDown: 'Down',
  extendSelectionUp: 'Shift+Up',
  extendSelectionDown: 'Shift+Down',
  searchHistoryUp: 'Up',
  searchHistoryDown: 'Down',
};

export const SHORTCUT_DEFINITIONS = Object.keys(DEFAULT_SHORTCUTS) as ShortcutId[];

const normalizeValue = (value: unknown): string | null =>
  typeof value === 'string' ? normalizeShortcut(value) : null;

const normalizeShortcutMap = (input: Partial<Record<ShortcutId, unknown>>): ShortcutMap => {
  const next = { ...DEFAULT_SHORTCUTS };
  for (const id of SHORTCUT_DEFINITIONS) {
    const normalized = normalizeValue(input[id]);
    if (normalized) {
      next[id] = normalized;
    }
  }
  return next;
};

export const getStoredShortcuts = (): ShortcutMap => {
  if (typeof window === 'undefined') {
    return DEFAULT_SHORTCUTS;
  }

  try {
    const raw = window.localStorage.getItem(SHORTCUTS_STORAGE_KEY);
    const parsed = raw ? (JSON.parse(raw) as Partial<Record<ShortcutId, unknown>>) : {};
    return normalizeShortcutMap(parsed);
  } catch {
    return DEFAULT_SHORTCUTS;
  }
};

export const getStoredShortcutAccelerators = () => {
  const { quickLaunch, openPreferences, hideWindow } = getStoredShortcuts();
  return {
    quickLaunch,
    openPreferences: toMenuAccelerator(openPreferences),
    hideWindow: toMenuAccelerator(hideWindow),
  };
};

export const persistShortcuts = (shortcuts: ShortcutMap): void => {
  if (typeof window === 'undefined') {
    return;
  }

  const normalized = normalizeShortcutMap(shortcuts);

  try {
    window.localStorage.setItem(SHORTCUTS_STORAGE_KEY, JSON.stringify(normalized));
  } catch {
    // Ignore storage failures.
  }
};
