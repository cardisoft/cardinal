import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { Bookmark, BookmarkOpenMode } from '../types/bookmarks';
import type { BookmarksController } from '../hooks/useBookmarks';
import { exportTextFile, importTextFile } from '../utils/fileTransfer';
import { usePanelDrag, type ResizeHandleName } from '../hooks/usePanelDrag';
import { startNativeWindowDrag } from '../utils/windowDrag';

type BookmarksOverlayProps = {
  open: boolean;
  onClose: () => void;
  controller: BookmarksController;
  onOpenBookmark: (bookmark: Bookmark, mode: BookmarkOpenMode) => void;
  /** Filter presets offered in the bookmark editor, identified by their search text. */
  filterOptions: { id: string; label: string; search?: string }[];
  /**
   * `window` renders the manager as the content of its own OS window: the panel fills the
   * viewport, the header drags the native window and there is no in-app positioning.
   */
  variant?: 'overlay' | 'window';
};

type BookmarkDraft = {
  id: string | null;
  name: string;
  query: string;
  filterSearch: string;
};

type ContextMenuState = {
  bookmark: Bookmark;
  x: number;
  y: number;
};

const RESIZE_HANDLES: Array<{
  name: ResizeHandleName;
  labelKey: string;
  orientation: 'horizontal' | 'vertical';
}> = [
  { name: 'top', labelKey: 'bookmarks.resizeTop', orientation: 'horizontal' },
  { name: 'bottom', labelKey: 'bookmarks.resizeHeight', orientation: 'horizontal' },
  { name: 'left', labelKey: 'bookmarks.resizeWidth', orientation: 'vertical' },
  { name: 'right', labelKey: 'bookmarks.resizeRight', orientation: 'vertical' },
  { name: 'topLeft', labelKey: 'bookmarks.resizeCorner', orientation: 'vertical' },
  { name: 'topRight', labelKey: 'bookmarks.resizeCorner', orientation: 'vertical' },
  { name: 'bottomLeft', labelKey: 'bookmarks.resizeCorner', orientation: 'horizontal' },
  { name: 'bottomRight', labelKey: 'bookmarks.resizeCorner', orientation: 'horizontal' },
];

const emptyDraft = (): BookmarkDraft => ({
  id: null,
  name: '',
  query: '',
  filterSearch: '',
});

export function BookmarksOverlay({
  open,
  onClose,
  controller,
  onOpenBookmark,
  filterOptions,
  variant = 'overlay',
}: BookmarksOverlayProps): React.JSX.Element | null {
  const { t } = useTranslation();
  const {
    categories,
    bookmarks,
    defaultCategoryId,
    exportCsv,
    importCsv,
    addCategory,
    renameCategory,
    deleteCategory,
    addBookmark,
    updateBookmark,
    deleteBookmark,
    moveBookmark,
  } = controller;

  const [selectedCategoryId, setSelectedCategoryId] = useState<string | null>(null);
  const [categoryDraft, setCategoryDraft] = useState('');
  const [renamingCategoryId, setRenamingCategoryId] = useState<string | null>(null);
  const [renamingCategoryName, setRenamingCategoryName] = useState('');
  const [draft, setDraft] = useState<BookmarkDraft>(emptyDraft());
  const [contextMenu, setContextMenu] = useState<ContextMenuState | null>(null);
  const [transferMessage, setTransferMessage] = useState<string | null>(null);
  const cardRef = useRef<HTMLDivElement | null>(null);
  const { position, size, dragHandleProps, resizeHandleProps } = usePanelDrag(cardRef);

  // Keep the selection valid when categories change (e.g. deletion).
  useEffect(() => {
    if (!selectedCategoryId) {
      if (categories.length > 0) {
        setSelectedCategoryId(categories[0].id);
      }
      return;
    }
    if (!categories.some((category) => category.id === selectedCategoryId)) {
      setSelectedCategoryId(categories[0]?.id ?? null);
    }
  }, [categories, selectedCategoryId]);

  // Reset transient editing state whenever the overlay opens.
  useEffect(() => {
    if (open) {
      setDraft(emptyDraft());
      setRenamingCategoryId(null);
      setCategoryDraft('');
      setContextMenu(null);
      setTransferMessage(null);
    }
  }, [open]);

  useEffect(() => {
    if (!open || !contextMenu) {
      return;
    }
    const closeContextMenu = () => setContextMenu(null);
    window.addEventListener('click', closeContextMenu);
    return () => window.removeEventListener('click', closeContextMenu);
  }, [open, contextMenu]);

  useEffect(() => {
    if (!open) {
      return;
    }
    const handleKeyDown = (event: KeyboardEvent): void => {
      if (event.key === 'Escape') {
        event.preventDefault();
        onClose();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [open, onClose]);

  // Maps a stored filter search text back to its preset's display label.
  const filterLabelFor = useCallback(
    (filterSearch: string): string | undefined =>
      filterOptions.find(
        (option) => option.search === filterSearch || (option.search ?? '') === filterSearch,
      )?.label,
    [filterOptions],
  );

  const categoryBookmarks = useMemo(
    () =>
      selectedCategoryId
        ? bookmarks.filter((bookmark) => bookmark.categoryId === selectedCategoryId)
        : [],
    [bookmarks, selectedCategoryId],
  );

  const handleExport = useCallback(async () => {
    try {
      const saved = await exportTextFile({
        defaultPath: 'cardinal-bookmarks.csv',
        contents: exportCsv(),
        filterName: t('bookmarks.csvFileType'),
      });
      setTransferMessage(saved ? t('bookmarks.exported') : null);
    } catch (error) {
      console.error('Failed to export bookmarks', error);
      setTransferMessage(t('bookmarks.ioFailed'));
    }
  }, [exportCsv, t]);

  const handleImport = useCallback(async () => {
    try {
      const csv = await importTextFile(t('bookmarks.csvFileType'));
      if (csv == null) {
        return;
      }
      const result = importCsv(csv);
      setTransferMessage(
        result.bookmarks > 0
          ? t('bookmarks.imported', { count: result.bookmarks })
          : t('bookmarks.ioFailed'),
      );
    } catch (error) {
      console.error('Failed to import bookmarks', error);
      setTransferMessage(t('bookmarks.ioFailed'));
    }
  }, [importCsv, t]);

  const commitCategory = useCallback(() => {
    const name = categoryDraft.trim();
    if (!name) {
      return;
    }
    addCategory(name);
    setCategoryDraft('');
  }, [addCategory, categoryDraft]);

  const startRenameCategory = useCallback((categoryId: string, currentName: string) => {
    setRenamingCategoryId(categoryId);
    setRenamingCategoryName(currentName);
  }, []);

  const commitRenameCategory = useCallback(() => {
    if (renamingCategoryId == null) {
      return;
    }
    const name = renamingCategoryName.trim();
    if (name) {
      renameCategory(renamingCategoryId, name);
    }
    setRenamingCategoryId(null);
    setRenamingCategoryName('');
  }, [renamingCategoryId, renamingCategoryName, renameCategory]);

  const startEditBookmark = useCallback((bookmark: Bookmark) => {
    setDraft({
      id: bookmark.id,
      name: bookmark.name,
      query: bookmark.query,
      filterSearch: bookmark.filterSearch,
    });
  }, []);

  const commitBookmark = useCallback(() => {
    if (!selectedCategoryId) {
      return;
    }
    const input = {
      name: draft.name,
      query: draft.query,
      filterSearch: draft.filterSearch,
    };
    if (draft.id == null) {
      addBookmark(selectedCategoryId, input);
    } else {
      updateBookmark(draft.id, input);
    }
    setDraft(emptyDraft());
  }, [addBookmark, draft, selectedCategoryId, updateBookmark]);

  if (!open) {
    return null;
  }

  const openBookmark = (bookmark: Bookmark, mode: BookmarkOpenMode) => {
    // The panel stays open in both modes: it is a floating, non-modal panel, so nothing is
    // hidden by leaving it up, and several bookmarks can be launched in a row.
    onOpenBookmark(bookmark, mode);
  };

  const isWindow = variant === 'window';

  return (
    // In `overlay` mode this is a non-modal floating panel inside the app. In `window` mode it
    // fills its own OS window, which can be dragged anywhere on screen.
    <div
      className={`bookmarks-overlay${isWindow ? ' bookmarks-overlay--window' : ''}`}
      role="dialog"
      aria-label={t('bookmarks.title')}
    >
      <div
        ref={cardRef}
        className={`bookmarks-card${!isWindow && position ? ' is-floating' : ''}${
          !isWindow && size ? ' is-resized' : ''
        }${isWindow ? ' is-window' : ''}`}
        style={
          isWindow
            ? undefined
            : {
                ...(position
                  ? { left: position.x, top: position.y, right: 'auto', bottom: 'auto' }
                  : {}),
                ...(size ? { width: size.width, height: size.height } : {}),
              }
        }
      >
        {/* Eight handles: four edges plus four corners (a corner adjusts both edges).
            A native window is resized by the OS, so they are overlay-only. */}
        {isWindow
          ? null
          : RESIZE_HANDLES.map(({ name, labelKey, orientation }) => (
              <div
                key={name}
                className={`bookmarks-resize-handle bookmarks-resize-handle--${name}`}
                role="separator"
                aria-orientation={orientation}
                aria-label={t(labelKey)}
                {...resizeHandleProps(name)}
              />
            ))}
        <header
          className="bookmarks-card__header is-drag-handle"
          title={t('bookmarks.dragHint')}
          {...(isWindow
            ? {
                onMouseDown: (event: React.MouseEvent<HTMLElement>) => {
                  // Buttons in the header keep their own behaviour.
                  if ((event.target as HTMLElement).closest('button, input, select')) {
                    return;
                  }
                  event.preventDefault();
                  void startNativeWindowDrag();
                },
              }
            : dragHandleProps)}
        >
          <h1 className="bookmarks-card__title">{t('bookmarks.title')}</h1>
          <div className="bookmarks-card__actions">
            <button type="button" className="bookmarks-button" onClick={() => void handleImport()}>
              {t('bookmarks.import')}
            </button>
            <button type="button" className="bookmarks-button" onClick={() => void handleExport()}>
              {t('bookmarks.export')}
            </button>
            <button
              type="button"
              className="bookmarks-close"
              onClick={onClose}
              aria-label={t('bookmarks.close')}
            >
              ×
            </button>
          </div>
        </header>

        <div className="bookmarks-body">
          {/* Categories (left) */}
          <aside className="bookmarks-categories">
            <div className="bookmarks-categories__list">
              {categories.map((category) => (
                <div
                  key={category.id}
                  className={`bookmarks-category${
                    category.id === selectedCategoryId ? ' is-selected' : ''
                  }`}
                  onClick={() => setSelectedCategoryId(category.id)}
                >
                  {renamingCategoryId === category.id ? (
                    <input
                      className="bookmarks-category__rename-input"
                      value={renamingCategoryName}
                      autoFocus
                      onChange={(event) => setRenamingCategoryName(event.target.value)}
                      onKeyDown={(event) => {
                        if (event.key === 'Enter') {
                          commitRenameCategory();
                        } else if (event.key === 'Escape') {
                          setRenamingCategoryId(null);
                        }
                      }}
                      onBlur={commitRenameCategory}
                      onClick={(event) => event.stopPropagation()}
                    />
                  ) : (
                    <>
                      <span className="bookmarks-category__name">{category.name}</span>
                      <span className="bookmarks-category__count">
                        {bookmarks.filter((bookmark) => bookmark.categoryId === category.id).length}
                      </span>
                      {category.id === defaultCategoryId ? null : (
                        <>
                          <button
                            type="button"
                            className="bookmarks-category__action"
                            title={t('bookmarks.renameCategory')}
                            onClick={(event) => {
                              event.stopPropagation();
                              startRenameCategory(category.id, category.name);
                            }}
                          >
                            ✎
                          </button>
                          <button
                            type="button"
                            className="bookmarks-category__action"
                            title={t('bookmarks.deleteCategory')}
                            onClick={(event) => {
                              event.stopPropagation();
                              deleteCategory(category.id);
                            }}
                          >
                            ×
                          </button>
                        </>
                      )}
                    </>
                  )}
                </div>
              ))}
            </div>
            <div className="bookmarks-category-add">
              <input
                className="bookmarks-input"
                value={categoryDraft}
                placeholder={t('bookmarks.newCategory')}
                onChange={(event) => setCategoryDraft(event.target.value)}
                onKeyDown={(event) => {
                  if (event.key === 'Enter') {
                    commitCategory();
                  }
                }}
              />
              <button type="button" className="bookmarks-button" onClick={commitCategory}>
                +
              </button>
            </div>
          </aside>

          {/* Bookmarks (right) */}
          <section className="bookmarks-bookmarks">
            {selectedCategoryId ? (
              <>
                <div className="bookmarks-bookmarks__list">
                  {categoryBookmarks.map((bookmark, index) => (
                    <div key={bookmark.id} className="bookmarks-bookmark-row">
                      <button
                        type="button"
                        className="bookmarks-bookmark-name"
                        title={bookmark.query}
                        onDoubleClick={() => openBookmark(bookmark, 'new')}
                        onContextMenu={(event) => {
                          event.preventDefault();
                          event.stopPropagation();
                          setContextMenu({ bookmark, x: event.clientX, y: event.clientY });
                        }}
                        onClick={() => startEditBookmark(bookmark)}
                      >
                        {/* Wrapped so the cell keeps a single line height and can ellipsize
                            without relying on the button's own text rendering. */}
                        <span className="bookmarks-bookmark-cell-text">{bookmark.name}</span>
                      </button>
                      {/* The query behaves like the name: single click edits, double click
                          opens in a new tab, right click shows the search menu. */}
                      <button
                        type="button"
                        className="bookmarks-bookmark-query"
                        title={bookmark.query}
                        onDoubleClick={() => openBookmark(bookmark, 'new')}
                        onContextMenu={(event) => {
                          event.preventDefault();
                          event.stopPropagation();
                          setContextMenu({ bookmark, x: event.clientX, y: event.clientY });
                        }}
                        onClick={() => startEditBookmark(bookmark)}
                      >
                        <span className="bookmarks-bookmark-cell-text">{bookmark.query}</span>
                        {bookmark.filterSearch ? (
                          <span className="bookmarks-bookmark-filter">
                            {filterLabelFor(bookmark.filterSearch) ?? t('bookmarks.filterFallback')}
                          </span>
                        ) : null}
                      </button>
                      <div className="bookmarks-bookmark-tools">
                        <button
                          type="button"
                          title={t('bookmarks.moveUp')}
                          disabled={index === 0}
                          onClick={() => moveBookmark(bookmark.id, -1)}
                        >
                          ↑
                        </button>
                        <button
                          type="button"
                          title={t('bookmarks.moveDown')}
                          disabled={index === categoryBookmarks.length - 1}
                          onClick={() => moveBookmark(bookmark.id, 1)}
                        >
                          ↓
                        </button>
                        <button
                          type="button"
                          title={t('bookmarks.edit')}
                          onClick={() => startEditBookmark(bookmark)}
                        >
                          ✎
                        </button>
                        <button
                          type="button"
                          title={t('bookmarks.delete')}
                          onClick={() => deleteBookmark(bookmark.id)}
                        >
                          ×
                        </button>
                      </div>
                    </div>
                  ))}
                </div>

                <div className="bookmarks-bookmark-add">
                  <select
                    className="bookmarks-input bookmarks-filter-select"
                    aria-label={t('bookmarks.filterLabel')}
                    title={t('bookmarks.filterLabel')}
                    value={draft.filterSearch}
                    onChange={(event) =>
                      setDraft((prev) => ({ ...prev, filterSearch: event.target.value }))
                    }
                  >
                    <option value="">{t('bookmarks.filterNone')}</option>
                    {filterOptions
                      .filter((option) => option.search)
                      .map((option) => (
                        <option key={option.id} value={option.search}>
                          {option.label}
                        </option>
                      ))}
                    {draft.filterSearch &&
                    !filterOptions.some((option) => option.search === draft.filterSearch) ? (
                      // Keep a filter that no longer matches a preset visible instead of
                      // silently resetting it to "none".
                      <option value={draft.filterSearch}>
                        {filterLabelFor(draft.filterSearch) ?? t('bookmarks.filterFallback')}
                      </option>
                    ) : null}
                  </select>
                  <input
                    className="bookmarks-input"
                    value={draft.name}
                    placeholder={t('bookmarks.name')}
                    onChange={(event) =>
                      setDraft((prev) => ({ ...prev, name: event.target.value }))
                    }
                    onKeyDown={(event) => {
                      // Enter commits from any field in the editor, not only the query one.
                      if (event.key === 'Enter') {
                        commitBookmark();
                      }
                    }}
                  />
                  <input
                    className="bookmarks-input"
                    value={draft.query}
                    placeholder={t('bookmarks.query')}
                    onChange={(event) =>
                      setDraft((prev) => ({ ...prev, query: event.target.value }))
                    }
                    onKeyDown={(event) => {
                      if (event.key === 'Enter') {
                        commitBookmark();
                      }
                    }}
                  />
                  <button type="button" className="bookmarks-button" onClick={commitBookmark}>
                    {draft.id == null ? t('bookmarks.add') : t('bookmarks.save')}
                  </button>
                </div>
              </>
            ) : (
              <div className="bookmarks-empty">{t('bookmarks.empty')}</div>
            )}
          </section>
        </div>

        {transferMessage ? (
          <p className="bookmarks-status" role="status">
            {transferMessage}
          </p>
        ) : null}

        {contextMenu ? (
          <div
            className="bookmarks-context-menu"
            style={{ top: contextMenu.y, left: contextMenu.x }}
            role="menu"
            onClick={(event) => event.stopPropagation()}
          >
            <button
              type="button"
              role="menuitem"
              onClick={() => {
                openBookmark(contextMenu.bookmark, 'current');
                setContextMenu(null);
              }}
            >
              {t('bookmarks.searchCurrent')}
            </button>
            <button
              type="button"
              role="menuitem"
              onClick={() => {
                openBookmark(contextMenu.bookmark, 'new');
                setContextMenu(null);
              }}
            >
              {t('bookmarks.searchNew')}
            </button>
          </div>
        ) : null}
      </div>
    </div>
  );
}

export default BookmarksOverlay;
