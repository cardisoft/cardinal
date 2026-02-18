import { describe, expect, it } from 'vitest';

import { splitPath } from '../path';

describe('splitPath', () => {
  it('returns empty values for undefined path', () => {
    expect(splitPath(undefined)).toEqual({ name: '', directory: '' });
  });

  it('returns root values for "/"', () => {
    expect(splitPath('/')).toEqual({ name: '/', directory: '/' });
  });

  it('handles paths without slash', () => {
    expect(splitPath('filename')).toEqual({ name: 'filename', directory: '' });
  });

  it('does not normalize backslashes', () => {
    expect(splitPath('C:\\Users\\alice\\file.txt')).toEqual({
      name: 'C:\\Users\\alice\\file.txt',
      directory: '',
    });
  });

  it('keeps trailing slash as empty leaf name', () => {
    expect(splitPath('/Users/alice/')).toEqual({
      name: '',
      directory: '/Users/alice',
    });
  });
});
