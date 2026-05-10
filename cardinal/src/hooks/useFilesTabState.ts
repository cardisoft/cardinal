import { useCallback, useState } from 'react';
import type { ChangeEvent, KeyboardEvent as ReactKeyboardEvent } from 'react';
import type { StatusTabKey } from '../components/StatusBar';
import { hasModifierKey } from '../utils/keyboard';
import { useSearchHistory } from './useSearchHistory';

type QueueSearchOptions = {
  immediate?: boolean;
  onSearchCommitted?: () => void;
};

type UseFilesTabStateOptions = {
  searchQuery: string;
  directoryQuery: string;
  queueSearch: (query: string, options?: QueueSearchOptions) => void;
  queueDirectorySearch: (directoryQuery: string, options?: QueueSearchOptions) => void;
  maxSearchHistoryEntries?: number;
  onNavigateFromSearchToResults?: () => void;
};

type UseFilesTabStateResult = {
  activeTab: StatusTabKey;
  setActiveTab: (tab: StatusTabKey) => void;
  onTabChange: (tab: StatusTabKey) => void;
  isSearchFocused: boolean;
  handleSearchFocus: () => void;
  handleSearchBlur: () => void;
  eventFilterQuery: string;
  setEventFilterQuery: (value: string) => void;
  searchInputValue: string;
  directoryInputValue: string;
  onQueryChange: (event: ChangeEvent<HTMLInputElement>) => void;
  onDirectoryQueryChange: (event: ChangeEvent<HTMLInputElement>) => void;
  onDirectoryInputKeyDown: (event: ReactKeyboardEvent<HTMLInputElement>) => void;
  onSearchInputKeyDown: (event: ReactKeyboardEvent<HTMLInputElement>) => void;
  submitFilesQuery: (query: string, options?: { immediate?: boolean }) => void;
};

/**
 * Manages files/events tab UI state, including search input behavior and history navigation.
 */
export function useFilesTabState({
  searchQuery,
  directoryQuery,
  queueSearch,
  queueDirectorySearch,
  maxSearchHistoryEntries = 50,
  onNavigateFromSearchToResults,
}: UseFilesTabStateOptions): UseFilesTabStateResult {
  const [activeTab, setActiveTab] = useState<StatusTabKey>('files');
  const [isSearchFocused, setIsSearchFocused] = useState(false);
  const [eventFilterQuery, setEventFilterQuery] = useState('');
  const {
    handleInputChange: updateHistoryFromInput,
    navigate: navigateSearchHistory,
    ensureTailValue: ensureHistoryBuffer,
    resetCursorToTail,
  } = useSearchHistory({ maxEntries: maxSearchHistoryEntries });

  const handleSearchFocus = useCallback(() => {
    setIsSearchFocused(true);
  }, []);

  const handleSearchBlur = useCallback(() => {
    setIsSearchFocused(false);
  }, []);

  const submitFilesQuery = useCallback(
    (query: string, options?: { immediate?: boolean }) => {
      queueSearch(query, {
        immediate: options?.immediate,
        onSearchCommitted: () => updateHistoryFromInput(query),
      });
    },
    [queueSearch, updateHistoryFromInput],
  );

  const handleHistoryNavigation = useCallback(
    (direction: 'older' | 'newer') => {
      if (activeTab !== 'files') {
        return;
      }

      const nextValue = navigateSearchHistory(direction);
      if (nextValue === null) {
        if (direction === 'newer') {
          onNavigateFromSearchToResults?.();
        }
        return;
      }

      queueSearch(nextValue);
    },
    [activeTab, navigateSearchHistory, onNavigateFromSearchToResults, queueSearch],
  );

  const onSearchInputKeyDown = useCallback(
    (event: ReactKeyboardEvent<HTMLInputElement>) => {
      if (activeTab !== 'files') {
        return;
      }

      if (event.key === 'Enter') {
        submitFilesQuery(event.currentTarget.value, { immediate: true });
        return;
      }

      if (event.key !== 'ArrowUp' && event.key !== 'ArrowDown') {
        return;
      }

      if (hasModifierKey(event)) {
        return;
      }

      event.preventDefault();
      handleHistoryNavigation(event.key === 'ArrowUp' ? 'older' : 'newer');
    },
    [activeTab, handleHistoryNavigation, submitFilesQuery],
  );

  const onDirectoryInputKeyDown = useCallback(
    (event: ReactKeyboardEvent<HTMLInputElement>) => {
      if (activeTab !== 'files' || event.key !== 'Enter') {
        return;
      }

      queueDirectorySearch(event.currentTarget.value, { immediate: true });
    },
    [activeTab, queueDirectorySearch],
  );

  const onQueryChange = useCallback(
    (event: ChangeEvent<HTMLInputElement>) => {
      const inputValue = event.target.value;

      if (activeTab === 'events') {
        setEventFilterQuery(inputValue);
        return;
      }

      submitFilesQuery(inputValue);
    },
    [activeTab, setEventFilterQuery, submitFilesQuery],
  );

  const onDirectoryQueryChange = useCallback(
    (event: ChangeEvent<HTMLInputElement>) => {
      if (activeTab !== 'files') {
        return;
      }

      queueDirectorySearch(event.target.value);
    },
    [activeTab, queueDirectorySearch],
  );

  const onTabChange = useCallback(
    (nextTab: StatusTabKey) => {
      setActiveTab(nextTab);

      if (nextTab === 'events') {
        setEventFilterQuery('');
        resetCursorToTail();
        return;
      }

      ensureHistoryBuffer('');
      queueSearch('', { immediate: true });
    },
    [ensureHistoryBuffer, queueSearch, resetCursorToTail, setEventFilterQuery],
  );

  const searchInputValue = activeTab === 'events' ? eventFilterQuery : searchQuery;
  const directoryInputValue = activeTab === 'files' ? directoryQuery : '';

  return {
    activeTab,
    setActiveTab,
    onTabChange,
    isSearchFocused,
    handleSearchFocus,
    handleSearchBlur,
    eventFilterQuery,
    setEventFilterQuery,
    searchInputValue,
    directoryInputValue,
    onQueryChange,
    onDirectoryQueryChange,
    onDirectoryInputKeyDown,
    onSearchInputKeyDown,
    submitFilesQuery,
  };
}
