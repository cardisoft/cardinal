import { describe, expect, it } from 'vitest';
import { render, screen } from '@testing-library/react';
import React from 'react';

import { MiddleEllipsisHighlight, splitTextWithHighlights } from './MiddleEllipsisHighlight';

describe('splitTextWithHighlights', () => {
  it('returns entire string as plain text when no needles are provided', () => {
    expect(splitTextWithHighlights('foo', undefined)).toEqual([
      { text: 'foo', isHighlight: false },
    ]);
    expect(splitTextWithHighlights('foo', [])).toEqual([{ text: 'foo', isHighlight: false }]);
  });

  it('splits around simple highlight matches', () => {
    expect(splitTextWithHighlights('foo bar baz', ['bar'])).toEqual([
      { text: 'foo ', isHighlight: false },
      { text: 'bar', isHighlight: true },
      { text: ' baz', isHighlight: false },
    ]);
  });

  it('merges overlapping highlight spans automatically', () => {
    expect(splitTextWithHighlights('foobarbaz', ['foo', 'ooba', 'bar'])).toEqual([
      { text: 'foobar', isHighlight: true },
      { text: 'baz', isHighlight: false },
    ]);
  });

  it('respects case sensitivity flags', () => {
    expect(splitTextWithHighlights('AlphaBeta', ['alpha'], { caseInsensitive: true })).toEqual([
      { text: 'Alpha', isHighlight: true },
      { text: 'Beta', isHighlight: false },
    ]);

    expect(splitTextWithHighlights('AlphaBeta', ['alpha'], { caseInsensitive: false })).toEqual([
      { text: 'AlphaBeta', isHighlight: false },
    ]);
  });

  it('handles multiple non-overlapping matches', () => {
    expect(splitTextWithHighlights('abc abc abc', ['abc'])).toEqual([
      { text: 'abc', isHighlight: true },
      { text: ' ', isHighlight: false },
      { text: 'abc', isHighlight: true },
      { text: ' ', isHighlight: false },
      { text: 'abc', isHighlight: true },
    ]);
  });
});

describe('MiddleEllipsisHighlight', () => {
  it('uses layout measurement before the rendered text is observed', () => {
    const originalGetBoundingClientRect = HTMLElement.prototype.getBoundingClientRect;
    HTMLElement.prototype.getBoundingClientRect = () =>
      ({
        width: 80,
        height: 20,
        top: 0,
        right: 80,
        bottom: 20,
        left: 0,
        x: 0,
        y: 0,
        toJSON: () => ({}),
      }) as DOMRect;

    try {
      render(
        React.createElement(MiddleEllipsisHighlight, {
          text: 'abcdefghijklmnopqrstuvwxyz',
          className: 'test-middle-ellipsis',
        }),
      );

      const rendered = screen.getByTitle('abcdefghijklmnopqrstuvwxyz');
      expect(rendered.textContent).toContain('…');
      expect(rendered.textContent).not.toBe('abcdefghijklmnopqrstuvwxyz');
    } finally {
      HTMLElement.prototype.getBoundingClientRect = originalGetBoundingClientRect;
    }
  });
});
