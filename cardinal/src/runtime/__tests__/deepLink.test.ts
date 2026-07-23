import { describe, expect, it } from 'vitest';
import { parseDeepLinkSearchUrl, parseDeepLinkSearchUrls } from '../deepLink';

describe('deepLink runtime parsing', () => {
  it('parses search scope links', () => {
    expect(
      parseDeepLinkSearchUrl('cardinal://search?scope=%2FUsers%2Fzc%2FProjects&q=report'),
    ).toEqual({
      query: 'report',
      directoryQuery: '/Users/zc/Projects',
      directoryScopeOpen: true,
    });
  });

  it('supports scope-only folder searches', () => {
    expect(parseDeepLinkSearchUrl('cardinal://search?scope=/tmp/work')).toEqual({
      query: '',
      directoryQuery: '/tmp/work',
      directoryScopeOpen: true,
    });
  });

  it('rejects unsupported links', () => {
    expect(parseDeepLinkSearchUrl('https://search?scope=/tmp')).toBeNull();
    expect(parseDeepLinkSearchUrl('cardinal://open?scope=/tmp')).toBeNull();
    expect(parseDeepLinkSearchUrl('cardinal://search')).toBeNull();
    expect(parseDeepLinkSearchUrl('cardinal://search?query=report')).toBeNull();
    expect(parseDeepLinkSearchUrl('cardinal://search?folder=/tmp')).toBeNull();
    expect(parseDeepLinkSearchUrl('cardinal://search?directoryQuery=/tmp')).toBeNull();
    expect(parseDeepLinkSearchUrl('not a url')).toBeNull();
  });

  it('filters URL batches to supported search actions', () => {
    expect(
      parseDeepLinkSearchUrls([
        'cardinal://search?q=report',
        'cardinal://open?scope=/tmp',
        'cardinal://search?directoryQuery=/tmp/work',
        'cardinal://search?scope=/tmp/work',
      ]),
    ).toEqual([
      { query: 'report', directoryQuery: '', directoryScopeOpen: false },
      { query: '', directoryQuery: '/tmp/work', directoryScopeOpen: true },
    ]);
  });
});
