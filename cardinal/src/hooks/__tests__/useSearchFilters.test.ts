import { act, renderHook } from '@testing-library/react';
import { beforeEach, describe, expect, it } from 'vitest';
import { useSearchFilters } from '../useSearchFilters';
import { DEFAULT_FILTER_PRESETS } from '../../types/searchFilters';
import { parseFilterRows } from '../../utils/filterCsv';
import { decodeCsv, encodeCsv } from '../../utils/csv';

const renderFilters = () => renderHook(() => useSearchFilters());

describe('useSearchFilters', () => {
  beforeEach(() => {
    window.localStorage.clear();
  });

  it('seeds the bundled presets on first run', () => {
    const { result } = renderFilters();
    expect(result.current.filters.map((filter) => filter.name)).toEqual(
      DEFAULT_FILTER_PRESETS.map((preset) => preset.name),
    );
    // Nothing selected means no filtering.
    expect(result.current.activeSearch).toBe('');
  });

  it('exposes the search fragment of the selected preset', () => {
    const { result } = renderFilters();
    const videos = result.current.filters.find((filter) => filter.name === 'Videos');

    act(() => {
      result.current.selectFilter(videos!.id);
    });

    expect(result.current.activeSearch).toContain('ext:');
    expect(result.current.activeSearch).toContain('mp4');
  });

  it('adds, updates and deletes filters', () => {
    const { result } = renderFilters();

    act(() => {
      result.current.addFilter('  Custom  ', ' ext:abc ');
    });
    const custom = result.current.filters[result.current.filters.length - 1];
    expect(custom.name).toBe('Custom');
    expect(custom.search).toBe('ext:abc');

    act(() => {
      result.current.updateFilter(custom.id, { name: 'Renamed', search: 'folder:' });
    });
    expect(result.current.filters[result.current.filters.length - 1]).toMatchObject({
      name: 'Renamed',
      search: 'folder:',
    });

    act(() => {
      result.current.deleteFilter(custom.id);
    });
    expect(result.current.filters.some((filter) => filter.id === custom.id)).toBe(false);
  });

  it('clears the selection when the selected filter is deleted', () => {
    const { result } = renderFilters();
    const first = result.current.filters[0];

    act(() => {
      result.current.selectFilter(first.id);
      result.current.deleteFilter(first.id);
    });

    expect(result.current.selectedId).toBeNull();
    expect(result.current.activeSearch).toBe('');
  });

  it('appends imported filters without dropping existing ones', () => {
    const { result } = renderFilters();
    const before = result.current.filters.length;

    act(() => {
      result.current.appendFilters([
        { name: 'Imported A', search: 'ext:a' },
        { name: 'Imported B', search: 'ext:b' },
      ]);
    });

    expect(result.current.filters.length).toBe(before + 2);
    expect(result.current.filters.map((filter) => filter.name)).toContain('Imported A');
  });
});

describe('filter CSV round-trip', () => {
  it('exports and reimports the same entries', () => {
    const rows = [
      ['name', 'search'],
      ['视频', 'ext:mp4;mkv'],
      ['Folders', 'folder:'],
    ];
    const parsed = parseFilterRows(decodeCsv(encodeCsv(rows)));

    expect(parsed).toEqual([
      { name: '视频', search: 'ext:mp4;mkv' },
      { name: 'Folders', search: 'folder:' },
    ]);
  });

  it('accepts headerless payloads and drops blank names', () => {
    const parsed = parseFilterRows(decodeCsv('Videos,ext:mp4\n,ext:mkv\n'));
    expect(parsed).toEqual([{ name: 'Videos', search: 'ext:mp4' }]);
  });

  it('starts with no filter selected even if a legacy selection was stored', () => {
    // Regression: the persisted selection used to be restored on launch while the first tab's
    // `filterQuery` started empty, so the dropdown showed e.g. "Folders" while the launch list
    // returned unfiltered results — and picking "All" then looked like it did nothing.
    window.localStorage.setItem('cardinal.searchFilters.selected.v1', 'some-filter-id');

    const { result } = renderFilters();

    expect(result.current.selectedId).toBeNull();
    expect(result.current.activeSearch).toBe('');
    expect(window.localStorage.getItem('cardinal.searchFilters.selected.v1')).toBeNull();
  });

  it('still remembers the selection while the app is running', () => {
    const { result } = renderFilters();
    const first = result.current.filters[0];

    act(() => {
      result.current.selectFilter(first.id);
    });

    expect(result.current.selectedId).toBe(first.id);
    expect(result.current.activeSearch).toBe(first.search);
  });
});
