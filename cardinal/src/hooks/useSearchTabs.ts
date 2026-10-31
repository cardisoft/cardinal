import { useCallback, useMemo, useState } from 'react';

export type SearchTab = {
  id: string;
  query: string;
  // Filter fragment (e.g. `ext:mp4;mkv`) owned by this tab, so switching tabs restores the
  // filter as well as the text query.
  filterQuery: string;
  // Folder scope owned by this tab. Keeping it here (instead of in the shared search params)
  // is what stops a scope typed in one tab from narrowing every other tab.
  directoryQuery: string;
};

let tabSequence = 0;
const nextTabId = (): string => {
  tabSequence += 1;
  return `tab-${tabSequence.toString(36)}-${Date.now().toString(36)}`;
};

export type SearchTabsController = {
  tabs: SearchTab[];
  activeTabId: string;
  activeTab: SearchTab;
  createTab: (query?: string, filterQuery?: string, directoryQuery?: string) => string;
  closeTab: (id: string) => void;
  activateTab: (id: string) => void;
  setActiveTabQuery: (query: string) => void;
  setActiveTabFilter: (filterQuery: string) => void;
  setActiveTabDirectory: (directoryQuery: string) => void;
};

/**
 * Manages multiple search tabs. Each tab carries its own query and filter; the active tab's
 * values feed the search bar. Search execution is orchestrated by the caller
 * (App reacts to `activeTabId` changes).
 */
export function useSearchTabs(): SearchTabsController {
  const emptyTab = (): SearchTab => ({
    id: nextTabId(),
    query: '',
    filterQuery: '',
    directoryQuery: '',
  });
  const [initialTab] = useState<SearchTab>(emptyTab);
  const [tabs, setTabs] = useState<SearchTab[]>([initialTab]);
  const [activeTabId, setActiveTabId] = useState<string>(initialTab.id);

  const createTab = useCallback((query = '', filterQuery = '', directoryQuery = ''): string => {
    const id = nextTabId();
    setTabs((prev) => [...prev, { id, query, filterQuery, directoryQuery }]);
    setActiveTabId(id);
    return id;
  }, []);

  const closeTab = useCallback(
    (id: string) => {
      const index = tabs.findIndex((tab) => tab.id === id);
      if (index < 0) {
        return;
      }
      const remaining = tabs.filter((tab) => tab.id !== id);
      if (remaining.length === 0) {
        const fresh = emptyTab();
        setTabs([fresh]);
        setActiveTabId(fresh.id);
        return;
      }
      setTabs(remaining);
      if (id === activeTabId) {
        const nextActive = remaining[Math.min(index, remaining.length - 1)];
        setActiveTabId(nextActive.id);
      }
    },
    [activeTabId, tabs],
  );

  const activateTab = useCallback((id: string) => {
    setActiveTabId(id);
  }, []);

  const setActiveTabQuery = useCallback(
    (query: string) => {
      setTabs((prev) => prev.map((tab) => (tab.id === activeTabId ? { ...tab, query } : tab)));
    },
    [activeTabId],
  );

  const setActiveTabFilter = useCallback(
    (filterQuery: string) => {
      setTabs((prev) =>
        prev.map((tab) => (tab.id === activeTabId ? { ...tab, filterQuery } : tab)),
      );
    },
    [activeTabId],
  );

  const setActiveTabDirectory = useCallback(
    (directoryQuery: string) => {
      setTabs((prev) =>
        prev.map((tab) => (tab.id === activeTabId ? { ...tab, directoryQuery } : tab)),
      );
    },
    [activeTabId],
  );

  const activeTab = tabs.find((tab) => tab.id === activeTabId) ?? tabs[0];

  // Memoised so consumers can use this object in dependency arrays. Returning a fresh literal
  // every render made every callback that depends on it unstable, which caused listeners to be
  // re-registered on every render (see the bookmark-open handler in App).
  return useMemo(
    () => ({
      tabs,
      activeTabId,
      activeTab,
      createTab,
      closeTab,
      activateTab,
      setActiveTabQuery,
      setActiveTabFilter,
      setActiveTabDirectory,
    }),
    [
      tabs,
      activeTabId,
      activeTab,
      createTab,
      closeTab,
      activateTab,
      setActiveTabQuery,
      setActiveTabFilter,
      setActiveTabDirectory,
    ],
  );
}
