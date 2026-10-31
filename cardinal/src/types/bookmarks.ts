export type BookmarkCategory = Readonly<{
  id: string;
  name: string;
}>;

/** Id of the built-in category that receives Cmd+S saves and imported uncategorised rows. */
export const DEFAULT_BOOKMARK_CATEGORY_ID = 'default';

export type Bookmark = Readonly<{
  id: string;
  categoryId: string;
  // Display name shown in the bookmark list.
  name: string;
  // Actual search text used when the bookmark is opened.
  query: string;
  // Filter applied when the bookmark is opened, stored as the preset's search text
  // (e.g. `ext:mp4;mkv`). Empty string means "no filter". Storing the search text instead of
  // a preset id keeps the bookmark valid after the preset is renamed or deleted, and makes
  // CSV round-trips portable.
  filterSearch: string;
}>;

export type BookmarksState = Readonly<{
  categories: BookmarkCategory[];
  bookmarks: Bookmark[];
}>;

export type BookmarkOpenMode = 'current' | 'new';

export type BookmarkOpenRequest = Readonly<{
  query: string;
  filterSearch: string;
  mode: BookmarkOpenMode;
}>;

export const isDefaultCategory = (categoryId: string): boolean =>
  categoryId === DEFAULT_BOOKMARK_CATEGORY_ID;
