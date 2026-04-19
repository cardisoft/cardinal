import React from 'react';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { VirtualList } from '../VirtualList';
import type { SearchResultItem } from '../../types/search';
import type { SlabIndex } from '../../types/slab';

const mockUseDataLoader = vi.fn();
const mockUseIconViewport = vi.fn();
const mockEnsureRangeLoaded = vi.fn();

vi.mock('../../hooks/useDataLoader', () => ({
  useDataLoader: (...args: unknown[]) => mockUseDataLoader(...args),
}));

vi.mock('../../hooks/useIconViewport', () => ({
  useIconViewport: (...args: unknown[]) => mockUseIconViewport(...args),
}));

vi.mock('../Scrollbar', () => ({
  __esModule: true,
  default: () => null,
}));

const buildItem = (path: string): SearchResultItem => ({
  path,
  icon: undefined,
  metadata: undefined,
  size: undefined,
  mtime: undefined,
  ctime: undefined,
});

describe('VirtualList frozen viewport', () => {
  beforeEach(() => {
    mockEnsureRangeLoaded.mockReset();
    mockUseIconViewport.mockReset();
    Object.defineProperty(HTMLElement.prototype, 'clientHeight', {
      configurable: true,
      get: () => 40,
    });
  });

  it('keeps the previous viewport visible until the new viewport data is ready', async () => {
    let cache = new Map<SlabIndex, SearchResultItem>([
      [1 as SlabIndex, buildItem('/old-a')],
      [2 as SlabIndex, buildItem('/old-b')],
    ]);

    mockUseDataLoader.mockImplementation(() => ({
      cache,
      ensureRangeLoaded: mockEnsureRangeLoaded,
    }));

    const renderRow = (
      rowIndex: number,
      item: SearchResultItem | undefined,
      style: React.CSSProperties,
    ) => (
      <div key={`${rowIndex}-${item?.path ?? 'loading'}`} style={style}>
        {item?.path ?? 'loading'}
      </div>
    );

    const { rerender, container } = render(
      <VirtualList
        results={[1 as SlabIndex, 2 as SlabIndex]}
        dataResultsVersion={1}
        displayedResultsVersion={1}
        rowHeight={20}
        overscan={0}
        renderRow={renderRow}
        onScrollSync={vi.fn()}
      />,
    );

    await waitFor(() => {
      expect(screen.getByText('/old-a')).toBeInTheDocument();
      expect(screen.getByText('/old-b')).toBeInTheDocument();
    });

    cache = new Map<SlabIndex, SearchResultItem>();
    rerender(
      <VirtualList
        results={[3 as SlabIndex, 4 as SlabIndex]}
        dataResultsVersion={2}
        displayedResultsVersion={2}
        rowHeight={20}
        overscan={0}
        renderRow={renderRow}
        onScrollSync={vi.fn()}
      />,
    );

    await waitFor(() => {
      const overlay = container.querySelector('.virtual-list-overlay');
      expect(overlay).not.toBeNull();
      expect(screen.getByText('/old-a')).toBeInTheDocument();
      expect(screen.getByText('/old-b')).toBeInTheDocument();
    });

    cache = new Map<SlabIndex, SearchResultItem>([
      [3 as SlabIndex, buildItem('/new-a')],
      [4 as SlabIndex, buildItem('/new-b')],
    ]);
    rerender(
      <VirtualList
        results={[3 as SlabIndex, 4 as SlabIndex]}
        dataResultsVersion={2}
        displayedResultsVersion={2}
        rowHeight={20}
        overscan={0}
        renderRow={renderRow}
        onScrollSync={vi.fn()}
      />,
    );

    await waitFor(() => {
      expect(container.querySelector('.virtual-list-overlay')).toBeNull();
      expect(screen.queryByText('/old-a')).not.toBeInTheDocument();
      expect(screen.queryByText('/old-b')).not.toBeInTheDocument();
      expect(screen.getByText('/new-a')).toBeInTheDocument();
      expect(screen.getByText('/new-b')).toBeInTheDocument();
    });
  });

  it('drops the frozen viewport if the user scrolls before the next viewport is ready', async () => {
    let cache = new Map<SlabIndex, SearchResultItem>([
      [1 as SlabIndex, buildItem('/old-a')],
      [2 as SlabIndex, buildItem('/old-b')],
      [3 as SlabIndex, buildItem('/old-c')],
      [4 as SlabIndex, buildItem('/old-d')],
    ]);

    mockUseDataLoader.mockImplementation(() => ({
      cache,
      ensureRangeLoaded: mockEnsureRangeLoaded,
    }));

    const renderRow = (
      rowIndex: number,
      item: SearchResultItem | undefined,
      style: React.CSSProperties,
    ) => (
      <div key={`${rowIndex}-${item?.path ?? 'loading'}`} style={style}>
        {item?.path ?? 'loading'}
      </div>
    );

    const { rerender, container } = render(
      <VirtualList
        results={[1 as SlabIndex, 2 as SlabIndex, 3 as SlabIndex, 4 as SlabIndex]}
        dataResultsVersion={1}
        displayedResultsVersion={1}
        rowHeight={20}
        overscan={0}
        renderRow={renderRow}
        onScrollSync={vi.fn()}
      />,
    );

    await waitFor(() => {
      expect(screen.getByText('/old-a')).toBeInTheDocument();
      expect(screen.getByText('/old-b')).toBeInTheDocument();
    });

    cache = new Map<SlabIndex, SearchResultItem>();
    rerender(
      <VirtualList
        results={[5 as SlabIndex, 6 as SlabIndex, 7 as SlabIndex, 8 as SlabIndex]}
        dataResultsVersion={2}
        displayedResultsVersion={2}
        rowHeight={20}
        overscan={0}
        renderRow={renderRow}
        onScrollSync={vi.fn()}
      />,
    );

    await waitFor(() => {
      expect(container.querySelector('.virtual-list-overlay')).not.toBeNull();
    });

    fireEvent.wheel(container.querySelector('.virtual-list') as Element, {
      deltaY: 20,
      deltaMode: 0,
    });

    await waitFor(() => {
      expect(container.querySelector('.virtual-list-overlay')).toBeNull();
    });
  });
});
