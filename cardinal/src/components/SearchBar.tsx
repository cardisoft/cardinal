import React, { useCallback, useEffect, useRef } from 'react';
import type { ChangeEvent, FocusEventHandler } from 'react';
import { hasModifierKey } from '../utils/keyboard';

type SearchBarProps = {
  inputRef: React.RefObject<HTMLInputElement>;
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

const isCollapsedAtStart = (input: HTMLInputElement): boolean =>
  input.selectionStart === 0 && input.selectionEnd === 0;

const isCollapsedAtEnd = (input: HTMLInputElement): boolean => {
  const end = input.value.length;
  return input.selectionStart === end && input.selectionEnd === end;
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

  const handleQueryKeyDown = useCallback(
    (event: React.KeyboardEvent<HTMLInputElement>) => {
      if (
        directoryScopeEnabled &&
        directoryScopeOpen &&
        event.key === 'ArrowLeft' &&
        !hasModifierKey(event) &&
        isCollapsedAtStart(event.currentTarget)
      ) {
        event.preventDefault();
        const input = directoryInputRef.current;
        input?.focus();
        const end = input?.value.length ?? 0;
        input?.setSelectionRange(end, end);
        return;
      }

      onKeyDown(event);
    },
    [directoryScopeEnabled, directoryScopeOpen, onKeyDown],
  );

  const handleDirectoryKeyDown = useCallback(
    (event: React.KeyboardEvent<HTMLInputElement>) => {
      if (
        event.key === 'ArrowRight' &&
        !hasModifierKey(event) &&
        isCollapsedAtEnd(event.currentTarget)
      ) {
        event.preventDefault();
        inputRef.current?.focus();
        inputRef.current?.setSelectionRange(0, 0);
        return;
      }

      onDirectoryKeyDown(event);
    },
    [inputRef, onDirectoryKeyDown],
  );

  return (
    <div className="search-container">
      <div className="search-bar">
        {directoryScopeEnabled ? (
          <>
            <button
              type="button"
              className="directory-scope-toggle"
              aria-label={directoryScopeLabel}
              aria-pressed={directoryScopeOpen}
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
                onKeyDown={handleDirectoryKeyDown}
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
          onKeyDown={handleQueryKeyDown}
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
