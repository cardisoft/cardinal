import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { SearchFilters } from '../SearchFilters';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

describe('SearchFilters', () => {
  it('turns friendly controls into an internal query without showing syntax', () => {
    const onChange = vi.fn();
    render(<SearchFilters onChange={onChange} />);

    fireEvent.change(screen.getByLabelText('search.filters.type'), {
      target: { value: 'picture' },
    });
    fireEvent.change(screen.getByLabelText('search.filters.modified'), {
      target: { value: 'pastweek' },
    });
    fireEvent.change(screen.getByLabelText('search.filters.size'), {
      target: { value: '>100MB' },
    });

    expect(onChange).toHaveBeenLastCalledWith('type:picture dm:pastweek size:>100MB');
    expect(screen.queryByDisplayValue(/type:|dm:|size:/)).not.toBeInTheDocument();

    fireEvent.change(screen.getByLabelText('search.filters.type'), {
      target: { value: 'folder' },
    });
    expect(onChange).toHaveBeenLastCalledWith('type:folder dm:pastweek size:>100MB');
  });

  it('restores visual controls when a saved filter query is applied', () => {
    const onChange = vi.fn();
    render(<SearchFilters query="type:folder dm:today size:>1GB" onChange={onChange} />);

    expect(screen.getByLabelText('search.filters.type')).toHaveValue('folder');
    expect(screen.getByLabelText('search.filters.modified')).toHaveValue('today');
    expect(screen.getByLabelText('search.filters.size')).toHaveValue('>1GB');
    expect(onChange).not.toHaveBeenCalled();
  });
});
