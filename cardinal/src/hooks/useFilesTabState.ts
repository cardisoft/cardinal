import { useCallback, useState } from 'react';
import type { StatusTabKey } from '../components/StatusBar';

type UseFilesTabStateResult = {
  activeTab: StatusTabKey;
  setActiveTab: (tab: StatusTabKey) => void;
  isSearchFocused: boolean;
  handleSearchFocus: () => void;
  handleSearchBlur: () => void;
};

export function useFilesTabState(): UseFilesTabStateResult {
  const [activeTab, setActiveTab] = useState<StatusTabKey>('files');
  const [isSearchFocused, setIsSearchFocused] = useState(false);

  const handleSearchFocus = useCallback(() => {
    setIsSearchFocused(true);
  }, []);

  const handleSearchBlur = useCallback(() => {
    setIsSearchFocused(false);
  }, []);

  return {
    activeTab,
    setActiveTab,
    isSearchFocused,
    handleSearchFocus,
    handleSearchBlur,
  };
}
