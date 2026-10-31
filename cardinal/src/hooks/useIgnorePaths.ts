import { useCallback } from 'react';
import { useStoredState } from './useStoredState';

const STORAGE_KEY = 'cardinal.ignorePaths';
/**
 * Defaults that have already been merged into this profile's list. A default is added at most
 * once, so removing it later is a durable choice — unlike the old "user edited anything"
 * boolean, which disabled migrations forever after a single Preferences save.
 */
const APPLIED_DEFAULTS_KEY = 'cardinal.ignorePaths.defaultsApplied';
/** Legacy marker from the previous (broken) migration; removed on sight. */
const LEGACY_USER_EDITED_KEY = 'cardinal.ignorePaths.userEdited';
// Ignore generated, disposable, or high-churn macOS paths by default:
// - `/Volumes` can include external drives and network mounts that are large or slow to traverse.
// - `~/Library/CloudStorage` can contain cloud stubs that trigger downloads or network I/O.
// - `~/Library/Biome` stores system suggestions/activity data rather than user-authored files.
// - `~/Library/Caches`, `~/Library/Logs`, and `~/Library/Metadata` are cache/log/metadata stores rather than user-authored files.
// - `/Library/Caches` and `/System/Library/Caches` are transient system caches with low search value.
// - `/private/var` is a broad system runtime area for temp files, caches, logs, and databases.
// - `/private/tmp` is a temporary-file area with very high churn and little search value.
const DEFAULT_IGNORE_PATHS = [
  '/Volumes',
  '~/Library/CloudStorage',
  '~/Library/Biome',
  '~/Library/Caches',
  '~/Library/Logs',
  '~/Library/Metadata',
  '/Library/Caches',
  '/System/Library/Caches',
  '/private/var',
  '/private/tmp',
];

const cleanPaths = (next: string[]): string[] =>
  next.map((item) => item.trim()).filter((item) => item.length > 0);

/**
 * Defaults shipped by the release that introduced the `defaultsApplied` bookkeeping. A profile
 * without that key was written by an older build, so these are considered already applied —
 * only defaults added *after* this point get merged.
 */
const BASELINE_APPLIED_DEFAULTS = [
  // The v1 defaults — deliberately WITHOUT `/Volumes`, which is the whole reason this
  // migration exists. A profile with no bookkeeping is treated as having applied exactly
  // these, so `/Volumes` is the one default that gets merged into it.
  '~/Library/CloudStorage',
  '~/Library/Biome',
  '~/Library/Caches',
  '~/Library/Logs',
  '~/Library/Metadata',
  '/Library/Caches',
  '/System/Library/Caches',
  '/private/var',
  '/private/tmp',
];

/**
 * Defaults to append to a stored list.
 *
 * A default is merged only when it has never been applied to this profile, which is what makes
 * a user's deletion durable while still delivering new defaults on upgrade. `null` means
 * nothing to do.
 */
export function migrateStoredIgnorePaths(
  stored: string[],
  appliedDefaults: readonly string[],
): string[] | null {
  const applied = new Set(appliedDefaults);
  const missing = DEFAULT_IGNORE_PATHS.filter(
    (path) => !applied.has(path) && !stored.includes(path),
  );
  if (missing.length === 0) {
    return null;
  }
  // An empty list means the user removed everything on purpose; do not resurrect defaults.
  if (stored.length === 0) {
    return null;
  }
  return [...stored, ...missing];
}

const readAppliedDefaults = (): string[] => {
  try {
    const raw = window.localStorage.getItem(APPLIED_DEFAULTS_KEY);
    if (raw == null) {
      // Older profile: treat the baseline as applied so only new defaults are merged.
      return [...BASELINE_APPLIED_DEFAULTS];
    }
    const parsed: unknown = JSON.parse(raw);
    return Array.isArray(parsed)
      ? parsed.filter((item): item is string => typeof item === 'string')
      : [];
  } catch {
    return [...BASELINE_APPLIED_DEFAULTS];
  }
};

/**
 * Runs the migration before `useStoredState` reads the value, so `start_logic` sees the
 * merged list on the very first launch after the upgrade.
 */
function applyIgnorePathMigration(): void {
  if (typeof window === 'undefined') {
    return;
  }
  try {
    window.localStorage.removeItem(LEGACY_USER_EDITED_KEY);

    const applied = readAppliedDefaults();
    const raw = window.localStorage.getItem(STORAGE_KEY);
    if (raw != null) {
      const parsed: unknown = JSON.parse(raw);
      if (Array.isArray(parsed)) {
        const stored = cleanPaths(
          parsed.filter((item): item is string => typeof item === 'string'),
        );
        const migrated = migrateStoredIgnorePaths(stored, applied);
        if (migrated != null) {
          window.localStorage.setItem(STORAGE_KEY, JSON.stringify(migrated));
          console.info('Merged new default ignore paths into the saved list', migrated);
        }
      }
    }

    // Record every default as applied so a later deletion of one is never undone. Only write
    // when it actually changes, so a normal boot does not touch storage.
    const nextApplied = Array.from(new Set([...applied, ...DEFAULT_IGNORE_PATHS]));
    if (nextApplied.length !== applied.length) {
      window.localStorage.setItem(APPLIED_DEFAULTS_KEY, JSON.stringify(nextApplied));
    }
  } catch (error) {
    console.warn('Unable to migrate saved ignore paths', error);
  }
}

export function useIgnorePaths() {
  // Synchronous and idempotent: must run before the stored value is read.
  applyIgnorePathMigration();

  const [ignorePaths, setIgnorePathsState] = useStoredState<string[]>({
    key: STORAGE_KEY,
    defaultValue: DEFAULT_IGNORE_PATHS,
    read: (raw) => {
      const parsed = JSON.parse(raw);
      if (!Array.isArray(parsed)) return null;
      return cleanPaths(parsed.filter((item): item is string => typeof item === 'string'));
    },
    write: (value) => JSON.stringify(value),
    readErrorMessage: 'Unable to read saved ignore paths',
    writeErrorMessage: 'Unable to persist default ignore paths',
  });

  const setIgnorePaths = useCallback(
    (next: string[]) => {
      // No "user edited" flag any more: migration is tracked per default
      // (`APPLIED_DEFAULTS_KEY`), so editing this list never blocks future defaults.
      setIgnorePathsState(cleanPaths(next));
    },
    [setIgnorePathsState],
  );

  return { ignorePaths, setIgnorePaths, defaultIgnorePaths: DEFAULT_IGNORE_PATHS };
}
