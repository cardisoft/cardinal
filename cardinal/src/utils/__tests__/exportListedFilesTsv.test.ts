import { describe, expect, it } from 'vitest';
import {
  buildListedFilesTsv,
  createListedFilesTsvFilename,
  type TsvColumnLabels,
} from '../exportListedFilesTsv';
import type { NodeInfoResponse } from '../../types/search';
import { formatKB, formatTimestamp } from '../format';

const labels: TsvColumnLabels = {
  filename: 'Filename',
  path: 'Path',
  size: 'Size',
  modified: 'Modified',
  created: 'Created',
};

describe('buildListedFilesTsv', () => {
  it('keeps row order and formats cells like file rows', () => {
    const nodes: NodeInfoResponse[] = [
      {
        path: '/Users/me/project/file.txt',
        metadata: {
          type: 0,
          size: 2048,
          mtime: 1704067200,
          ctime: 1704067100,
        },
      },
      {
        path: '/Users/me/project/folder',
        metadata: {
          type: 1,
          size: 0,
          mtime: 1704067300,
          ctime: 1704067000,
        },
      },
    ];

    const tsv = buildListedFilesTsv(nodes, labels);
    const lines = tsv.split('\n');

    expect(lines[0]).toBe('Filename\tPath\tSize\tModified\tCreated');
    expect(lines[1]).toBe(
      `file.txt\t/Users/me/project\t${formatKB(2048)}\t${formatTimestamp(1704067200)}\t${formatTimestamp(1704067100)}`,
    );
    expect(lines[2]).toBe(
      `folder\t/Users/me/project\t—\t${formatTimestamp(1704067300)}\t${formatTimestamp(1704067000)}`,
    );
  });

  it('sanitizes tabs and new lines in fields', () => {
    const nodes: NodeInfoResponse[] = [
      {
        path: '/Users/me/line\nbreak\tname.txt',
        metadata: {
          type: 0,
          size: 1024,
          mtime: 0,
          ctime: 0,
        },
      },
    ];

    const tsv = buildListedFilesTsv(nodes, labels);
    expect(tsv).not.toContain('\tname.txt\t/Users/me/line');
    expect(tsv).toContain('line break name.txt');
  });
});

describe('createListedFilesTsvFilename', () => {
  it('uses a stable date-based fallback filename for empty queries', () => {
    const filename = createListedFilesTsvFilename('', new Date('2026-02-22T15:04:05.678Z'));
    expect(filename).toBe('cardinal-word-list-all-files-2026-02-22.tsv');
  });

  it('includes normalized query text and current date', () => {
    const filename = createListedFilesTsvFilename(
      '  Foo/bar: baz + qux  ',
      new Date('2026-02-22T15:04:05.678Z'),
    );
    expect(filename).toBe('cardinal-word-list-foo-bar-baz-qux-2026-02-22.tsv');
  });
});
