import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import StatusBar from '../StatusBar';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, opts?: Record<string, unknown>) =>
      opts ? `${key}:${JSON.stringify(opts)}` : key,
  }),
}));

const baseProps = {
  scannedFiles: 100,
  processedEvents: 5,
  lifecycleState: 'Ready' as const,
  searchDurationMs: 12,
  resultCount: 3,
  activeTab: 'files' as const,
  onTabChange: vi.fn(),
  onRequestRescan: vi.fn(),
  rescanErrorCount: 0,
  selection: null,
};

describe('StatusBar selection summary', () => {
  it('shows nothing about selection when there is none', () => {
    const { container } = render(<StatusBar {...baseProps} />);
    expect(container.querySelector('.status-selection')).toBeNull();
  });

  it('shows the selected path and human-readable size', () => {
    const { container } = render(
      <StatusBar
        {...baseProps}
        selection={{ path: '/Users/me/Movies/demo.mp4', size: '1.24G', count: 1 }}
      />,
    );

    expect(screen.getByText('/Users/me/Movies/demo.mp4')).toBeInTheDocument();
    expect(screen.getByText('1.24G')).toBeInTheDocument();
    // A single selection needs no count badge.
    expect(container.querySelector('.status-selection__count')).toBeNull();
  });

  it('shows how many entries are selected when more than one', () => {
    const { container } = render(
      <StatusBar {...baseProps} selection={{ path: '/Users/me/a.txt', size: '4KB', count: 3 }} />,
    );

    expect(container.querySelector('.status-selection__count')?.textContent).toContain('3');
  });
});
