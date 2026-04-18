import type { MouseEvent as ReactMouseEvent } from 'react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { startColumnResizeDrag } from '../resizeDrag';

const createResizeStartEvent = (
  element: HTMLSpanElement,
  clientX: number,
): ReactMouseEvent<HTMLSpanElement> =>
  ({
    clientX,
    currentTarget: element,
    preventDefault: vi.fn(),
    stopPropagation: vi.fn(),
  }) as unknown as ReactMouseEvent<HTMLSpanElement>;

describe('startColumnResizeDrag', () => {
  let animationFrameCallback: FrameRequestCallback | null = null;

  beforeEach(() => {
    animationFrameCallback = null;
    vi.spyOn(window, 'requestAnimationFrame').mockImplementation((callback) => {
      animationFrameCallback = callback;
      return 1;
    });
    vi.spyOn(window, 'cancelAnimationFrame').mockImplementation(() => {});
  });

  afterEach(() => {
    document.body.style.cursor = '';
    document.body.style.userSelect = '';
    vi.restoreAllMocks();
  });

  it('coalesces mousemove width updates into one animation frame', () => {
    const applyWidth = vi.fn();
    const resizer = document.createElement('span');

    startColumnResizeDrag({
      event: createResizeStartEvent(resizer, 100),
      startWidth: 200,
      clampWidth: (value) => value,
      applyWidth,
    });

    document.dispatchEvent(new MouseEvent('mousemove', { clientX: 110 }));
    document.dispatchEvent(new MouseEvent('mousemove', { clientX: 125 }));

    expect(applyWidth).not.toHaveBeenCalled();

    animationFrameCallback?.(performance.now());

    expect(applyWidth).toHaveBeenCalledTimes(1);
    expect(applyWidth).toHaveBeenCalledWith(225);
  });

  it('flushes the latest pending width on mouseup', () => {
    const applyWidth = vi.fn();
    const resizer = document.createElement('span');

    startColumnResizeDrag({
      event: createResizeStartEvent(resizer, 100),
      startWidth: 200,
      clampWidth: (value) => value,
      applyWidth,
    });

    document.dispatchEvent(new MouseEvent('mousemove', { clientX: 135 }));
    document.dispatchEvent(new MouseEvent('mouseup'));

    expect(window.cancelAnimationFrame).toHaveBeenCalledWith(1);
    expect(applyWidth).toHaveBeenCalledTimes(1);
    expect(applyWidth).toHaveBeenCalledWith(235);
  });
});
