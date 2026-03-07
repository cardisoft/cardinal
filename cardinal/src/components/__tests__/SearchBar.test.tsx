import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { SearchBar } from '../SearchBar';

const baseProps = {
  inputRef: { current: null },
  placeholder: 'Search',
  value: '',
  onChange: vi.fn(),
  onKeyDown: vi.fn(),
  caseSensitive: false,
  onToggleCaseSensitive: vi.fn(),
  caseSensitiveLabel: 'Toggle case-sensitive matching',
  filtersToggleLabel: 'Show filters',
  onToggleFilters: vi.fn(),
  onFocus: vi.fn(),
  onBlur: vi.fn(),
};

describe('SearchBar', () => {
  it('shows a filters toggle button when provided', () => {
    const onToggleFilters = vi.fn();
    render(<SearchBar {...baseProps} onToggleFilters={onToggleFilters} />);

    fireEvent.click(screen.getByRole('button', { name: 'Show filters' }));

    expect(onToggleFilters).toHaveBeenCalledTimes(1);
  });

  it('keeps the case sensitivity toggle available', () => {
    render(<SearchBar {...baseProps} />);

    expect(screen.getByRole('checkbox', { name: 'Toggle case-sensitive matching' })).toBeVisible();
  });
});
