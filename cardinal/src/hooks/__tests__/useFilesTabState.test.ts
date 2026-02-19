import { act, renderHook } from '@testing-library/react';
import type { ChangeEvent, KeyboardEvent as ReactKeyboardEvent } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useFilesTabState } from '../useFilesTabState';
import { useSearchHistory } from '../useSearchHistory';

vi.mock('../useSearchHistory', () => ({
  useSearchHistory: vi.fn(),
}));

const mockedUseSearchHistory = vi.mocked(useSearchHistory);

type HookProps = {
  searchQuery: string;
  queueSearch: (
    query: string,
    options?: {
      immediate?: boolean;
      onSearchCommitted?: (query: string) => void;
    },
  ) => void;
};

describe('useFilesTabState', () => {
  const queueSearch = vi.fn();
  const handleInputChange = vi.fn();
  const navigate = vi.fn();
  const ensureTailValue = vi.fn();
  const resetCursorToTail = vi.fn();

  const renderFilesTabState = (overrides: Partial<HookProps> = {}) =>
    renderHook((props: HookProps) => useFilesTabState(props), {
      initialProps: {
        searchQuery: 'needle',
        queueSearch,
        ...overrides,
      },
    });

  beforeEach(() => {
    vi.clearAllMocks();
    navigate.mockReturnValue(null);
    mockedUseSearchHistory.mockReturnValue({
      handleInputChange,
      navigate,
      ensureTailValue,
      resetCursorToTail,
    });
  });

  it('tracks active tab and search focus state', () => {
    const { result, rerender } = renderFilesTabState();

    expect(result.current.activeTab).toBe('files');
    expect(result.current.isSearchFocused).toBe(false);
    expect(result.current.searchInputValue).toBe('needle');

    act(() => {
      result.current.setActiveTab('events');
    });
    expect(result.current.activeTab).toBe('events');
    expect(result.current.searchInputValue).toBe('');

    act(() => {
      result.current.setEventFilterQuery('evt');
    });
    expect(result.current.searchInputValue).toBe('evt');

    rerender({
      searchQuery: 'needle-updated',
      queueSearch,
    });
    expect(result.current.searchInputValue).toBe('evt');

    act(() => {
      result.current.handleSearchFocus();
    });
    expect(result.current.isSearchFocused).toBe(true);

    act(() => {
      result.current.handleSearchBlur();
    });
    expect(result.current.isSearchFocused).toBe(false);
  });

  it('routes tab changes and performs tab-specific side effects', () => {
    const { result } = renderFilesTabState();

    act(() => {
      result.current.onTabChange('events');
    });
    expect(result.current.activeTab).toBe('events');
    expect(result.current.eventFilterQuery).toBe('');
    expect(resetCursorToTail).toHaveBeenCalledTimes(1);

    resetCursorToTail.mockClear();
    queueSearch.mockClear();
    ensureTailValue.mockClear();

    act(() => {
      result.current.onTabChange('files');
    });
    expect(result.current.activeTab).toBe('files');
    expect(ensureTailValue).toHaveBeenCalledWith('');
    expect(queueSearch).toHaveBeenCalledWith('', { immediate: true });
    expect(resetCursorToTail).not.toHaveBeenCalled();
  });

  it('submits files query with history callback and routes event filter input on events tab', () => {
    const { result } = renderFilesTabState();

    act(() => {
      result.current.onQueryChange({
        target: { value: 'abc' },
      } as ChangeEvent<HTMLInputElement>);
    });
    expect(queueSearch).toHaveBeenCalledWith('abc', {
      immediate: undefined,
      onSearchCommitted: handleInputChange,
    });

    queueSearch.mockClear();

    act(() => {
      result.current.onTabChange('events');
    });

    act(() => {
      result.current.onQueryChange({
        target: { value: 'event-path' },
      } as ChangeEvent<HTMLInputElement>);
    });
    expect(result.current.eventFilterQuery).toBe('event-path');
    expect(queueSearch).not.toHaveBeenCalled();
  });

  it('handles Enter and history navigation keys on files tab', () => {
    const { result } = renderFilesTabState();

    const enterPreventDefault = vi.fn();
    act(() => {
      result.current.onSearchInputKeyDown({
        key: 'Enter',
        currentTarget: { value: 'enter-query' },
        preventDefault: enterPreventDefault,
        altKey: false,
        ctrlKey: false,
        metaKey: false,
        shiftKey: false,
      } as unknown as ReactKeyboardEvent<HTMLInputElement>);
    });
    expect(queueSearch).toHaveBeenCalledWith('enter-query', {
      immediate: true,
      onSearchCommitted: handleInputChange,
    });
    expect(enterPreventDefault).not.toHaveBeenCalled();

    queueSearch.mockClear();
    navigate.mockReturnValueOnce('history-query');
    const arrowPreventDefault = vi.fn();
    act(() => {
      result.current.onSearchInputKeyDown({
        key: 'ArrowUp',
        currentTarget: { value: 'ignored' },
        preventDefault: arrowPreventDefault,
        altKey: false,
        ctrlKey: false,
        metaKey: false,
        shiftKey: false,
      } as unknown as ReactKeyboardEvent<HTMLInputElement>);
    });
    expect(arrowPreventDefault).toHaveBeenCalledTimes(1);
    expect(navigate).toHaveBeenCalledWith('older');
    expect(queueSearch).toHaveBeenCalledWith('history-query');
  });

  it('ignores history navigation when modifiers are pressed or no history entry exists', () => {
    const { result } = renderFilesTabState();
    const preventDefault = vi.fn();

    act(() => {
      result.current.onSearchInputKeyDown({
        key: 'ArrowDown',
        currentTarget: { value: '' },
        preventDefault,
        altKey: false,
        ctrlKey: false,
        metaKey: true,
        shiftKey: false,
      } as unknown as ReactKeyboardEvent<HTMLInputElement>);
    });
    expect(preventDefault).not.toHaveBeenCalled();
    expect(navigate).not.toHaveBeenCalled();

    act(() => {
      result.current.onSearchInputKeyDown({
        key: 'ArrowDown',
        currentTarget: { value: '' },
        preventDefault,
        altKey: false,
        ctrlKey: false,
        metaKey: false,
        shiftKey: false,
      } as unknown as ReactKeyboardEvent<HTMLInputElement>);
    });
    expect(preventDefault).toHaveBeenCalledTimes(1);
    expect(navigate).toHaveBeenCalledWith('newer');
    expect(queueSearch).not.toHaveBeenCalled();
  });

  it('exposes submitFilesQuery for external callers', () => {
    const { result } = renderFilesTabState();

    act(() => {
      result.current.submitFilesQuery('external-query', { immediate: true });
    });
    expect(queueSearch).toHaveBeenCalledWith('external-query', {
      immediate: true,
      onSearchCommitted: handleInputChange,
    });
  });
});
