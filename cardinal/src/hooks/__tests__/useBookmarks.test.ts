import { act, renderHook } from '@testing-library/react';
import { beforeEach, describe, expect, it } from 'vitest';
import { useBookmarks } from '../useBookmarks';

const renderBookmarks = () => renderHook(() => useBookmarks());

describe('useBookmarks', () => {
  beforeEach(() => {
    window.localStorage.clear();
  });

  it('adds a bookmark when only the search text is filled in', () => {
    const { result } = renderBookmarks();

    act(() => {
      result.current.addCategory('a');
    });
    const categoryId = result.current.categories[0].id;

    act(() => {
      // The dialog submits whatever the inputs hold; an empty name used to silently
      // discard the bookmark, which looked like the Add button doing nothing.
      result.current.addBookmark(categoryId, { name: '', query: 'aa' });
    });

    expect(result.current.bookmarks).toHaveLength(1);
    expect(result.current.bookmarks[0].name).toBe('aa');
    expect(result.current.bookmarks[0].query).toBe('aa');
  });

  it('keeps an explicit display name and query apart', () => {
    const { result } = renderBookmarks();

    act(() => {
      result.current.addCategory('a');
    });
    const categoryId = result.current.categories[0].id;

    act(() => {
      result.current.addBookmark(categoryId, { name: '周星驰', query: '周星驰推荐' });
    });

    expect(result.current.bookmarks[0]).toMatchObject({
      name: '周星驰',
      query: '周星驰推荐',
    });
  });

  it('still ignores a completely empty draft', () => {
    const { result } = renderBookmarks();

    act(() => {
      result.current.addCategory('a');
    });
    const categoryId = result.current.categories[0].id;

    act(() => {
      result.current.addBookmark(categoryId, { name: '   ', query: '  ' });
    });

    expect(result.current.bookmarks).toHaveLength(0);
  });

  it('reorders only within the same category', () => {
    const { result } = renderBookmarks();

    act(() => {
      result.current.addCategory('a');
      result.current.addCategory('b');
    });
    const first = result.current.categories.find((category) => category.name === 'a')!;
    const second = result.current.categories.find((category) => category.name === 'b')!;

    act(() => {
      result.current.addBookmark(first.id, { name: 'a1', query: 'a1' });
      result.current.addBookmark(first.id, { name: 'a2', query: 'a2' });
      result.current.addBookmark(second.id, { name: 'b1', query: 'b1' });
    });

    const byName = (name: string) =>
      result.current.bookmarks.find((bookmark) => bookmark.name === name);

    act(() => {
      result.current.moveBookmark(byName('a2')!.id, -1);
    });

    expect(byName('a2')!.name).toBe('a2');
    expect(result.current.bookmarks.map((bookmark) => bookmark.name)).toEqual(['a2', 'a1', 'b1']);
    expect(result.current.bookmarks.find((bookmark) => bookmark.name === 'b1')!.categoryId).toBe(
      second.id,
    );
  });

  it('drops bookmarks together with their category', () => {
    const { result } = renderBookmarks();

    act(() => {
      result.current.addCategory('a');
    });
    const categoryId = result.current.categories.find((category) => category.name === 'a')!.id;

    act(() => {
      result.current.addBookmark(categoryId, { name: 'a1', query: 'a1' });
    });
    expect(result.current.bookmarks).toHaveLength(1);

    act(() => {
      result.current.deleteCategory(categoryId);
    });

    // The built-in default category survives; only the user category is removed.
    expect(result.current.categories.map((category) => category.id)).toEqual([
      result.current.defaultCategoryId,
    ]);
    expect(result.current.bookmarks).toHaveLength(0);
  });

  it('never deletes or renames the default category', () => {
    const { result } = renderBookmarks();
    const defaultId = result.current.defaultCategoryId;

    act(() => {
      result.current.deleteCategory(defaultId);
      result.current.renameCategory(defaultId, 'Renamed');
    });

    const defaultCategory = result.current.categories.find((category) => category.id === defaultId);
    expect(defaultCategory).toBeDefined();
    expect(defaultCategory?.name).not.toBe('Renamed');
  });
});

describe('useBookmarks CSV', () => {
  beforeEach(() => {
    window.localStorage.clear();
  });

  it('exports category/name/query rows and reimports them grouped', () => {
    const { result } = renderBookmarks();

    act(() => {
      result.current.addCategory('影视');
    });
    const categoryId = result.current.categories.find((category) => category.name === '影视')!.id;
    act(() => {
      result.current.addBookmark(categoryId, { name: '周星驰', query: '周星驰推荐' });
      result.current.addBookmark(categoryId, { name: '洪金宝', query: '洪金宝推荐' });
    });

    const csv = result.current.exportCsv();
    expect(csv.split(/\r?\n/)[0]).toBe('category,name,query,filter');

    // Import into a fresh store: the category is recreated and both bookmarks land in it.
    window.localStorage.clear();
    const second = renderBookmarks();
    act(() => {
      second.result.current.importCsv(csv);
    });

    const categories = second.result.current.categories;
    const drama = categories.find((category) => category.name === '影视');
    expect(drama).toBeDefined();
    const imported = second.result.current.bookmarks;
    expect(imported.map((bookmark) => bookmark.name)).toEqual(['周星驰', '洪金宝']);
    expect(imported.every((bookmark) => bookmark.categoryId === drama!.id)).toBe(true);
    expect(imported[0].query).toBe('周星驰推荐');
  });

  it('ignores headers and blank rows while importing', () => {
    const { result } = renderBookmarks();

    act(() => {
      // Header row + blank row are skipped; a row without a category lands in the default one.
      const outcome = result.current.importCsv(
        'category,name,query,filter\r\n,,,\r\n,OnlyName,,ext:md\r\n',
      );
      expect(outcome.bookmarks).toBe(1);
    });

    expect(result.current.bookmarks).toHaveLength(1);
    expect(result.current.bookmarks[0].categoryId).toBe(result.current.defaultCategoryId);
    expect(result.current.bookmarks[0].filterSearch).toBe('ext:md');
  });
});

describe('useBookmarks filters and CSV categories', () => {
  beforeEach(() => {
    window.localStorage.clear();
  });

  it('stores a filter alongside a bookmark and updates it later', () => {
    const { result } = renderBookmarks();
    const categoryId = result.current.defaultCategoryId;

    act(() => {
      result.current.addBookmark(categoryId, {
        name: '周星驰',
        query: '周星驰推荐',
        filterSearch: 'ext:mp4;mkv',
      });
    });
    expect(result.current.bookmarks[0].filterSearch).toBe('ext:mp4;mkv');

    // The bookmark editor can switch a saved filter back to "no filter".
    act(() => {
      result.current.updateBookmark(result.current.bookmarks[0].id, {
        name: '周星驰',
        query: '周星驰推荐',
        filterSearch: '',
      });
    });
    expect(result.current.bookmarks[0].filterSearch).toBe('');
  });

  it('exports the default category with an empty category cell', () => {
    const { result } = renderBookmarks();

    act(() => {
      result.current.addBookmark(result.current.defaultCategoryId, {
        name: '工作文件',
        query: '工作文件',
        filterSearch: 'folder:',
      });
    });

    const rows = result.current.exportCsv().split(/\r?\n/).filter(Boolean);
    expect(rows[1]).toBe(',工作文件,工作文件,folder:');
  });

  it('imports an empty category row into the default category', () => {
    const { result } = renderBookmarks();

    act(() => {
      result.current.importCsv('category,name,query,filter\n,工作文件,工作文件,folder:\n');
    });

    expect(result.current.bookmarks[0].categoryId).toBe(result.current.defaultCategoryId);
    expect(result.current.bookmarks[0].filterSearch).toBe('folder:');
  });

  it('creates a category from a row that has a category but no bookmark', () => {
    const { result } = renderBookmarks();

    let outcome = { categories: 0, bookmarks: 0 };
    act(() => {
      outcome = result.current.importCsv('category,name,query,filter\n空分类,,,\n');
    });

    expect(outcome).toEqual({ categories: 1, bookmarks: 0 });
    expect(result.current.categories.map((category) => category.name)).toContain('空分类');
  });

  it('round-trips the filter column through CSV', () => {
    const { result } = renderBookmarks();

    act(() => {
      result.current.addCategory('电影');
    });
    const categoryId = result.current.categories.find((c) => c.name === '电影')!.id;
    act(() => {
      result.current.addBookmark(categoryId, {
        name: '周星驰',
        query: '周星驰',
        filterSearch: 'ext:mp4',
      });
    });

    const csv = result.current.exportCsv();
    window.localStorage.clear();
    const second = renderBookmarks();
    act(() => {
      second.result.current.importCsv(csv);
    });

    expect(second.result.current.bookmarks[0].filterSearch).toBe('ext:mp4');
  });

  it('exports a category that has no bookmarks', () => {
    const { result } = renderBookmarks();

    act(() => {
      result.current.addCategory('空分类');
    });

    const rows = result.current.exportCsv().split(/\r?\n/).filter(Boolean);
    expect(rows).toContain('空分类,,,');
  });

  it('round-trips an empty category through export and import', () => {
    const { result } = renderBookmarks();

    act(() => {
      result.current.addCategory('空分类');
    });
    const csv = result.current.exportCsv();

    window.localStorage.clear();
    const second = renderBookmarks();
    act(() => {
      second.result.current.importCsv(csv);
    });

    expect(second.result.current.categories.map((category) => category.name)).toContain('空分类');
  });
});
