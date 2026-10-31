import React, { useCallback, useEffect, useMemo } from 'react';
import { emitTo } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useTranslation } from 'react-i18next';
import { BookmarksOverlay } from './BookmarksOverlay';
import { useBookmarks } from '../hooks/useBookmarks';
import { useSearchFilters } from '../hooks/useSearchFilters';
import { DEFAULT_FILTER_PRESETS } from '../types/searchFilters';
import type { Bookmark, BookmarkOpenMode } from '../types/bookmarks';
import { BOOKMARK_OPEN_EVENT, type BookmarkOpenPayload } from '../constants/appEvents';

/**
 * Content of the standalone bookmarks window (`index.html?window=bookmarks`).
 *
 * The manager lives in its own OS window so it can be dragged anywhere on screen — including
 * outside the Cardinal window — which is impossible for a DOM overlay. Bookmarks still live in
 * localStorage, shared with the main window because both webviews use the same data store.
 */
export function BookmarksWindow(): React.JSX.Element {
  const { t } = useTranslation();
  const controller = useBookmarks();
  const searchFilters = useSearchFilters();

  const filterOptions = useMemo(
    () =>
      searchFilters.filters.map((filter) => {
        const preset = DEFAULT_FILTER_PRESETS.find((candidate) => candidate.name === filter.name);
        return {
          id: filter.id,
          label: preset?.labelKey ? t(`search.filters.presets.${preset.labelKey}`) : filter.name,
          search: filter.search,
        };
      }),
    [searchFilters.filters, t],
  );

  const handleOpenBookmark = useCallback((bookmark: Bookmark, mode: BookmarkOpenMode) => {
    // Searching happens in the main window; this window only owns the manager UI.
    const payload: BookmarkOpenPayload = { bookmark, mode };
    void emitTo('main', BOOKMARK_OPEN_EVENT, payload);
  }, []);

  const handleClose = useCallback(() => {
    void getCurrentWindow().hide();
  }, []);

  // Esc hides the panel window (the overlay's own Esc handler only closes an in-app panel).
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        event.preventDefault();
        handleClose();
      }
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [handleClose]);

  return (
    <BookmarksOverlay
      open
      variant="window"
      onClose={handleClose}
      controller={controller}
      onOpenBookmark={handleOpenBookmark}
      filterOptions={filterOptions}
    />
  );
}

export default BookmarksWindow;
