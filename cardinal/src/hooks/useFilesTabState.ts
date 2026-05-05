import { useCallback, useMemo, useState } from 'react';
import type { ChangeEvent, KeyboardEvent as ReactKeyboardEvent } from 'react';
import type { StatusTabKey } from '../components/StatusBar';
import type { ShortcutMap } from '../shortcuts';
import { shortcutMatchesKeydown } from '../utils/shortcutCapture';
import { useSearchHistory } from './useSearchHistory';

type QueueSearchOptions = {
  immediate?: boolean;
  onSearchCommitted?: (query: string) => void;
};

type UseFilesTabStateOptions = {
  searchQuery: string;
  queueSearch: (query: string, options?: QueueSearchOptions) => void;
  shortcuts: ShortcutMap;
  shortcutsEnabled: boolean;
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
  onQueryChange: (event: ChangeEvent<HTMLInputElement>) => void;
  onSearchInputKeyDown: (event: ReactKeyboardEvent<HTMLInputElement>) => void;
  submitFilesQuery: (query: string, options?: { immediate?: boolean }) => void;
};

/**
 * Manages files/events tab UI state, including search input behavior and history navigation.
 */
export function useFilesTabState({
  searchQuery,
  queueSearch,
  shortcuts,
  shortcutsEnabled,
}: UseFilesTabStateOptions): UseFilesTabStateResult {
  const [activeTab, setActiveTab] = useState<StatusTabKey>('files');
  const [isSearchFocused, setIsSearchFocused] = useState(false);
  const [eventFilterQuery, setEventFilterQuery] = useState('');
  const {
    handleInputChange: updateHistoryFromInput,
    navigate: navigateSearchHistory,
    ensureTailValue: ensureHistoryBuffer,
    resetCursorToTail,
  } = useSearchHistory();

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
        onSearchCommitted: updateHistoryFromInput,
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
        return;
      }

      queueSearch(nextValue);
    },
    [activeTab, navigateSearchHistory, queueSearch],
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

      if (!shortcutsEnabled) {
        return;
      }

      const isHistoryUp = shortcutMatchesKeydown(event, shortcuts.searchHistoryUp);
      const isHistoryDown = shortcutMatchesKeydown(event, shortcuts.searchHistoryDown);
      if (!isHistoryUp && !isHistoryDown) {
        return;
      }

      event.preventDefault();
      handleHistoryNavigation(isHistoryUp ? 'older' : 'newer');
    },
    [activeTab, handleHistoryNavigation, shortcuts, shortcutsEnabled, submitFilesQuery],
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

  const searchInputValue = useMemo(
    () => (activeTab === 'events' ? eventFilterQuery : searchQuery),
    [activeTab, eventFilterQuery, searchQuery],
  );

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
    onQueryChange,
    onSearchInputKeyDown,
    submitFilesQuery,
  };
}
