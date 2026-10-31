import { act, renderHook } from '@testing-library/react';
import { beforeEach, describe, expect, it } from 'vitest';
import { useFontSize } from '../useFontSize';
import {
  FONT_SIZE_DEFAULT,
  FONT_SIZE_MAX,
  FONT_SIZE_MIN,
  rowHeightForFontSize,
} from '../../types/fontSize';

const renderFontSize = () => renderHook(() => useFontSize());

describe('useFontSize', () => {
  beforeEach(() => {
    window.localStorage.clear();
    document.documentElement.style.removeProperty('--app-font-size');
  });

  it('defaults to the base size and exposes it as a CSS variable', () => {
    const { result } = renderFontSize();

    expect(result.current.fontSize).toBe(FONT_SIZE_DEFAULT);
    expect(document.documentElement.style.getPropertyValue('--app-font-size')).toBe(
      `${FONT_SIZE_DEFAULT}px`,
    );
  });

  it('updates the CSS variable and row height when the size changes', () => {
    const { result } = renderFontSize();

    act(() => {
      result.current.setFontSize(18);
    });

    expect(result.current.fontSize).toBe(18);
    expect(document.documentElement.style.getPropertyValue('--app-font-size')).toBe('18px');
    expect(result.current.rowHeight).toBe(rowHeightForFontSize(18));
  });

  it('clamps out-of-range values', () => {
    const { result } = renderFontSize();

    act(() => {
      result.current.setFontSize(200);
    });
    expect(result.current.fontSize).toBe(FONT_SIZE_MAX);

    act(() => {
      result.current.setFontSize(1);
    });
    expect(result.current.fontSize).toBe(FONT_SIZE_MIN);
  });

  it('resets back to the default', () => {
    const { result } = renderFontSize();

    act(() => {
      result.current.setFontSize(19);
    });
    act(() => {
      result.current.resetFontSize();
    });

    expect(result.current.fontSize).toBe(FONT_SIZE_DEFAULT);
  });
});
