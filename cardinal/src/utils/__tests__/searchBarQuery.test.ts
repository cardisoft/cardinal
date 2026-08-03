import { describe, expect, it } from 'vitest';
import {
  CUSTOM,
  readContentTerm,
  readFileType,
  setContentTerm,
  setFileType,
} from '../searchBarQuery';

describe('file type', () => {
  it('reads the token the dropdown writes', () => {
    expect(readFileType('informe type:image')).toBe('image');
    expect(readFileType('TYPE:IMAGE')).toBe('image');
    expect(readFileType('informe')).toBe('');
  });

  it('round-trips the multi-group documents option', () => {
    const query = setFileType('informe', 'document');
    expect(query).toBe('informe (type:doc|type:pdf|type:presentation|type:spreadsheet)');
    expect(readFileType(query)).toBe('document');
  });

  it('reports custom when it cannot represent the query faithfully', () => {
    expect(readFileType('informe !type:image')).toBe(CUSTOM);
    expect(readFileType('type:image type:video')).toBe(CUSTOM);
    expect(readFileType('(type:image | *.png)')).toBe(CUSTOM);
    expect(readFileType('type:unknownthing')).toBe(CUSTOM);
  });

  it('ignores a type: that is part of a quoted value, not a filter', () => {
    expect(readFileType('content:"type:image"')).toBe('');
  });

  it('replaces its own token, including a hand-written bare alias', () => {
    expect(setFileType('informe type:image', 'video')).toBe('informe type:video');
    expect(setFileType('informe type:pdf', 'image')).toBe('informe type:image');
    expect(setFileType('informe (type:doc|type:pdf|type:presentation|type:spreadsheet)', '')).toBe(
      'informe',
    );
    expect(setFileType('', 'email')).toBe('type:email');
  });

  it('keeps tokens it did not write', () => {
    expect(setFileType('!type:image informe', 'pdf' as never)).toBe('!type:image informe');
  });
});

describe('contains', () => {
  it('round-trips a phrase, spaces and all', () => {
    const query = setContentTerm('informe', 'Bearer token');
    expect(query).toBe('informe content:"Bearer token"');
    expect(readContentTerm(query)).toBe('Bearer token');
  });

  it('keeps a trailing space, which the engine searches for', () => {
    expect(readContentTerm('content:"Bearer "')).toBe('Bearer ');
  });

  it('escapes quotes so the token cannot be broken out of', () => {
    const query = setContentTerm('', 'say "hi"');
    expect(query).toBe('content:"say \\"hi\\""');
    expect(readContentTerm(query)).toBe('say "hi"');
  });

  it('replaces rather than stacks, and clears on empty', () => {
    expect(setContentTerm('a content:"one" b', 'two')).toBe('a b content:"two"');
    expect(setContentTerm('a content:"one"', '   ')).toBe('a');
  });

  it('reports custom for a content filter it did not write', () => {
    expect(readContentTerm('!content:"draft"')).toBe(CUSTOM);
    expect(readContentTerm('content:"a" content:"b"')).toBe(CUSTOM);
    expect(readContentTerm('informe')).toBe('');
  });
});
