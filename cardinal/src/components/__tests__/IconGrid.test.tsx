import React, { createRef } from 'react';
import { fireEvent, render, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { IconGrid, ICON_SIZE_PRESETS, gridLabelAreaHeight, gridLabelWidth } from '../IconGrid';
import type { IconGridHandle } from '../IconGrid';
import type { SearchResultItem } from '../../types/search';
import type { SlabIndex } from '../../types/slab';

const mockUseDataLoader = vi.fn();
const mockUseIconViewport = vi.fn();
const mockEnsureRangeLoaded = vi.fn();

vi.mock('../../hooks/useDataLoader', () => ({
  useDataLoader: (...args: unknown[]) => mockUseDataLoader(...args),
  GRID_ICON_SIZE: 384,
}));

vi.mock('../../hooks/useIconViewport', () => ({
  useIconViewport: (...args: unknown[]) => mockUseIconViewport(...args),
}));

const scrollbarProps: { viewportHeight?: number }[] = [];

vi.mock('../Scrollbar', () => ({
  __esModule: true,
  default: (props: { viewportHeight: number }) => {
    scrollbarProps.push(props);
    return null;
  },
}));

const ITEM_COUNT = 24;

const buildItem = (index: number): SearchResultItem => ({
  path: `/file-${index}`,
  icon: undefined,
  metadata: undefined,
  size: undefined,
  mtime: undefined,
  ctime: undefined,
});

const results = Array.from({ length: ITEM_COUNT }, (_, index) => index as SlabIndex);

describe('IconGrid scrolling', () => {
  beforeEach(() => {
    mockEnsureRangeLoaded.mockReset();
    mockUseIconViewport.mockReset();
    mockUseDataLoader.mockImplementation(() => ({
      cache: new Map<SlabIndex, SearchResultItem>(
        results.map((slabIndex, index) => [slabIndex, buildItem(index)]),
      ),
      ensureRangeLoaded: mockEnsureRangeLoaded,
    }));
    Object.defineProperty(HTMLElement.prototype, 'clientHeight', {
      configurable: true,
      get: () => 300,
    });
    Object.defineProperty(HTMLElement.prototype, 'clientWidth', {
      configurable: true,
      get: () => 600,
    });
  });

  it('reserves three label lines derived from the base font size', () => {
    // The tiled view must keep a uniform cell height: a three-line name takes space from the
    // icon instead of making the row taller.
    expect(gridLabelAreaHeight(13)).toBeGreaterThan(gridLabelAreaHeight(20) / 2);
    expect(gridLabelAreaHeight(20)).toBeGreaterThan(gridLabelAreaHeight(13));

    const { container } = render(
      <IconGrid
        results={results}
        dataResultsVersion={1}
        displayedResultsVersion={1}
        iconSize="medium"
        fontSize={20}
        renderItem={(rowIndex, _item, style) => (
          <div key={rowIndex} data-testid={`sized-${rowIndex}`} style={style} />
        )}
      />,
    );

    const first = container.querySelector('[data-testid="sized-0"]') as HTMLElement;
    const expectedHeight = ICON_SIZE_PRESETS.medium + gridLabelAreaHeight(20);
    expect(Number.parseFloat(first.style.height)).toBe(expectedHeight);
    // The icon box caps itself at the preset size and shrinks for longer names.
    expect(first.style.getPropertyValue('--icon-size')).toBe(`${ICON_SIZE_PRESETS.medium}px`);
    // Six CJK glyphs must fit on a line, and the width follows the font-size setting.
    expect(first.style.getPropertyValue('--label-width')).toBe(`${gridLabelWidth(20)}px`);
    expect(gridLabelWidth(20)).toBeGreaterThan(gridLabelWidth(13));
    // Rounded to whole pixels (94px for the default 16px font size).
    expect(gridLabelWidth(20)).toBe(Math.round(20 * 0.78 * 6));
  });

  it('keeps grid items inside the viewport after scrolling', () => {
    const { container } = render(
      <IconGrid
        results={results}
        dataResultsVersion={1}
        displayedResultsVersion={1}
        iconSize="small"
        renderItem={(rowIndex, _item, style) => (
          <div key={rowIndex} data-testid={`item-${rowIndex}`} style={style} />
        )}
      />,
    );

    const items = container.querySelector('.virtual-list-items') as HTMLElement;
    const visibleCount = items.children.length;
    expect(visibleCount).toBeGreaterThan(0);

    fireEvent.wheel(container.querySelector('.icon-grid') as Element, {
      deltaY: ICON_SIZE_PRESETS.small,
      deltaMode: 0,
    });

    // After scrolling, rendered cells must be shifted up by the scroll offset so the
    // visible window stays in view instead of being pushed below the clipped viewport.
    const tops = Array.from(items.children).map((child) =>
      Number.parseFloat((child as HTMLElement).style.top),
    );
    const minTop = Math.min(...tops);
    expect(minTop).toBeLessThan(0);
  });

  it('does not jump when asked to reveal an already visible row', async () => {
    const ref = createRef<IconGridHandle>();
    const { container } = render(
      <IconGrid
        ref={ref}
        results={results}
        dataResultsVersion={1}
        displayedResultsVersion={1}
        iconSize="small"
        renderItem={(rowIndex, _item, style) => (
          <div key={rowIndex} data-testid={`item-${rowIndex}`} style={style} />
        )}
      />,
    );

    const topOfFirstCell = () => {
      const cell = container.querySelector('[data-testid="item-0"]') as HTMLElement;
      return Number.parseFloat(cell.style.top);
    };

    // Scroll down one grid row so the viewport no longer starts at offset 0.
    fireEvent.wheel(container.querySelector('.icon-grid') as Element, {
      deltaY: ICON_SIZE_PRESETS.small,
      deltaMode: 0,
    });
    await waitFor(() => expect(topOfFirstCell()).toBeLessThan(0));
    const scrolledTop = topOfFirstCell();

    // Row 1 is still fully inside the viewport, so revealing it with `nearest` (the mode used
    // when the active row changes, e.g. right-click selection) must not move the view.
    ref.current?.scrollToRow(8, 'nearest');

    await waitFor(() => expect(ref.current).not.toBeNull());
    // Give the state update a chance to land before asserting nothing moved.
    await new Promise((resolve) => setTimeout(resolve, 50));
    expect(topOfFirstCell()).toBe(scrolledTop);
  });

  it('measures the scrolling viewport so the horizontal scrollbar cannot cover the last row', () => {
    // The wrapper includes the horizontal scrollbar strip; the inner scrolling element does
    // not. Virtualisation must use the smaller value or the last row sits under the bar.
    Object.defineProperty(HTMLElement.prototype, 'clientHeight', {
      configurable: true,
      get(this: HTMLElement) {
        return this.classList.contains('virtual-list-viewport') ? 280 : 300;
      },
    });

    scrollbarProps.length = 0;
    render(
      <IconGrid
        results={results}
        dataResultsVersion={1}
        displayedResultsVersion={1}
        iconSize="small"
        renderItem={(rowIndex, _item, style) => (
          <div key={rowIndex} data-testid={`item-${rowIndex}`} style={style} />
        )}
      />,
    );

    expect(scrollbarProps[scrollbarProps.length - 1]?.viewportHeight).toBe(280);
  });
});

describe('IconGrid end-of-list callback', () => {
  beforeEach(() => {
    mockUseDataLoader.mockImplementation(() => ({
      cache: new Map<SlabIndex, SearchResultItem>(
        results.map((slabIndex, index) => [slabIndex, buildItem(index)]),
      ),
      ensureRangeLoaded: mockEnsureRangeLoaded,
    }));
    Object.defineProperty(HTMLElement.prototype, 'clientHeight', {
      configurable: true,
      get: () => 300,
    });
    Object.defineProperty(HTMLElement.prototype, 'clientWidth', {
      configurable: true,
      get: () => 600,
    });
  });

  it('asks for the next page when scrolled near the bottom', () => {
    // Long enough that the first screen is nowhere near the end.
    const manyResults = Array.from({ length: 400 }, (_, index) => index as SlabIndex);
    mockUseDataLoader.mockImplementation(() => ({
      cache: new Map<SlabIndex, SearchResultItem>(
        manyResults.map((slabIndex, index) => [slabIndex, buildItem(index)]),
      ),
      ensureRangeLoaded: mockEnsureRangeLoaded,
    }));

    const onNearEnd = vi.fn();
    const { container } = render(
      <IconGrid
        results={manyResults}
        dataResultsVersion={1}
        displayedResultsVersion={1}
        iconSize="small"
        onNearEnd={onNearEnd}
        renderItem={(rowIndex, _item, style) => <div key={rowIndex} style={style} />}
      />,
    );

    expect(onNearEnd).not.toHaveBeenCalled();

    fireEvent.wheel(container.querySelector('.icon-grid') as Element, {
      deltaY: 100000,
      deltaMode: 0,
    });

    expect(onNearEnd).toHaveBeenCalled();
  });
});
