import { useCallback, useEffect } from 'react';
import { emit } from '@tauri-apps/api/event';
import { useStoredState } from './useStoredState';
import { BOOKMARKS_CHANGED_EVENT } from '../constants/appEvents';
import { decodeCsv, encodeCsv } from '../utils/csv';
import {
  DEFAULT_BOOKMARK_CATEGORY_ID,
  isDefaultCategory,
  type Bookmark,
  type BookmarkCategory,
  type BookmarksState,
} from '../types/bookmarks';

const BOOKMARKS_STORAGE_KEY = 'cardinal.bookmarks.v1';

// Generates collision-resistant ids without external deps.
const createId = (): string => {
  const random = Math.random().toString(36).slice(2, 10);
  return `${Date.now().toString(36)}-${random}`;
};

type StoredBookmarks = {
  categories: BookmarkCategory[];
  bookmarks: Bookmark[];
};

const readBookmarks = (raw: string): BookmarksState | null => {
  try {
    const parsed = JSON.parse(raw) as Partial<StoredBookmarks>;
    if (!parsed || !Array.isArray(parsed.categories) || !Array.isArray(parsed.bookmarks)) {
      return null;
    }

    const categories: BookmarkCategory[] = parsed.categories
      .filter(
        (category): category is BookmarkCategory =>
          !!category && typeof category.id === 'string' && typeof category.name === 'string',
      )
      .map((category) => ({ id: category.id, name: category.name }));

    const bookmarks: Bookmark[] = parsed.bookmarks
      .filter(
        (bookmark): bookmark is Bookmark =>
          !!bookmark &&
          typeof bookmark.id === 'string' &&
          typeof bookmark.categoryId === 'string' &&
          typeof bookmark.name === 'string' &&
          typeof bookmark.query === 'string',
      )
      .map((bookmark) => ({
        id: bookmark.id,
        categoryId: bookmark.categoryId,
        name: bookmark.name,
        query: bookmark.query,
        // Older payloads predate filters; they simply have none.
        filterSearch: typeof bookmark.filterSearch === 'string' ? bookmark.filterSearch : '',
      }));

    return withDefaultCategory({ categories, bookmarks });
  } catch {
    return null;
  }
};

const writeBookmarks = (state: BookmarksState): string => JSON.stringify(state);

const BOOKMARK_HEADER_WORDS = new Set(['category', 'name', 'query', 'search', 'filter']);

/** True when every cell of `row` is a known column title (i.e. the row is a header). */
const looksLikeBookmarkHeader = (row: readonly string[]): boolean =>
  row.length > 0 && row.every((cell) => BOOKMARK_HEADER_WORDS.has(cell.trim().toLowerCase()));

/**
 * Guarantees the built-in category exists and sits first. The category cannot be deleted, so
 * Cmd+S always has somewhere to write and imported rows without a category have a home.
 */
const withDefaultCategory = (state: BookmarksState): BookmarksState => {
  const hasDefault = state.categories.some((category) => isDefaultCategory(category.id));
  const categories = hasDefault
    ? state.categories
    : [
        { id: DEFAULT_BOOKMARK_CATEGORY_ID, name: state.categories.length > 0 ? '默认' : '默认' },
        ...state.categories,
      ];
  return { categories, bookmarks: state.bookmarks };
};

const defaultState = (): BookmarksState => withDefaultCategory({ categories: [], bookmarks: [] });

export type BookmarkInput = {
  name: string;
  query: string;
  filterSearch?: string;
};

export type BookmarksController = {
  categories: BookmarkCategory[];
  bookmarks: Bookmark[];
  defaultCategoryId: string;
  /** Serialises every bookmark as `category,name,query,filter` rows. */
  exportCsv: () => string;
  /** Merges `category,name,query,filter` rows, reusing categories by name. */
  importCsv: (csv: string) => { categories: number; bookmarks: number };
  addCategory: (name: string) => string | null;
  renameCategory: (categoryId: string, name: string) => void;
  deleteCategory: (categoryId: string) => void;
  addBookmark: (categoryId: string, input: BookmarkInput) => void;
  updateBookmark: (bookmarkId: string, input: BookmarkInput) => void;
  deleteBookmark: (bookmarkId: string) => void;
  moveBookmark: (bookmarkId: string, direction: -1 | 1) => void;
};

/**
 * Persistent bookmark manager. Layout follows the Chrome/Finder "categories on the left,
 * bookmarks on the right" model: a category acts as a folder and bookmarks are its contents.
 */
export function useBookmarks(): BookmarksController {
  const [state, setState] = useStoredState<BookmarksState>({
    key: BOOKMARKS_STORAGE_KEY,
    // Bookmarks live in localStorage, which both windows share. The event keeps the separate
    // bookmarks window in sync with Cmd+S saves and imports done in the main window.
    syncEvent: BOOKMARKS_CHANGED_EVENT,
    defaultValue: defaultState(),
    read: readBookmarks,
    write: writeBookmarks,
    normalize: withDefaultCategory,
    readErrorMessage: 'Unable to read saved bookmarks',
    writeErrorMessage: 'Unable to persist bookmarks',
  });

  const { categories, bookmarks } = state;

  /**
   * Writes the bookmarks and tells the other window (the bookmarks manager lives in its own
   * OS window) to re-read localStorage. Without the broadcast, a Cmd+S save in the main window
   * would stay invisible in the manager until it was reopened.
   */
  const mutate = useCallback(
    (updater: (prev: BookmarksState) => BookmarksState) => {
      setState(updater);
      try {
        void Promise.resolve(emit(BOOKMARKS_CHANGED_EVENT)).catch(() => {
          // No Tauri IPC (unit tests, plain browser): nothing to broadcast to.
        });
      } catch {
        // Broadcasting is best-effort; the local write already happened.
      }
    },
    [setState],
  );

  const addCategory = useCallback(
    (name: string): string | null => {
      const trimmed = name.trim();
      if (!trimmed) {
        return null;
      }
      const id = createId();
      mutate((prev) => ({
        ...prev,
        categories: [...prev.categories, { id, name: trimmed }],
      }));
      return id;
    },
    [mutate],
  );

  const renameCategory = useCallback(
    (categoryId: string, name: string) => {
      const trimmed = name.trim();
      if (!trimmed || isDefaultCategory(categoryId)) {
        return;
      }
      mutate((prev) => ({
        ...prev,
        categories: prev.categories.map((category) =>
          category.id === categoryId ? { ...category, name: trimmed } : category,
        ),
      }));
    },
    [mutate],
  );

  const deleteCategory = useCallback(
    (categoryId: string) => {
      // The default category is a permanent home for Cmd+S saves and imports.
      if (isDefaultCategory(categoryId)) {
        return;
      }
      mutate((prev) => ({
        ...prev,
        categories: prev.categories.filter((category) => category.id !== categoryId),
        bookmarks: prev.bookmarks.filter((bookmark) => bookmark.categoryId !== categoryId),
      }));
    },
    [mutate],
  );

  const addBookmark = useCallback(
    (categoryId: string, input: BookmarkInput) => {
      let trimmedName = input.name.trim();
      let trimmedQuery = input.query.trim();
      // A bookmark only needs to be identifiable: allow filling just the display name or
      // just the search text, and mirror the other field (like Chrome's behaviour).
      if (!trimmedName && !trimmedQuery) {
        return;
      }
      if (!trimmedName) {
        trimmedName = trimmedQuery;
      }
      if (!trimmedQuery) {
        trimmedQuery = trimmedName;
      }
      mutate((prev) => ({
        ...prev,
        bookmarks: [
          ...prev.bookmarks,
          {
            id: createId(),
            categoryId,
            name: trimmedName,
            query: trimmedQuery,
            filterSearch: (input.filterSearch ?? '').trim(),
          },
        ],
      }));
    },
    [mutate],
  );

  const updateBookmark = useCallback(
    (bookmarkId: string, input: BookmarkInput) => {
      let trimmedName = input.name.trim();
      let trimmedQuery = input.query.trim();
      if (!trimmedName && !trimmedQuery) {
        return;
      }
      if (!trimmedName) {
        trimmedName = trimmedQuery;
      }
      if (!trimmedQuery) {
        trimmedQuery = trimmedName;
      }
      mutate((prev) => ({
        ...prev,
        bookmarks: prev.bookmarks.map((bookmark) =>
          bookmark.id === bookmarkId
            ? {
                ...bookmark,
                name: trimmedName,
                query: trimmedQuery,
                filterSearch: (input.filterSearch ?? '').trim(),
              }
            : bookmark,
        ),
      }));
    },
    [mutate],
  );

  const deleteBookmark = useCallback(
    (bookmarkId: string) => {
      mutate((prev) => ({
        ...prev,
        bookmarks: prev.bookmarks.filter((bookmark) => bookmark.id !== bookmarkId),
      }));
    },
    [mutate],
  );

  const moveBookmark = useCallback(
    (bookmarkId: string, direction: -1 | 1) => {
      mutate((prev) => {
        const target = prev.bookmarks.find((bookmark) => bookmark.id === bookmarkId);
        if (!target) {
          return prev;
        }
        const siblings = prev.bookmarks.filter(
          (bookmark) => bookmark.categoryId === target.categoryId,
        );
        const indexInCategory = siblings.findIndex((bookmark) => bookmark.id === bookmarkId);
        const nextIndex = indexInCategory + direction;
        const swappedWith = siblings[nextIndex];
        if (indexInCategory < 0 || !swappedWith) {
          return prev;
        }
        // Swap the payload but keep the ids in place so React keys stay stable.
        return {
          ...prev,
          bookmarks: prev.bookmarks.map((bookmark) => {
            if (bookmark.id === target.id) {
              return { ...swappedWith, id: target.id };
            }
            if (bookmark.id === swappedWith.id) {
              return { ...target, id: swappedWith.id };
            }
            return bookmark;
          }),
        };
      });
    },
    [mutate],
  );

  const exportCsv = useCallback((): string => {
    const categoryNames = new Map(categories.map((category) => [category.id, category.name]));
    const rows: string[][] = [['category', 'name', 'query', 'filter']];
    bookmarks.forEach((bookmark) => {
      // The default category is exported with an empty category cell so files stay portable,
      // and empty-category rows import back into it.
      const categoryName = isDefaultCategory(bookmark.categoryId)
        ? ''
        : (categoryNames.get(bookmark.categoryId) ?? '');
      rows.push([categoryName, bookmark.name, bookmark.query, bookmark.filterSearch]);
    });

    // Categories without any bookmark would otherwise vanish from the export; emit a
    // name-less row for each so the grouping survives a round-trip.
    const usedCategoryIds = new Set(bookmarks.map((bookmark) => bookmark.categoryId));
    categories.forEach((category) => {
      if (isDefaultCategory(category.id) || usedCategoryIds.has(category.id)) {
        return;
      }
      rows.push([category.name, '', '', '']);
    });

    return encodeCsv(rows);
  }, [bookmarks, categories]);

  const importCsv = useCallback(
    (csv: string): { categories: number; bookmarks: number } => {
      const rows = decodeCsv(csv);
      if (rows.length === 0) {
        return { categories: 0, bookmarks: 0 };
      }

      // Skip the header row only when every cell looks like a column name, so headerless
      // files (and a bookmark literally called "name") still import.
      const body = looksLikeBookmarkHeader(rows[0]) ? rows.slice(1) : rows;

      let addedCategories = 0;
      let addedBookmarks = 0;

      mutate((prev) => {
        const categories = [...prev.categories];
        const bookmarks = [...prev.bookmarks];
        const byName = new Map(categories.map((category) => [category.name, category.id]));

        body.forEach((row) => {
          const categoryName = (row[0] ?? '').trim();
          const name = (row[1] ?? '').trim();
          const query = (row[2] ?? '').trim();
          const filterSearch = (row[3] ?? '').trim();

          let categoryId = categoryName ? byName.get(categoryName) : DEFAULT_BOOKMARK_CATEGORY_ID;
          if (!categoryId) {
            categoryId = createId();
            categories.push({ id: categoryId, name: categoryName });
            byName.set(categoryName, categoryId);
            addedCategories += 1;
          }

          if (!name && !query) {
            // A category-only row still creates the (empty) category, matching what the user
            // typed in the CSV rather than silently dropping it.
            return;
          }

          bookmarks.push({
            id: createId(),
            categoryId,
            name: name || query,
            query: query || name,
            filterSearch,
          });
          addedBookmarks += 1;
        });

        if (addedBookmarks === 0 && addedCategories === 0) {
          return prev;
        }
        return { categories, bookmarks };
      });

      return { categories: addedCategories, bookmarks: addedBookmarks };
    },
    [mutate],
  );

  return {
    categories,
    bookmarks,
    defaultCategoryId: DEFAULT_BOOKMARK_CATEGORY_ID,
    exportCsv,
    importCsv,
    addCategory,
    renameCategory,
    deleteCategory,
    addBookmark,
    updateBookmark,
    deleteBookmark,
    moveBookmark,
  };
}
