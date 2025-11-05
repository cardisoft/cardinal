import { describe, expect, it } from 'vitest';
import { formatKB } from '../format';

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
