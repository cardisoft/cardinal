import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { SortMenu } from '../SortMenu';

const labels = {
  sortBySize: 'By size',
  sortByCreated: 'By created',
  sortByModified: 'By modified',
  sortByName: 'By name',
  ascending: 'Ascending',
  descending: 'Descending',
  clear: 'Clear',
};

const renderMenu = (overrides: Partial<React.ComponentProps<typeof SortMenu>> = {}) =>
  render(
    <SortMenu
      sortState={null}
      onSelect={vi.fn()}
      labels={labels}
      x={100}
      y={100}
      onClose={vi.fn()}
      {...overrides}
    />,
  );

describe('SortMenu', () => {
  it('lists the four sort keys', () => {
    renderMenu();
    expect(screen.getByText('By size')).toBeInTheDocument();
    expect(screen.getByText('By created')).toBeInTheDocument();
    expect(screen.getByText('By modified')).toBeInTheDocument();
    expect(screen.getByText('By name')).toBeInTheDocument();
  });

  it('reveals the direction submenu on hover of the second-level item', async () => {
    renderMenu();

    fireEvent.pointerEnter(screen.getByText('By created'));

    await waitFor(() => expect(screen.getByText('Ascending')).toBeInTheDocument());
    expect(screen.getByText('Descending')).toBeInTheDocument();
  });

  it('reports the chosen key and direction', () => {
    const onSelect = vi.fn();
    const onClose = vi.fn();
    renderMenu({ onSelect, onClose });

    fireEvent.pointerEnter(screen.getByText('By size'));
    fireEvent.click(screen.getByText('Descending'));

    expect(onSelect).toHaveBeenCalledWith({ key: 'size', direction: 'desc' });
    expect(onClose).toHaveBeenCalled();
  });

  it('marks the active direction and offers clearing', () => {
    const onSelect = vi.fn();
    renderMenu({ sortState: { key: 'mtime', direction: 'asc' }, onSelect });

    fireEvent.pointerEnter(screen.getByText('By modified'));
    // The active choice is checked.
    const ascending = screen.getByText('Ascending');
    expect(ascending).toHaveAttribute('aria-checked', 'true');

    fireEvent.click(screen.getByText('Clear'));
    expect(onSelect).toHaveBeenCalledWith(null);
  });

  it('closes on Escape', () => {
    const onClose = vi.fn();
    renderMenu({ onClose });

    fireEvent.keyDown(document, { key: 'Escape' });
    expect(onClose).toHaveBeenCalled();
  });
});
