import { useCallback } from 'react';
import { useStoredState } from './useStoredState';

const STORAGE_KEY = 'cardinal.ignorePaths';
const DEFAULT_IGNORE_PATHS = [
  '# Root-anchored system paths',
  '/Volumes/',
  '/cores/',
  '/dev/',
  '/private/',
  '/System/Applications/**/Contents/Resources/',
  '/System/Volumes/',
  '/usr/share/',
  '/xarts/',

  '',
  '# Common project/build caches',
  'node_modules/',
  '.next/',
  '.bun/',
  '.pnpm/',
  '**/.local/fsindex*',

  '',
  '# Application-specific heavy caches',
  '**/com.docker.docker/Data/',
  '**/Firefox/Profiles/**/sessionstore-backups/',
  '**/Firefox/Profiles/**/storage/default/',
  '**/Firefox/Profiles/**/storage/permanent/',
  '**/Google/Chrome*/Cache/',
  '**/Google/Chrome*/leveldb/',
  '**/IconJar*/Backups/',
  '**/Sublime Text */Index/',
  '**/var/postgres/base/',
  '**/var/postgres/pg_stat_tmp/',
  '**/var/postgres/pg_wal/',
  '**/Spotify/Users/*/pending-messages*',

  '',
  '# Root user-library indexing data',
  '/Library/Biome/',
  '/Library/DuetExpertCenter/',

  '',
  '# Basename folders to ignore anywhere',
  '.cache/',
  '.cocoapods/',
  '.git/',
  '.opam/',
  '__pycache__/',
  'Cache/',
  'Caches/',
  'doc/',
  'Xcode.app/',
  'wharf/',
  'Index.noindex/',
  'TextIndex/',
  'io.tailscale.ipn.macos/',
  '.stversions/',

  '',
  '# File patterns',
  '*.com.google.Chrome',
  '*.pyc',
  '.dat.nosync*',
  'webappsstore.sqlite-wal',
  '.DS_Store',
];

const keepStringEntries = (next: unknown[]): string[] =>
  next.filter((item): item is string => typeof item === 'string');

export function useIgnorePaths() {
  const [ignorePaths, setIgnorePathsState] = useStoredState<string[]>({
    key: STORAGE_KEY,
    defaultValue: DEFAULT_IGNORE_PATHS,
    read: (raw) => {
      const parsed = JSON.parse(raw);
      if (!Array.isArray(parsed)) return null;
      return keepStringEntries(parsed);
    },
    write: (value) => JSON.stringify(value),
    readErrorMessage: 'Unable to read saved ignore paths',
    writeErrorMessage: 'Unable to persist default ignore paths',
  });

  const setIgnorePaths = useCallback(
    (next: string[]) => {
      setIgnorePathsState(next);
    },
    [setIgnorePathsState],
  );

  return { ignorePaths, setIgnorePaths, defaultIgnorePaths: DEFAULT_IGNORE_PATHS };
}
