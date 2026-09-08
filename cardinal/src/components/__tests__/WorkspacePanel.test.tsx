import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { WorkspacePanel } from '../WorkspacePanel';

vi.mock('react-i18next', () => ({ useTranslation: () => ({ t: (key: string) => key }) }));

const baseProps = {
  activeSection: 'saved' as const,
  onClose: vi.fn(),
  savedSearches: [],
  favorites: [],
  selectedPaths: ['/tmp/selected'],
  onSaveSearch: vi.fn(),
  onApplySavedSearch: vi.fn(),
  onRemoveSavedSearch: vi.fn(),
  onAddSelectedFavorites: vi.fn(),
  onOpenFavorite: vi.fn(),
  onRevealFavorite: vi.fn(),
  onRemoveFavorite: vi.fn(),
  coverage: {
    watchRoot: '/',
    ignorePaths: ['/System', '/tmp/cache'],
    includePaths: ['/Volumes/Work'],
    fullDiskAccessGranted: true,
    lifecycleState: 'Ready' as const,
    scannedFiles: 1200,
    currentQuery: 'invoice',
    currentDirectoryQuery: '',
  },
  operations: {
    indexingPaused: false,
    lifecycleState: 'Ready' as const,
    activity: 'idle' as const,
    scannedFiles: 1200,
    fileOperations: [],
    isSearching: false,
    isSorting: false,
    onToggleIndexing: vi.fn(),
    onCancelSearch: vi.fn(),
    onCancelSort: vi.fn(),
    onRequestRescan: vi.fn(),
  },
  onOpenPreferences: vi.fn(),
};

describe('WorkspacePanel', () => {
  it('saves the current search with a user supplied name', () => {
    const onSaveSearch = vi.fn();
    render(<WorkspacePanel {...baseProps} onSaveSearch={onSaveSearch} />);

    fireEvent.change(screen.getByLabelText('workspace.saved.nameLabel'), {
      target: { value: 'Aylık faturalar' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'workspace.saved.saveCurrent' }));

    expect(onSaveSearch).toHaveBeenCalledWith('Aylık faturalar');
  });

  it('adds the current selection and exposes favorite actions', () => {
    const onAddSelectedFavorites = vi.fn();
    const onOpenFavorite = vi.fn();
    render(
      <WorkspacePanel
        {...baseProps}
        activeSection="favorites"
        favorites={[{ path: '/tmp/favorite', addedAt: 1 }]}
        onAddSelectedFavorites={onAddSelectedFavorites}
        onOpenFavorite={onOpenFavorite}
      />,
    );

    fireEvent.click(screen.getByRole('button', { name: 'workspace.favorites.addSelection' }));
    fireEvent.click(screen.getByRole('button', { name: 'workspace.favorites.open' }));

    expect(onAddSelectedFavorites).toHaveBeenCalledTimes(1);
    expect(onOpenFavorite).toHaveBeenCalledWith('/tmp/favorite');
  });

  it('explains index coverage and offers settings and rescan actions', () => {
    const onOpenPreferences = vi.fn();
    const onRequestRescan = vi.fn();
    render(
      <WorkspacePanel
        {...baseProps}
        activeSection="coverage"
        onOpenPreferences={onOpenPreferences}
        operations={{ ...baseProps.operations, onRequestRescan }}
      />,
    );

    expect(screen.getByText('/System')).toBeInTheDocument();
    expect(screen.getByText('/Volumes/Work')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'workspace.coverage.openSettings' }));
    fireEvent.click(screen.getByRole('button', { name: 'workspace.operations.rescan' }));

    expect(onOpenPreferences).toHaveBeenCalledTimes(1);
    expect(onRequestRescan).toHaveBeenCalledTimes(1);
  });

  it('shows active operations and lets the user pause or cancel them', () => {
    const onToggleIndexing = vi.fn();
    const onCancelSearch = vi.fn();
    const onCancelSort = vi.fn();
    render(
      <WorkspacePanel
        {...baseProps}
        activeSection="operations"
        operations={{
          ...baseProps.operations,
          lifecycleState: 'Updating',
          isSearching: true,
          isSorting: true,
          onToggleIndexing,
          onCancelSearch,
          onCancelSort,
        }}
      />,
    );

    fireEvent.click(screen.getByRole('button', { name: 'workspace.operations.pauseIndexing' }));
    fireEvent.click(screen.getByRole('button', { name: 'workspace.operations.cancelSearch' }));
    fireEvent.click(screen.getByRole('button', { name: 'workspace.operations.cancelSort' }));

    expect(onToggleIndexing).toHaveBeenCalledTimes(1);
    expect(onCancelSearch).toHaveBeenCalledTimes(1);
    expect(onCancelSort).toHaveBeenCalledTimes(1);
  });
});
