import { describe, expect, it } from 'vitest';
import { extractContentTerms } from '../contentQuery';

describe('extractContentTerms', () => {
  it('extracts bare and quoted content filters', () => {
    expect(extractContentTerms('report content:foo content:"bar baz"')).toEqual(['foo', 'bar baz']);
  });

  it('unescapes quoted values and ignores empty filters', () => {
    expect(extractContentTerms(String.raw`content:"foo\"bar" content:"" content:`)).toEqual([
      'foo"bar',
    ]);
  });

  it('deduplicates repeated values while preserving order', () => {
    expect(extractContentTerms('content:foo content:bar content:foo')).toEqual(['foo', 'bar']);
  });
});
