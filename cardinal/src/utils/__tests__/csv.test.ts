import { describe, expect, it } from 'vitest';
import { decodeCsv, encodeCsv } from '../csv';

describe('csv', () => {
  it('round-trips plain rows', () => {
    const rows = [
      ['category', 'name', 'query'],
      ['影视', '周星驰', '周星驰推荐'],
    ];
    expect(decodeCsv(encodeCsv(rows))).toEqual(rows);
  });

  it('quotes fields containing commas, quotes and newlines', () => {
    const rows = [['a,b', 'say "hi"', 'line1\nline2']];
    const encoded = encodeCsv(rows);

    expect(encoded).toContain('"a,b"');
    expect(encoded).toContain('"say ""hi"""');
    expect(decodeCsv(encoded)).toEqual(rows);
  });

  it('handles CRLF row endings and a UTF-8 BOM', () => {
    const text = '\ufeffname,search\r\nvideos,ext:mp4\r\ndocs,ext:md\r\n';
    expect(decodeCsv(text)).toEqual([
      ['name', 'search'],
      ['videos', 'ext:mp4'],
      ['docs', 'ext:md'],
    ]);
  });

  it('keeps empty trailing fields', () => {
    expect(decodeCsv('a,,c\n')).toEqual([['a', '', 'c']]);
  });

  it('returns no rows for empty input', () => {
    expect(decodeCsv('')).toEqual([]);
    expect(encodeCsv([])).toBe('');
  });
});
