import { act, renderHook } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { useSearchTabs } from '../useSearchTabs';

describe('useSearchTabs', () => {
  it('keeps each tab’s filter separate', () => {
    const { result } = renderHook(() => useSearchTabs());
    const firstId = result.current.tabs[0].id;

    act(() => {
      result.current.setActiveTabQuery('a');
      result.current.setActiveTabFilter('ext:bat;cmd');
    });

    let secondId = '';
    act(() => {
      secondId = result.current.createTab('a', 'ext:mp4');
    });

    // The new tab carries its own filter...
    expect(result.current.tabs.find((tab) => tab.id === secondId)).toMatchObject({
      query: 'a',
      filterQuery: 'ext:mp4',
    });
    // ...and the previously active tab is untouched. This is the bug where opening a second
    // bookmark rewrote the first tab's filter.
    expect(result.current.tabs.find((tab) => tab.id === firstId)).toMatchObject({
      query: 'a',
      filterQuery: 'ext:bat;cmd',
    });
  });

  it('keeps each tab’s folder scope separate', () => {
    // Regression: the scope lived in the shared search params, so a path typed in one tab
    // silently narrowed every other tab.
    const { result } = renderHook(() => useSearchTabs());
    const firstId = result.current.tabs[0].id;

    act(() => {
      result.current.setActiveTabDirectory('/Volumes/nfs_D');
    });
    expect(result.current.tabs.find((tab) => tab.id === firstId)?.directoryQuery).toBe(
      '/Volumes/nfs_D',
    );

    let secondId = '';
    act(() => {
      secondId = result.current.createTab('白咲碧', '', '');
    });
    // A brand new tab starts with no scope...
    expect(result.current.tabs.find((tab) => tab.id === secondId)?.directoryQuery).toBe('');
    // ...and the first tab keeps its own.
    expect(result.current.tabs.find((tab) => tab.id === firstId)?.directoryQuery).toBe(
      '/Volumes/nfs_D',
    );

    act(() => {
      result.current.setActiveTabDirectory('~/Desktop');
      result.current.activateTab(firstId);
    });
    expect(result.current.tabs.find((tab) => tab.id === firstId)?.directoryQuery).toBe(
      '/Volumes/nfs_D',
    );
    expect(result.current.tabs.find((tab) => tab.id === secondId)?.directoryQuery).toBe(
      '~/Desktop',
    );
  });

  it('only rewrites the active tab when its filter changes', () => {
    const { result } = renderHook(() => useSearchTabs());
    const firstId = result.current.tabs[0].id;

    act(() => {
      result.current.setActiveTabFilter('folder:');
    });

    expect(result.current.tabs.find((tab) => tab.id === firstId)?.filterQuery).toBe('folder:');
  });
});
