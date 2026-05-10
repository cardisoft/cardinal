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
});
