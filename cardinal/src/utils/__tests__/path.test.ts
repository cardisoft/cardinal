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

  it('normalizes Windows backslashes before splitting', () => {
    expect(splitPath('C:\\Users\\alice\\file.txt')).toEqual({
      name: 'file.txt',
      directory: 'C:/Users/alice',
    });
  });

  it('keeps trailing slash paths consistent with current behavior', () => {
    expect(splitPath('/Users/alice/')).toEqual({
      name: '/Users/alice/',
      directory: '/Users/alice',
    });
  });
});
