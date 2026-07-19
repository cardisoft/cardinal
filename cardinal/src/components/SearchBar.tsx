import React, { useCallback, useEffect, useRef } from 'react';
import type { ChangeEvent, FocusEventHandler } from 'react';
import { hasModifierKey } from '../utils/keyboard';

const HISTORY_ICON = (
  <svg viewBox="0 0 16 16" aria-hidden="true" focusable="false">
    <path
      d="M8 2.5a5.5 5.5 0 1 1-5.35 6.8"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      strokeLinecap="round"
    />
    <path d="M2.2 5.4V9l3.4-.9z" fill="currentColor" stroke="none" />
    <path
      d="M8 5.2V8l2.2 1.4"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      strokeLinecap="round"
      strokeLinejoin="round"
    />
  </svg>
);

const MACOS_FOLDER_ICON =
  'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAACAAAAAgCAYAAABzenr0AAAAAXNSR0IArs4c6QAAAERlWElmTU0AKgAAAAgAAYdpAAQAAAABAAAAGgAAAAAAA6ABAAMAAAABAAEAAKACAAQAAAABAAAAIKADAAQAAAABAAAAIAAAAACshmLzAAADGUlEQVRYCe1XsW4TQRCdPZ9tEsuxiEmTABIFSijpKKClooiEhNKAlJJIFBRUfAEVEgW/QEoKkCiQKEAUNDRE0EBQFKBACbIT49z57pb39rx3Z0exneSiUGSsOe/tzs68ndmZ3RM5oWP2gMrYd9Eud7mQ6e9vRujwutzpH9zvuwVA47W7r78/Gps4fdsRXRqgKPK9ztsn16YWIPMH7IP1APmBQxZA7c7L1celanWR0lpraLRD6XzFXqWkgDGtvW/NlU/3l5euf4BEmEoNbFHOei+gpLVy7uarX18dkaJWWiKshyD6icYddLsQdAuOFCDr4KfYOYwgEumo7W01l5/Nzz2AeAMc0PWk8a22V6SBvVYfixExjAJ2AQ/+GwBDIkC9Xe+NSbGyeGP5i/tiYe4edDYsgMJ2G6FUWNoIZFesdKx46BSoJVACJpDIcW9hzkNwAkC1PWxo2tejgRhqtE+AoC1gtJht4xSxHpC/vi9n6xNyabouk9US4mu3R5+mA76G2FObW758/rkh65vb1GJSPQFQr47LlYvTRn0A4c6o+3pEQIxurVI2Nt6srHOWWWECYHbmjHgB0g87e9hGHNFmj5gK4xRmGGZnJuV9dzQBMHaqJF4YLxuiGI6B9Gg54AuzgKSERVSEtiwlALBsAUhTaLSOBa3QYf9tTbFpntWXAAgiFCYESkX5br6sMeNVhFgyC0wARPS+sT1CVevVuv+3jIkEQIj6q1S+rt8LmQ0Jx3sAAMFec/Ltz5wzPQCO1QORSZGjKcO73ZeG+j/yQBggCwfdxHav46A92qRcPDvxgOnToSlER3UiosjEF500AmkWBPSAOd/hBQdFIe9jmSlOtXhoXrm6ZD0Qtdo7UnZdhAGCuG4pVqu89iRU8ZCTEB6Aq73AXAfRm9aBnUajuVafqJw3wFiSc64Jpvh0S3Cj2VqDnR3aspWnjvaFy0/fPa+UyjPFYl5Lp4mUOp1IWr734+PS1Xn0roI3LACejwQxBa6BmQ4cs+NoHooYdDJPHN6Gf4M3wH7WAEHYL6M8jUOtIQvCfhfwg+aE5B+lBx09YnlGKQAAAABJRU5ErkJggg==';

type SearchBarProps = {
  inputRef: React.RefObject<HTMLInputElement>;
  placeholder: string;
  ariaLabel: string;
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
  historyEnabled?: boolean;
  historyEntries?: string[];
  historyOpen?: boolean;
  onToggleHistory?: () => void;
  onSelectHistoryEntry?: (query: string) => void;
  onClearHistory?: () => void;
  historyLabel?: string;
  historyEmptyLabel?: string;
  historyClearLabel?: string;
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
  ariaLabel,
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
  historyEnabled = false,
  historyEntries = [],
  historyOpen = false,
  onToggleHistory,
  onSelectHistoryEntry,
  onClearHistory,
  historyLabel = 'Search history',
  historyEmptyLabel = 'No search history yet',
  historyClearLabel = 'Clear history',
}: SearchBarProps): React.JSX.Element {
  const directoryInputRef = useRef<HTMLInputElement | null>(null);
  const historyContainerRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    if (directoryScopeOpen) {
      directoryInputRef.current?.focus();
    }
  }, [directoryScopeOpen]);

  useEffect(() => {
    if (!historyOpen) {
      return;
    }

    const handlePointerDown = (event: MouseEvent) => {
      const container = historyContainerRef.current;
      if (container && event.target instanceof Node && !container.contains(event.target)) {
        onToggleHistory?.();
      }
    };
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        onToggleHistory?.();
      }
    };

    document.addEventListener('mousedown', handlePointerDown);
    document.addEventListener('keydown', handleKeyDown);
    return () => {
      document.removeEventListener('mousedown', handlePointerDown);
      document.removeEventListener('keydown', handleKeyDown);
    };
  }, [historyOpen, onToggleHistory]);

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
          <div className={`directory-scope-segment${directoryScopeOpen ? ' is-open' : ''}`}>
            {directoryScopeOpen ? null : (
              <button
                type="button"
                className="directory-scope-toggle"
                aria-label={directoryScopeLabel}
                aria-pressed="false"
                title={directoryScopeLabel}
                onClick={onToggleDirectoryScope}
              >
                <svg className="directory-scope-chevron" viewBox="0 0 16 16" aria-hidden="true">
                  <path d="M4 0L12 8l-8 8" />
                </svg>
              </button>
            )}
            <div
              className={`directory-scope-field${directoryScopeOpen ? ' is-open' : ''}`}
              aria-hidden={!directoryScopeOpen}
            >
              {directoryScopeOpen ? (
                <button
                  type="button"
                  className="directory-scope-field-toggle"
                  aria-label={directoryScopeLabel}
                  aria-pressed="true"
                  title={directoryScopeLabel}
                  onClick={onToggleDirectoryScope}
                >
                  <img src={MACOS_FOLDER_ICON} alt="" aria-hidden="true" />
                </button>
              ) : null}
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
          </div>
        ) : null}
        <div className="search-segment query-search-segment">
          <input
            id="search-input"
            ref={inputRef}
            value={value}
            onChange={onChange}
            onKeyDown={handleQueryKeyDown}
            placeholder={placeholder}
            aria-label={ariaLabel}
            spellCheck={false}
            autoCorrect="off"
            autoComplete="off"
            autoCapitalize="off"
            onFocus={onFocus}
            onBlur={onBlur}
          />
        </div>
        <div className="search-segment search-options">
          {historyEnabled ? (
            <div className="search-history" ref={historyContainerRef}>
              <button
                type="button"
                className="search-history-toggle"
                aria-label={historyLabel}
                aria-haspopup="true"
                aria-expanded={historyOpen}
                title={historyLabel}
                onClick={onToggleHistory}
              >
                {HISTORY_ICON}
              </button>
              {historyOpen ? (
                <div className="search-history-panel" aria-label={historyLabel}>
                  {historyEntries.length === 0 ? (
                    <div className="search-history-empty">{historyEmptyLabel}</div>
                  ) : (
                    <>
                      <div className="search-history-list">
                        {historyEntries.map((entry) => (
                          <button
                            type="button"
                            key={entry}
                            className="search-history-item"
                            title={entry}
                            onClick={() => onSelectHistoryEntry?.(entry)}
                          >
                            {entry}
                          </button>
                        ))}
                      </div>
                      <button
                        type="button"
                        className="search-history-clear"
                        onClick={onClearHistory}
                      >
                        {historyClearLabel}
                      </button>
                    </>
                  )}
                </div>
              ) : null}
            </div>
          ) : null}
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
