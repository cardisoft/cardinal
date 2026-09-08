import { act, renderHook } from '@testing-library/react';
import { beforeEach, describe, expect, it } from 'vitest';
import { useWorkspaceCollections } from '../useWorkspaceCollections';

describe('useWorkspaceCollections', () => {
  beforeEach(() => window.localStorage.clear());

  it('persists saved searches and restores them in a new hook instance', () => {
    const first = renderHook(() => useWorkspaceCollections());

    act(() => {
      first.result.current.saveSearch({
        name: 'Bugün değişenler',
        query: 'rapor',
        filterQuery: 'dm:today',
        directoryQuery: '/Users/example/Documents',
        directoryScopeOpen: true,
        caseSensitive: false,
      });
    });
    first.unmount();

    const second = renderHook(() => useWorkspaceCollections());
    expect(second.result.current.savedSearches).toEqual([
      expect.objectContaining({
        name: 'Bugün değişenler',
        query: 'rapor',
        filterQuery: 'dm:today',
      }),
    ]);
  });

  it('deduplicates favorite paths and can remove them', () => {
    const { result } = renderHook(() => useWorkspaceCollections());

    act(() => result.current.addFavorites(['/tmp/a', '/tmp/a', '/tmp/b']));
    expect(result.current.favorites.map((favorite) => favorite.path)).toEqual(['/tmp/a', '/tmp/b']);

    act(() => result.current.removeFavorite('/tmp/a'));
    expect(result.current.favorites.map((favorite) => favorite.path)).toEqual(['/tmp/b']);
  });

  it('drops malformed persisted entries instead of crashing the workspace panel', () => {
    window.localStorage.setItem(
      'cardinal.workspace.savedSearches',
      JSON.stringify([
        null,
        { id: 'bad' },
        {
          id: 'ok',
          name: 'Rapor',
          query: '',
          filterQuery: '',
          directoryQuery: '',
          directoryScopeOpen: false,
          caseSensitive: false,
          createdAt: 1,
        },
      ]),
    );
    window.localStorage.setItem(
      'cardinal.workspace.favorites',
      JSON.stringify([null, { path: 42 }, { path: '/tmp/ok', addedAt: 1 }]),
    );

    const { result } = renderHook(() => useWorkspaceCollections());
    expect(result.current.savedSearches.map((item) => item.id)).toEqual(['ok']);
    expect(result.current.favorites.map((item) => item.path)).toEqual(['/tmp/ok']);
  });
});
