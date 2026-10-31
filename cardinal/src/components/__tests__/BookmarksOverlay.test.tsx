import React from 'react';
import { fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { BookmarksOverlay } from '../BookmarksOverlay';
import type { BookmarksController } from '../../hooks/useBookmarks';
import type { Bookmark } from '../../types/bookmarks';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

vi.mock('../../utils/fileTransfer', () => ({
  exportTextFile: vi.fn(),
  importTextFile: vi.fn(),
}));

const buildController = (overrides: Partial<BookmarksController> = {}): BookmarksController => ({
  categories: [{ id: 'default', name: 'Default' }],
  bookmarks: [],
  defaultCategoryId: 'default',
  exportCsv: () => '',
  importCsv: () => ({ categories: 0, bookmarks: 0 }),
  addCategory: vi.fn(() => 'new-category'),
  renameCategory: vi.fn(),
  deleteCategory: vi.fn(),
  addBookmark: vi.fn(),
  updateBookmark: vi.fn(),
  deleteBookmark: vi.fn(),
  moveBookmark: vi.fn(),
  ...overrides,
});

const renderOverlay = (controller: BookmarksController) =>
  render(
    <BookmarksOverlay
      open
      onClose={vi.fn()}
      controller={controller}
      onOpenBookmark={vi.fn()}
      filterOptions={[
        { id: 'f1', label: 'Videos', search: 'ext:mp4' },
        { id: 'f2', label: 'Folders', search: 'folder:' },
      ]}
    />,
  );

describe('BookmarksOverlay editor', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('submits from the name field on Enter, not only from the query field', () => {
    const controller = buildController();
    renderOverlay(controller);

    fireEvent.change(screen.getByPlaceholderText('bookmarks.name'), {
      target: { value: '周星驰' },
    });
    fireEvent.change(screen.getByPlaceholderText('bookmarks.query'), {
      target: { value: '周星驰推荐' },
    });
    fireEvent.keyDown(screen.getByPlaceholderText('bookmarks.name'), { key: 'Enter' });

    expect(controller.addBookmark).toHaveBeenCalledWith('default', {
      name: '周星驰',
      query: '周星驰推荐',
      filterSearch: '',
    });
  });

  it('carries the selected filter into the new bookmark', () => {
    const controller = buildController();
    renderOverlay(controller);

    const filterSelect = screen.getByLabelText('bookmarks.filterLabel');
    fireEvent.change(filterSelect, { target: { value: 'ext:mp4' } });
    fireEvent.change(screen.getByPlaceholderText('bookmarks.name'), { target: { value: '电影' } });
    fireEvent.change(screen.getByPlaceholderText('bookmarks.query'), { target: { value: '电影' } });
    fireEvent.keyDown(screen.getByPlaceholderText('bookmarks.query'), { key: 'Enter' });

    expect(controller.addBookmark).toHaveBeenCalledWith('default', {
      name: '电影',
      query: '电影',
      filterSearch: 'ext:mp4',
    });
  });

  it('keeps the panel open when a bookmark opens in a new tab', () => {
    const onOpenBookmark = vi.fn();
    const onClose = vi.fn();
    const bookmark: Bookmark = {
      id: 'b1',
      categoryId: 'default',
      name: 'a',
      query: 'a',
      filterSearch: '',
    };
    render(
      <BookmarksOverlay
        open
        onClose={onClose}
        controller={buildController({ bookmarks: [bookmark] })}
        onOpenBookmark={onOpenBookmark}
        filterOptions={[]}
      />,
    );

    // Both the name and query columns render "a"; target the name column explicitly.
    const nameButton = document.querySelector('.bookmarks-bookmark-name') as HTMLElement;
    fireEvent.contextMenu(nameButton);
    fireEvent.click(screen.getByText('bookmarks.searchNew'));

    expect(onOpenBookmark).toHaveBeenCalledWith(bookmark, 'new');
    // Several bookmarks can be launched in a row without reopening the panel.
    expect(onClose).not.toHaveBeenCalled();
  });

  it('keeps the panel open when replacing the current tab search', () => {
    const onOpenBookmark = vi.fn();
    const onClose = vi.fn();
    const bookmark: Bookmark = {
      id: 'b1',
      categoryId: 'default',
      name: 'a',
      query: 'a',
      filterSearch: '',
    };
    render(
      <BookmarksOverlay
        open
        onClose={onClose}
        controller={buildController({ bookmarks: [bookmark] })}
        onOpenBookmark={onOpenBookmark}
        filterOptions={[]}
      />,
    );

    // Both the name and query columns render "a"; target the name column explicitly.
    const nameButton = document.querySelector('.bookmarks-bookmark-name') as HTMLElement;
    fireEvent.contextMenu(nameButton);
    fireEvent.click(screen.getByText('bookmarks.searchCurrent'));

    expect(onOpenBookmark).toHaveBeenCalledWith(bookmark, 'current');
    // The panel is a floating, non-modal window: it stays up so several bookmarks can be
    // launched in a row without reopening it.
    expect(onClose).not.toHaveBeenCalled();
  });

  it('lets the query column be clicked, double-clicked and right-clicked like the name', () => {
    const onOpenBookmark = vi.fn();
    const controller = buildController({
      bookmarks: [
        {
          id: 'b1',
          categoryId: 'default',
          name: '周星驰',
          query: '周星驰推荐',
          filterSearch: '',
        },
      ],
    });
    render(
      <BookmarksOverlay
        open
        onClose={vi.fn()}
        controller={controller}
        onOpenBookmark={onOpenBookmark}
        filterOptions={[]}
      />,
    );

    const queryButton = screen.getByRole('button', { name: /周星驰推荐/ });

    // Single click loads the row into the editor.
    fireEvent.click(queryButton);
    expect(screen.getByDisplayValue('周星驰推荐')).toBeInTheDocument();

    // Double click opens in a new tab.
    fireEvent.doubleClick(queryButton);
    expect(onOpenBookmark).toHaveBeenCalledWith(expect.objectContaining({ id: 'b1' }), 'new');

    // Right click shows the search mode menu.
    fireEvent.contextMenu(queryButton);
    expect(screen.getByText('bookmarks.searchCurrent')).toBeInTheDocument();
    expect(screen.getByText('bookmarks.searchNew')).toBeInTheDocument();
  });
});
