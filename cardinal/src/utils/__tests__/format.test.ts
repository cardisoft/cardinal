import { describe, expect, it } from 'vitest';
import { formatKB, formatSize } from '../format';

describe('formatKB', () => {
  it('formats whole kilobytes without decimal digits', () => {
    expect(formatKB(2048)).toBe('2.0 KB');
  });

  it('formats small values with a single decimal place', () => {
    expect(formatKB(1536)).toBe('1.5 KB');
  });

  it('returns null for nullish or non-finite inputs', () => {
    expect(formatKB(null)).toBeNull();
    expect(formatKB(undefined)).toBeNull();
    expect(formatKB(Number.POSITIVE_INFINITY)).toBeNull();
  });
});

describe('formatSize', () => {
  it.each([
    [0, '0B'],
    [512, '512B'],
    [4096, '4KB'],
    [50 * 1024 * 1024, '50MB'],
    [200 * 1024 * 1024, '200MB'],
    [1024 * 1024 * 1024, '1G'],
    [1.24 * 1024 ** 3, '1.24G'],
    [15.5 * 1024 ** 3, '16G'],
    [2 * 1024 ** 4, '2T'],
  ])('formats %i bytes as %s', (bytes, expected) => {
    expect(formatSize(bytes)).toBe(expected);
  });

  it('returns null for unusable input', () => {
    expect(formatSize(null)).toBeNull();
    expect(formatSize(undefined)).toBeNull();
    expect(formatSize(Number.NaN)).toBeNull();
    expect(formatSize(-1)).toBeNull();
  });
});
