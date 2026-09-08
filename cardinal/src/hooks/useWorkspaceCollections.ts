import { useCallback, useState } from 'react';

export type SavedSearchDraft = {
  name: string;
  query: string;
  filterQuery: string;
  directoryQuery: string;
  directoryScopeOpen: boolean;
  caseSensitive: boolean;
};

export type SavedSearch = SavedSearchDraft & {
  id: string;
  createdAt: number;
};

export type FavoritePath = {
  path: string;
  addedAt: number;
};

const SAVED_SEARCHES_KEY = 'cardinal.workspace.savedSearches';
const FAVORITES_KEY = 'cardinal.workspace.favorites';

const isRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === 'object' && value !== null;

const isSavedSearch = (value: unknown): value is SavedSearch =>
  isRecord(value) &&
  typeof value.id === 'string' &&
  typeof value.name === 'string' &&
  typeof value.query === 'string' &&
  typeof value.filterQuery === 'string' &&
  typeof value.directoryQuery === 'string' &&
  typeof value.directoryScopeOpen === 'boolean' &&
  typeof value.caseSensitive === 'boolean' &&
  typeof value.createdAt === 'number';

const isFavoritePath = (value: unknown): value is FavoritePath =>
  isRecord(value) && typeof value.path === 'string' && typeof value.addedAt === 'number';

const readCollection = <T>(key: string, guard: (value: unknown) => value is T): T[] => {
  if (typeof window === 'undefined') return [];
  try {
    const value = JSON.parse(window.localStorage.getItem(key) ?? '[]');
    if (!Array.isArray(value)) return [];
    const clean = value.filter(guard);
    if (clean.length !== value.length) persistCollection(key, clean);
    return clean;
  } catch {
    return [];
  }
};

const persistCollection = <T>(key: string, value: T[]): void => {
  try {
    window.localStorage.setItem(key, JSON.stringify(value));
  } catch {
    // Keep the current session usable when storage is unavailable.
  }
};

const createId = (): string =>
  globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random().toString(16).slice(2)}`;

export function useWorkspaceCollections() {
  const [savedSearches, setSavedSearches] = useState<SavedSearch[]>(() =>
    readCollection(SAVED_SEARCHES_KEY, isSavedSearch),
  );
  const [favorites, setFavorites] = useState<FavoritePath[]>(() =>
    readCollection(FAVORITES_KEY, isFavoritePath),
  );

  const saveSearch = useCallback((draft: SavedSearchDraft) => {
    const name = draft.name.trim();
    if (!name) return;
    setSavedSearches((current) => {
      const next = [
        ...current.filter((item) => item.name.toLocaleLowerCase() !== name.toLocaleLowerCase()),
        { ...draft, name, id: createId(), createdAt: Date.now() },
      ];
      persistCollection(SAVED_SEARCHES_KEY, next);
      return next;
    });
  }, []);

  const removeSavedSearch = useCallback((id: string) => {
    setSavedSearches((current) => {
      const next = current.filter((item) => item.id !== id);
      persistCollection(SAVED_SEARCHES_KEY, next);
      return next;
    });
  }, []);

  const addFavorites = useCallback((paths: string[]) => {
    setFavorites((current) => {
      const known = new Set(current.map((item) => item.path));
      const additions: FavoritePath[] = [];
      paths.forEach((candidate) => {
        const path = candidate.trim();
        if (!path || known.has(path)) return;
        known.add(path);
        additions.push({ path, addedAt: Date.now() });
      });
      const next = [...current, ...additions];
      persistCollection(FAVORITES_KEY, next);
      return next;
    });
  }, []);

  const removeFavorite = useCallback((path: string) => {
    setFavorites((current) => {
      const next = current.filter((item) => item.path !== path);
      persistCollection(FAVORITES_KEY, next);
      return next;
    });
  }, []);

  return {
    savedSearches,
    favorites,
    saveSearch,
    removeSavedSearch,
    addFavorites,
    removeFavorite,
  };
}
