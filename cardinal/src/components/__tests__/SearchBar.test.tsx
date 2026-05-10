import { fireEvent, render, screen } from '@testing-library/react';
import { createRef } from 'react';
import type { ComponentProps } from 'react';
import { describe, expect, it, vi } from 'vitest';
import { SearchBar } from '../SearchBar';

const renderSearchBar = (overrides: Partial<ComponentProps<typeof SearchBar>> = {}) => {
  const props: ComponentProps<typeof SearchBar> = {
    inputRef: createRef<HTMLInputElement>(),
    placeholder: 'Search',
    value: '',
    onChange: vi.fn(),
    onKeyDown: vi.fn(),
    directoryScopeEnabled: true,
    directoryScopeOpen: true,
    directoryScopeLabel: 'Folder scope',
    directoryPlaceholder: 'Folder',
    directoryValue: '',
    onToggleDirectoryScope: vi.fn(),
    onDirectoryChange: vi.fn(),
    onDirectoryKeyDown: vi.fn(),
    caseSensitive: false,
    onToggleCaseSensitive: vi.fn(),
    caseSensitiveLabel: 'Case sensitive',
    onFocus: vi.fn(),
    onBlur: vi.fn(),
    ...overrides,
  };

  render(<SearchBar {...props} />);
  return props;
};

describe('SearchBar', () => {
  it('does not mark the folder scope toggle as pressed while folded with a saved value', () => {
    renderSearchBar({
      directoryScopeOpen: false,
      directoryValue: 'Work/Docs',
    });

    expect(screen.getByRole('button', { name: 'Folder scope' })).toHaveAttribute(
      'aria-pressed',
      'false',
    );
  });

  it('routes directory input focus state through the shared search focus handlers', () => {
    const onFocus = vi.fn();
    const onBlur = vi.fn();
    renderSearchBar({ onFocus, onBlur });
    const directoryInput = screen.getByPlaceholderText('Folder');

    onFocus.mockClear();
    fireEvent.focus(directoryInput);
    expect(onFocus).toHaveBeenCalledTimes(1);

    fireEvent.blur(directoryInput);
    expect(onBlur).toHaveBeenCalledTimes(1);
  });

  it('moves focus from the query start to the directory input with ArrowLeft', () => {
    const queryRef = createRef<HTMLInputElement>();
    const onKeyDown = vi.fn();
    renderSearchBar({
      inputRef: queryRef,
      value: 'report',
      directoryValue: 'Work/Docs',
      onKeyDown,
    });
    const directoryInput = screen.getByPlaceholderText('Folder') as HTMLInputElement;
    const queryInput = screen.getByPlaceholderText('Search') as HTMLInputElement;

    queryInput.focus();
    queryInput.setSelectionRange(0, 0);
    fireEvent.keyDown(queryInput, { key: 'ArrowLeft' });

    expect(document.activeElement).toBe(directoryInput);
    expect(directoryInput.selectionStart).toBe('Work/Docs'.length);
    expect(onKeyDown).not.toHaveBeenCalled();
  });

  it('moves focus from the directory end to the query input with ArrowRight', () => {
    const onDirectoryKeyDown = vi.fn();
    renderSearchBar({
      value: 'report',
      directoryValue: 'Work/Docs',
      onDirectoryKeyDown,
    });
    const directoryInput = screen.getByPlaceholderText('Folder') as HTMLInputElement;
    const queryInput = screen.getByPlaceholderText('Search') as HTMLInputElement;

    directoryInput.focus();
    directoryInput.setSelectionRange('Work/Docs'.length, 'Work/Docs'.length);
    fireEvent.keyDown(directoryInput, { key: 'ArrowRight' });

    expect(document.activeElement).toBe(queryInput);
    expect(queryInput.selectionStart).toBe(0);
    expect(onDirectoryKeyDown).not.toHaveBeenCalled();
  });

  it('does not move from query to directory when ArrowLeft has modifiers or selection is not at the start', () => {
    const onKeyDown = vi.fn();
    renderSearchBar({
      value: 'report',
      directoryValue: 'Work/Docs',
      onKeyDown,
    });
    const directoryInput = screen.getByPlaceholderText('Folder') as HTMLInputElement;
    const queryInput = screen.getByPlaceholderText('Search') as HTMLInputElement;

    queryInput.focus();
    queryInput.setSelectionRange(0, 0);
    fireEvent.keyDown(queryInput, { key: 'ArrowLeft', metaKey: true });
    expect(document.activeElement).toBe(queryInput);
    expect(onKeyDown).toHaveBeenCalledTimes(1);

    onKeyDown.mockClear();
    queryInput.setSelectionRange(1, 1);
    fireEvent.keyDown(queryInput, { key: 'ArrowLeft' });
    expect(document.activeElement).toBe(queryInput);
    expect(onKeyDown).toHaveBeenCalledTimes(1);

    onKeyDown.mockClear();
    queryInput.setSelectionRange(0, 2);
    fireEvent.keyDown(queryInput, { key: 'ArrowLeft' });
    expect(document.activeElement).toBe(queryInput);
    expect(onKeyDown).toHaveBeenCalledTimes(1);
    expect(document.activeElement).not.toBe(directoryInput);
  });

  it('does not move from query to directory when the folder scope is folded', () => {
    const onKeyDown = vi.fn();
    renderSearchBar({
      directoryScopeOpen: false,
      value: 'report',
      directoryValue: 'Work/Docs',
      onKeyDown,
    });
    const queryInput = screen.getByPlaceholderText('Search') as HTMLInputElement;

    queryInput.focus();
    queryInput.setSelectionRange(0, 0);
    fireEvent.keyDown(queryInput, { key: 'ArrowLeft' });

    expect(document.activeElement).toBe(queryInput);
    expect(onKeyDown).toHaveBeenCalledTimes(1);
  });

  it('does not move from directory to query when ArrowRight has modifiers or selection is not at the end', () => {
    const onDirectoryKeyDown = vi.fn();
    renderSearchBar({
      value: 'report',
      directoryValue: 'Work/Docs',
      onDirectoryKeyDown,
    });
    const directoryInput = screen.getByPlaceholderText('Folder') as HTMLInputElement;
    const queryInput = screen.getByPlaceholderText('Search') as HTMLInputElement;

    directoryInput.focus();
    directoryInput.setSelectionRange('Work/Docs'.length, 'Work/Docs'.length);
    fireEvent.keyDown(directoryInput, { key: 'ArrowRight', shiftKey: true });
    expect(document.activeElement).toBe(directoryInput);
    expect(onDirectoryKeyDown).toHaveBeenCalledTimes(1);

    onDirectoryKeyDown.mockClear();
    directoryInput.setSelectionRange(1, 1);
    fireEvent.keyDown(directoryInput, { key: 'ArrowRight' });
    expect(document.activeElement).toBe(directoryInput);
    expect(onDirectoryKeyDown).toHaveBeenCalledTimes(1);

    onDirectoryKeyDown.mockClear();
    directoryInput.setSelectionRange(0, 'Work/Docs'.length);
    fireEvent.keyDown(directoryInput, { key: 'ArrowRight' });
    expect(document.activeElement).toBe(directoryInput);
    expect(onDirectoryKeyDown).toHaveBeenCalledTimes(1);
    expect(document.activeElement).not.toBe(queryInput);
  });
});
