import React from 'react';
import type { ChangeEvent, FocusEventHandler } from 'react';

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
}: SearchBarProps): React.JSX.Element {
  return (
    <div className="search-container">
      <div className="search-bar">
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
