# Shortcut Customization Overview

This chapter documents how Cardinal implements configurable shortcuts across UI, menu/tray, and global hotkeys.

---

## Scope and goals

The shortcut system is designed to:
- replace hardcoded key bindings with a persisted `ShortcutMap`;
- provide a dedicated shortcut settings overlay from Preferences;
- pause all shortcut handlers while shortcut capture is active;
- keep menu/tray/context-menu labels aligned with current shortcut values.

Current non-goals:
- no shortcut conflict detection or resolution policy;
- no Tauri command-protocol changes for shortcut customization.

---

## Data model and persistence

Implementation: `cardinal/src/shortcuts.ts`

- `ShortcutId` defines all supported shortcut actions.
- `ShortcutMap` is `Record<ShortcutId, string>`.
- `DEFAULT_SHORTCUTS` is the canonical default set.
- `SHORTCUT_DEFINITIONS` is derived from `Object.keys(DEFAULT_SHORTCUTS)` to avoid duplicated metadata.

Storage behavior:
- single storage key: `cardinal.shortcuts`;
- `getStoredShortcuts()` reads and normalizes values, then merges into defaults;
- `persistShortcuts()` normalizes and writes the full map.

There is no legacy quick-launch compatibility layer in the current implementation.

---

## Parsing, capture, and formatting

Implementation: `cardinal/src/utils/shortcutCapture.ts`

The module is the single source for shortcut token rules and conversion:
- `normalizeShortcut()` canonicalizes persisted/user-provided values.
- `captureShortcutFromKeydown(event, requireModifier)` captures a shortcut from keyboard events.
- `shortcutMatchesKeydown()` matches runtime keyboard events against configured shortcuts.
- `formatShortcutForDisplay()` formats values for UI labels (example: `Command+Comma` -> `Cmd+,`).
- `toMenuAccelerator()` converts values to macOS menu accelerator format.

Key alias/display/menu rules are derived from unified token metadata instead of separate ad-hoc maps.

---

## Settings UI flow

Entry points:
- Preferences exposes `Shortcuts -> Configure`.
- The standalone editor is `ShortcutSettingsOverlay`.

Behavior:
- rows are generated from `SHORTCUT_DEFINITIONS`;
- i18n keys follow `shortcutSettings.items.${shortcutId}.label|description`;
- clicking a row button toggles recording mode for that shortcut;
- the next captured key combination updates draft state;
- Reset restores `DEFAULT_SHORTCUTS`, Save commits all draft values.

Capture errors (`modifierRequired`, `keyRequired`, `unsupportedKey`) are mapped to i18n error messages.

---

## Runtime integration

### App-level handlers
- `useAppHotkeys` consumes `ShortcutMap` for window shortcuts and files-tab actions/navigation.
- `useFilesTabState` consumes `searchHistoryUp`/`searchHistoryDown` from `ShortcutMap`.
- `useContextMenu` displays per-action accelerators from current shortcuts.

### Menu and tray
- `menu.ts` reads accelerators from `getStoredShortcutAccelerators()` for Preferences/Hide.
- `tray.ts` shows the current quick-launch accelerator.
- `refreshAppMenu()` and `refreshTrayMenu()` rebuild labels after shortcut updates.

### Global shortcut
- `globalShortcuts.ts` manages registration of the quick-launch shortcut only.
- `initializeGlobalShortcuts()` registers the stored value (with fallback to default on failure).
- `updateQuickLaunchShortcut()` updates runtime registration and persists quick-launch value.

---

## Pause behavior while settings are open

`useShortcutSettingsController` coordinates the pause/resume behavior:
- `setGlobalShortcutsPaused(true)` pauses the OS-level quick-launch registration;
- `setMenuShortcutsEnabled(false)` removes menu accelerators;
- `App.tsx` passes `enabled: false` to `useAppHotkeys`;
- `App.tsx` passes `shortcutsEnabled: false` to `useFilesTabState`.

Result: while `ShortcutSettingsOverlay` is open, shortcut handlers are effectively disabled across global, menu, and in-window layers.

---

## Save pipeline

When the user saves in `ShortcutSettingsOverlay`, the controller runs:
1. `updateQuickLaunchShortcut(nextShortcuts.quickLaunch)`
2. `persistShortcuts(nextShortcuts)`
3. local React state update (`setShortcuts`)
4. `Promise.all([refreshAppMenu(), refreshTrayMenu()])`

This keeps runtime registration, local persistence, and visible accelerator labels in sync.

---

## Test coverage

Current frontend tests cover:
- shortcut storage defaults, persistence, and accelerator mapping (`shortcuts.test.ts`);
- capture/normalize/match/format behavior (`shortcutCapture.test.ts`);
- settings-overlay capture and save flow (`ShortcutSettingsOverlay.test.tsx`);
- app/file-tab shortcut execution paths (`useAppHotkeys.test.ts`, `useFilesTabState.test.ts`).
