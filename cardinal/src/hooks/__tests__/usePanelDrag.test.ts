import { act, renderHook } from '@testing-library/react';
import { beforeEach, describe, expect, it } from 'vitest';
import { MIN_PANEL_HEIGHT, usePanelDrag } from '../usePanelDrag';
import { createRef } from 'react';

const buildPanelRef = () => {
  const panel = document.createElement('div');
  // jsdom has no layout; provide the geometry the hook reads.
  panel.getBoundingClientRect = () =>
    ({
      left: 400,
      top: 64,
      right: 900,
      bottom: 500,
      width: 500,
      height: 436,
      x: 400,
      y: 64,
    }) as DOMRect;
  Object.defineProperty(panel, 'offsetWidth', { configurable: true, value: 500 });
  Object.defineProperty(panel, 'offsetHeight', { configurable: true, value: 436 });
  document.body.appendChild(panel);
  const ref = createRef<HTMLDivElement>();
  (ref as { current: HTMLDivElement | null }).current = panel;
  return { ref, panel };
};

const pointer = (type: string, x: number, y: number, extra: Record<string, unknown> = {}) =>
  ({
    type,
    clientX: x,
    clientY: y,
    pointerId: 1,
    target: document.body,
    currentTarget: {
      setPointerCapture: () => {},
      releasePointerCapture: () => {},
      hasPointerCapture: () => true,
    },
    preventDefault: () => {},
    stopPropagation: () => {},
    ...extra,
  }) as unknown as React.PointerEvent<HTMLElement>;

describe('usePanelDrag', () => {
  beforeEach(() => {
    window.localStorage.clear();
    document.body.innerHTML = '';
  });

  it('starts anchored and reports the default position', () => {
    const { ref } = buildPanelRef();
    const { result } = renderHook(() => usePanelDrag(ref));
    expect(result.current.position).toBeNull();
  });

  it('moves with the pointer and persists the dropped position', () => {
    const { ref } = buildPanelRef();
    const { result } = renderHook(() => usePanelDrag(ref));

    act(() => {
      result.current.dragHandleProps.onPointerDown(pointer('pointerdown', 420, 80));
    });
    expect(result.current.position).toEqual({ x: 400, y: 64 });

    act(() => {
      result.current.dragHandleProps.onPointerMove(pointer('pointermove', 220, 180));
    });
    expect(result.current.position).toEqual({ x: 200, y: 164 });

    act(() => {
      result.current.dragHandleProps.onPointerUp(pointer('pointerup', 220, 180));
    });

    const stored = JSON.parse(window.localStorage.getItem('cardinal.bookmarks.panelPosition.v1')!);
    expect(stored).toEqual({ x: 200, y: 164 });
  });

  it('restores a stored position on the next mount', () => {
    window.localStorage.setItem(
      'cardinal.bookmarks.panelPosition.v1',
      JSON.stringify({ x: 120, y: 90 }),
    );
    const { ref } = buildPanelRef();
    const { result } = renderHook(() => usePanelDrag(ref));
    expect(result.current.position).toEqual({ x: 120, y: 90 });
  });

  it('ignores drags that start on header buttons', () => {
    const { ref } = buildPanelRef();
    const { result } = renderHook(() => usePanelDrag(ref));

    const button = document.createElement('button');
    act(() => {
      result.current.dragHandleProps.onPointerDown(
        pointer('pointerdown', 420, 80, { target: button }),
      );
    });

    expect(result.current.position).toBeNull();
  });

  it('clamps an off-screen stored position back into the viewport', () => {
    // A position saved from a larger window must never leave the panel unreachable.
    window.localStorage.setItem(
      'cardinal.bookmarks.panelPosition.v1',
      JSON.stringify({ x: 100_000, y: 100_000 }),
    );
    const { ref } = buildPanelRef();
    const { result } = renderHook(() => usePanelDrag(ref));

    expect(result.current.position!.x).toBeLessThanOrEqual(window.innerWidth - 500 - 8);
    expect(result.current.position!.y).toBeLessThanOrEqual(window.innerHeight - 436 - 8);
  });
});

describe('usePanelDrag resizing', () => {
  beforeEach(() => {
    window.localStorage.clear();
    document.body.innerHTML = '';
  });

  it('grows the width when the left edge is dragged left', () => {
    const { ref } = buildPanelRef();
    const { result } = renderHook(() => usePanelDrag(ref));

    act(() => {
      result.current.resizeHandleProps('left').onPointerDown(pointer('pointerdown', 400, 200));
    });
    act(() => {
      result.current.resizeHandleProps('left').onPointerMove(pointer('pointermove', 320, 200));
    });

    expect(result.current.size).toEqual({ width: 580, height: 436 });
    // The right edge stays put, so the left coordinate moves with the width.
    expect(result.current.position).toEqual({ x: 320, y: 64 });
  });

  it('grows the height when the bottom edge is dragged down', () => {
    const { ref } = buildPanelRef();
    const { result } = renderHook(() => usePanelDrag(ref));

    act(() => {
      result.current.resizeHandleProps('bottom').onPointerDown(pointer('pointerdown', 600, 500));
    });
    act(() => {
      result.current.resizeHandleProps('bottom').onPointerMove(pointer('pointermove', 600, 560));
    });

    expect(result.current.size).toEqual({ width: 500, height: 496 });
  });

  it('persists the size so it survives a reopen', () => {
    const { ref } = buildPanelRef();
    const { result } = renderHook(() => usePanelDrag(ref));

    act(() => {
      result.current.resizeHandleProps('bottom').onPointerDown(pointer('pointerdown', 600, 500));
    });
    act(() => {
      result.current.resizeHandleProps('bottom').onPointerMove(pointer('pointermove', 600, 540));
    });
    act(() => {
      result.current.resizeHandleProps('bottom').onPointerUp(pointer('pointerup', 600, 540));
    });

    const stored = JSON.parse(window.localStorage.getItem('cardinal.bookmarks.panelSize.v1')!);
    expect(stored).toEqual({ width: 500, height: 476 });
  });

  it('never shrinks below the usable minimum', () => {
    const { ref } = buildPanelRef();
    const { result } = renderHook(() => usePanelDrag(ref));

    act(() => {
      result.current.resizeHandleProps('bottom').onPointerDown(pointer('pointerdown', 600, 500));
    });
    act(() => {
      result.current.resizeHandleProps('bottom').onPointerMove(pointer('pointermove', 600, -900));
    });

    expect(result.current.size!.height).toBe(MIN_PANEL_HEIGHT);
  });

  it('a corner handle adjusts both edges at once', () => {
    const { ref } = buildPanelRef();
    const { result } = renderHook(() => usePanelDrag(ref));

    // Drag the bottom-right corner 40px right and 30px down.
    act(() => {
      result.current
        .resizeHandleProps('bottomRight')
        .onPointerDown(pointer('pointerdown', 900, 500));
    });
    act(() => {
      result.current
        .resizeHandleProps('bottomRight')
        .onPointerMove(pointer('pointermove', 940, 530));
    });

    expect(result.current.size).toEqual({ width: 540, height: 466 });
    // Growing from the south-east corner leaves the origin untouched.
    expect(result.current.position).toEqual({ x: 400, y: 64 });
  });

  it('the top-left corner grows up and to the left', () => {
    const { ref } = buildPanelRef();
    const { result } = renderHook(() => usePanelDrag(ref));

    act(() => {
      result.current.resizeHandleProps('topLeft').onPointerDown(pointer('pointerdown', 400, 64));
    });
    act(() => {
      result.current.resizeHandleProps('topLeft').onPointerMove(pointer('pointermove', 370, 34));
    });

    expect(result.current.size).toEqual({ width: 530, height: 466 });
    // Both edges moved, so the origin follows the pointer.
    expect(result.current.position).toEqual({ x: 370, y: 34 });
  });

  it('a single edge only changes its own dimension', () => {
    const { ref } = buildPanelRef();
    const { result } = renderHook(() => usePanelDrag(ref));

    act(() => {
      result.current.resizeHandleProps('right').onPointerDown(pointer('pointerdown', 900, 300));
    });
    act(() => {
      result.current.resizeHandleProps('right').onPointerMove(pointer('pointermove', 860, 380));
    });

    // Horizontal move shrinks the width; the vertical component is ignored.
    expect(result.current.size).toEqual({ width: 460, height: 436 });
    expect(result.current.position).toEqual({ x: 400, y: 64 });
  });

  it('the top edge only changes the height and moves the origin', () => {
    const { ref } = buildPanelRef();
    const { result } = renderHook(() => usePanelDrag(ref));

    act(() => {
      result.current.resizeHandleProps('top').onPointerDown(pointer('pointerdown', 600, 64));
    });
    act(() => {
      result.current.resizeHandleProps('top').onPointerMove(pointer('pointermove', 600, 24));
    });

    expect(result.current.size).toEqual({ width: 500, height: 476 });
    expect(result.current.position).toEqual({ x: 400, y: 24 });
  });
});
