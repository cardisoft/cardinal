import React from 'react';
import type { SearchTab } from '../hooks/useSearchTabs';

type SearchTabsBarProps = {
  tabs: SearchTab[];
  activeTabId: string;
  onActivate: (id: string) => void;
  onClose: (id: string, event: React.MouseEvent) => void;
  onAdd: () => void;
};

export function SearchTabsBar({
  tabs,
  activeTabId,
  onActivate,
  onClose,
  onAdd,
}: SearchTabsBarProps): React.JSX.Element {
  return (
    <div className="search-tabs" role="tablist" aria-label="Search tabs">
      {tabs.map((tab) => {
        const isActive = tab.id === activeTabId;
        const label = tab.query.trim() || 'New tab';
        return (
          <div
            key={tab.id}
            role="tab"
            aria-selected={isActive}
            className={`search-tab${isActive ? ' is-active' : ''}`}
            onClick={() => onActivate(tab.id)}
            title={tab.query}
          >
            <span className="search-tab__label">{label}</span>
            <button
              type="button"
              className="search-tab__close"
              aria-label="Close tab"
              onClick={(event) => onClose(tab.id, event)}
            >
              ×
            </button>
          </div>
        );
      })}
      <button
        type="button"
        className="search-tab search-tab--add"
        aria-label="New tab"
        onClick={onAdd}
      >
        +
      </button>
    </div>
  );
}

export default SearchTabsBar;
