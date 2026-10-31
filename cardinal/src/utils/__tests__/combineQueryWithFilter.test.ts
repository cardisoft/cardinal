import { describe, expect, it } from 'vitest';
import { combineQueryWithFilter } from '../../hooks/useFileSearch';

describe('combineQueryWithFilter', () => {
  it('returns the typed query unchanged when no filter is selected', () => {
    expect(combineQueryWithFilter('sql', '')).toBe('sql');
    expect(combineQueryWithFilter('sql', '   ')).toBe('sql');
  });

  it('uses the filter alone when the query box is empty', () => {
    // Picking "Videos" with nothing typed should browse videos, not return everything.
    expect(combineQueryWithFilter('', 'ext:mp4;mkv')).toBe('ext:mp4;mkv');
    expect(combineQueryWithFilter('   ', 'folder:')).toBe('folder:');
  });

  it('groups the query before AND-ing it with the filter', () => {
    // Without parentheses `a | b ext:mp4` would bind ext: to `b` only.
    expect(combineQueryWithFilter('a | b', 'ext:mp4')).toBe('(a | b) ext:mp4');
  });
});
