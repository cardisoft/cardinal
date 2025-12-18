import React from 'react';
import type { ChangeEvent, FocusEventHandler } from 'react';
import { ViewMode } from '../constants';
import { QueryBuilder } from './QueryBuilder';

type SearchBarProps = {
  inputRef: React.RefObject<HTMLInputElement>;
  placeholder: string;
  value: string;
  onChange: (event: ChangeEvent<HTMLInputElement>) => void;
  onKeyDown?: (event: React.KeyboardEvent<HTMLInputElement>) => void;
  caseSensitive: boolean;
  onToggleCaseSensitive: (event: ChangeEvent<HTMLInputElement>) => void;
  caseSensitiveLabel: string;
  onFocus?: FocusEventHandler<HTMLInputElement>;
  onBlur?: FocusEventHandler<HTMLInputElement>;
  viewMode: ViewMode;
  onToggleViewMode: (mode: ViewMode) => void;
};

export function SearchBar({
  inputRef,
  placeholder,
  value,
  onChange,
  onKeyDown,
  caseSensitive,
  onToggleCaseSensitive,
  caseSensitiveLabel,
  onFocus,
  onBlur,
  viewMode,
  onToggleViewMode,
}: SearchBarProps): React.JSX.Element {
  const [isBuilderOpen, setIsBuilderOpen] = React.useState(false);

  const handleApplyQuery = React.useCallback(
    (query: string) => {
      // Create a synthetic event to trigger the main onChange handler
      // We replace the entire query with the built one
      const event = {
        target: { value: query },
      } as React.ChangeEvent<HTMLInputElement>;

      onChange(event);

      const input = inputRef.current;
      if (input) {
        requestAnimationFrame(() => {
          input.focus();
        });
      }
    },
    [inputRef, onChange],
  );

  return (
    <div className="search-container">
      <div className="search-bar">
        <div className="search-help-trigger-container">
          <button
            className={`search-help-trigger ${isBuilderOpen ? 'active' : ''}`}
            onClick={() => setIsBuilderOpen(!isBuilderOpen)}
            title="Advanced Search"
            aria-expanded={isBuilderOpen}
          >
            <svg
              width="18"
              height="18"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
            >
              <polygon points="22 3 2 3 10 12.46 10 19 14 21 14 12.46 22 3" />
            </svg>
          </button>
          {isBuilderOpen && (
            <QueryBuilder
              onApplyQuery={handleApplyQuery}
              onClose={() => setIsBuilderOpen(false)}
              initialQuery={value}
            />
          )}
        </div>
        <input
          id="search-input"
          ref={inputRef}
          value={value}
          onChange={onChange}
          onKeyDown={onKeyDown}
          placeholder={placeholder}
          spellCheck={false}
          autoCorrect="off"
          autoComplete="off"
          autoCapitalize="off"
          onFocus={onFocus}
          onBlur={onBlur}
        />
        <div className="search-options">
          <label className="search-option" title={caseSensitiveLabel}>
            <input
              type="checkbox"
              checked={caseSensitive}
              onChange={onToggleCaseSensitive}
              aria-label={caseSensitiveLabel}
            />
            <span className="search-option__display" aria-hidden="true">
              Aa
            </span>
            <span className="sr-only">{caseSensitiveLabel}</span>
          </label>
          <div className="view-mode-toggle">
            <button
              className={`view-mode-button ${viewMode === 'list' ? 'active' : ''}`}
              onClick={() => onToggleViewMode('list')}
              title="List View"
            >
              <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
                <path d="M2 3h12v2H2V3zm0 4h12v2H2V7zm0 4h12v2H2v-2z" />
              </svg>
            </button>
            <button
              className={`view-mode-button ${viewMode === 'grid' ? 'active' : ''}`}
              onClick={() => onToggleViewMode('grid')}
              title="Grid View"
            >
              <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
                <path d="M3 3h4v4H3V3zm6 0h4v4H9V3zM3 9h4v4H3V9zm6 0h4v4H9V9z" />
              </svg>
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
