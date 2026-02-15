import { act, renderHook } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { useFilesTabState } from '../useFilesTabState';

describe('useFilesTabState', () => {
  it('tracks active tab and search focus state', () => {
    const { result } = renderHook(() => useFilesTabState());

    expect(result.current.activeTab).toBe('files');
    expect(result.current.isSearchFocused).toBe(false);

    act(() => {
      result.current.setActiveTab('events');
    });
    expect(result.current.activeTab).toBe('events');

    act(() => {
      result.current.handleSearchFocus();
    });
    expect(result.current.isSearchFocused).toBe(true);

    act(() => {
      result.current.handleSearchBlur();
    });
    expect(result.current.isSearchFocused).toBe(false);
  });
});
