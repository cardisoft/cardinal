import React, { useEffect, useRef } from 'react';
import type { ChangeEvent, FocusEventHandler } from 'react';

type SearchBarProps = {
  inputRef: React.Ref<HTMLInputElement>;
  placeholder: string;
  value: string;
  onChange: (event: ChangeEvent<HTMLInputElement>) => void;
  onKeyDown: (event: React.KeyboardEvent<HTMLInputElement>) => void;
  directoryScopeEnabled: boolean;
  directoryScopeOpen: boolean;
  directoryScopeLabel: string;
  directoryPlaceholder: string;
  directoryValue: string;
  onToggleDirectoryScope: () => void;
  onDirectoryChange: (event: ChangeEvent<HTMLInputElement>) => void;
  onDirectoryKeyDown: (event: React.KeyboardEvent<HTMLInputElement>) => void;
  caseSensitive: boolean;
  onToggleCaseSensitive: (event: ChangeEvent<HTMLInputElement>) => void;
  caseSensitiveLabel: string;
  onFocus: FocusEventHandler<HTMLInputElement>;
  onBlur: FocusEventHandler<HTMLInputElement>;
};

export function SearchBar({
  inputRef,
  placeholder,
  value,
  onChange,
  onKeyDown,
  directoryScopeEnabled,
  directoryScopeOpen,
  directoryScopeLabel,
  directoryPlaceholder,
  directoryValue,
  onToggleDirectoryScope,
  onDirectoryChange,
  onDirectoryKeyDown,
  caseSensitive,
  onToggleCaseSensitive,
  caseSensitiveLabel,
  onFocus,
  onBlur,
}: SearchBarProps): React.JSX.Element {
  const directoryInputRef = useRef<HTMLInputElement | null>(null);

  useEffect(() => {
    if (directoryScopeOpen) {
      directoryInputRef.current?.focus();
    }
  }, [directoryScopeOpen]);

  return (
    <div className="search-container">
      <div className="search-bar">
        {directoryScopeEnabled ? (
          <>
            <button
              type="button"
              className="directory-scope-toggle"
              aria-label={directoryScopeLabel}
              aria-pressed={directoryScopeOpen || directoryValue.trim().length > 0}
              title={directoryScopeLabel}
              onClick={onToggleDirectoryScope}
            >
              <span aria-hidden="true">📁</span>
            </button>
            <div
              className={`directory-scope-field${directoryScopeOpen ? ' is-open' : ''}`}
              aria-hidden={!directoryScopeOpen}
            >
              <input
                ref={directoryInputRef}
                id="directory-scope-input"
                value={directoryValue}
                onChange={onDirectoryChange}
                onKeyDown={onDirectoryKeyDown}
                placeholder={directoryPlaceholder}
                spellCheck={false}
                autoCorrect="off"
                autoComplete="off"
                autoCapitalize="off"
                aria-label={directoryScopeLabel}
                disabled={!directoryScopeOpen}
                onFocus={onFocus}
                onBlur={onBlur}
              />
            </div>
          </>
        ) : null}
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
        </div>
      </div>
    </div>
  );
}
