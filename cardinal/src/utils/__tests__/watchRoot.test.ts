import { describe, expect, it } from 'vitest';

import { getWatchRootValidation, isPathInputValid } from '../watchRoot';

describe('isPathInputValid', () => {
  it('rejects empty or whitespace-only inputs', () => {
    expect(isPathInputValid('')).toBe(false);
    expect(isPathInputValid('   ')).toBe(false);
  });

  it('accepts absolute paths', () => {
    expect(isPathInputValid('/')).toBe(true);
    expect(isPathInputValid('/Users/example')).toBe(true);
    expect(isPathInputValid(' /var ')).toBe(true);
  });

  it('accepts tilde roots', () => {
    expect(isPathInputValid('~')).toBe(true);
    expect(isPathInputValid('~/Documents')).toBe(true);
    expect(isPathInputValid(' ~/Downloads ')).toBe(true);
  });

  it('rejects relative paths and unsupported tilde forms', () => {
    expect(isPathInputValid('relative/path')).toBe(false);
    expect(isPathInputValid('./relative')).toBe(false);
    expect(isPathInputValid('../relative')).toBe(false);
    expect(isPathInputValid('~user')).toBe(false);
    expect(isPathInputValid('~user/Documents')).toBe(false);
  });
});

describe('getWatchRootValidation', () => {
  it('marks empty values as required', () => {
    expect(getWatchRootValidation('')).toEqual({
      isValid: false,
      errorKey: 'watchRoot.errors.required',
    });
    expect(getWatchRootValidation('   ')).toEqual({
      isValid: false,
      errorKey: 'watchRoot.errors.required',
    });
  });

  it('rejects non-absolute values with the absolute error key', () => {
    expect(getWatchRootValidation('relative')).toEqual({
      isValid: false,
      errorKey: 'watchRoot.errors.absolute',
    });
    expect(getWatchRootValidation('./relative')).toEqual({
      isValid: false,
      errorKey: 'watchRoot.errors.absolute',
    });
    expect(getWatchRootValidation('~user')).toEqual({
      isValid: false,
      errorKey: 'watchRoot.errors.absolute',
    });
  });

  it('accepts absolute and tilde values with no error key', () => {
    expect(getWatchRootValidation('/')).toEqual({
      isValid: true,
      errorKey: null,
    });
    expect(getWatchRootValidation('/Users/example')).toEqual({
      isValid: true,
      errorKey: null,
    });
    expect(getWatchRootValidation('~/Documents')).toEqual({
      isValid: true,
      errorKey: null,
    });
  });
});
