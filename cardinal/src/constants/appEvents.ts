export const OPEN_PREFERENCES_EVENT = 'cardinal:open-preferences';
export const OPEN_BOOKMARKS_EVENT = 'cardinal:open-bookmarks';

/**
 * Tauri event the bookmarks window emits when a bookmark should be searched in the main
 * window. Payload: `BookmarkOpenPayload`.
 */
export const BOOKMARK_OPEN_EVENT = 'cardinal:bookmark-open';

/** Emitted after any bookmark mutation so the other window can re-read localStorage. */
export const BOOKMARKS_CHANGED_EVENT = 'cardinal:bookmarks-changed';

export type BookmarkOpenPayload = {
  bookmark: { id: string; categoryId: string; name: string; query: string; filterSearch: string };
  mode: 'current' | 'new';
};
