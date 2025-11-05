import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { StateDisplay } from '../StateDisplay';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, options?: { query?: string }) => {
      switch (key) {
        case 'stateDisplay.loading':
          return 'Loading...';
        case 'stateDisplay.error':
          return 'Something went wrong';
        case 'stateDisplay.emptyTitle':
          return options?.query ? `No results for "${options.query}"` : 'No recent results';
        case 'stateDisplay.emptyMessage':
          return 'Try adjusting your filters.';
        default:
          return key;
      }
    },
  }),
}));

describe('StateDisplay', () => {
  it('shows a spinner when in loading state', () => {
    const { container } = render(<StateDisplay state="loading" />);
    expect(screen.getByText('Loading...')).toBeInTheDocument();
    expect(container.querySelector('.spinner')).not.toBeNull();
  });

  it('renders the error message from props', () => {
    render(<StateDisplay state="error" message="Disk scan failed" />);
    expect(screen.getByText('Something went wrong')).toBeInTheDocument();
    expect(screen.getByText('Disk scan failed')).toBeInTheDocument();
  });

  it('shows the empty-state copy with the current query', () => {
    render(<StateDisplay state="empty" query="report" />);
    expect(screen.getByText('No results for "report"')).toBeInTheDocument();
    expect(screen.getByText('Try adjusting your filters.')).toBeInTheDocument();
  });

  it('renders nothing for the results state', () => {
    const { container } = render(<StateDisplay state="results" />);
    expect(container.firstChild).toBeNull();
  });
});
