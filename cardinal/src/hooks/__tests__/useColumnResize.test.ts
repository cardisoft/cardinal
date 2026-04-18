import { act, renderHook } from '@testing-library/react';
import type { MouseEvent as ReactMouseEvent } from 'react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { CONTAINER_PADDING, MIN_COL_WIDTH, SCROLLBAR_WIDTH } from '../../constants';
import { useColumnResize } from '../useColumnResize';

const mocks = vi.hoisted(() => ({
  startColumnResizeDrag: vi.fn(),
}));

vi.mock('../resizeDrag', () => ({
  startColumnResizeDrag: mocks.startColumnResizeDrag,
}));

type ResizeDragOptions = {
  applyWidth: (nextWidth: number) => void;
};

const setWindowWidth = (width: number) => {
  Object.defineProperty(window, 'innerWidth', {
    value: width,
    writable: true,
    configurable: true,
  });
};

const createResizeStartEvent = (): ReactMouseEvent<HTMLSpanElement> =>
  ({
    currentTarget: document.createElement('span'),
    preventDefault: vi.fn(),
    stopPropagation: vi.fn(),
  }) as unknown as ReactMouseEvent<HTMLSpanElement>;

const dispatchWindowResize = () => {
  act(() => {
    window.dispatchEvent(new Event('resize'));
  });
};

const sumColumnWidths = (widths: ReturnType<typeof useColumnResize>['colWidths']) =>
  widths.filename + widths.path + widths.size + widths.modified + widths.created;

const fixedColumnWidth = (widths: ReturnType<typeof useColumnResize>['colWidths']) =>
  widths.path + widths.size + widths.modified + widths.created;

const getLastResizeDragOptions = () => {
  const calls = mocks.startColumnResizeDrag.mock.calls;
  return calls[calls.length - 1]?.[0] as ResizeDragOptions;
};

describe('useColumnResize', () => {
  beforeEach(() => {
    mocks.startColumnResizeDrag.mockClear();
  });

  afterEach(() => {
    setWindowWidth(1024);
  });

  it('shrinks the filename column in auto mode when the window shrinks', () => {
    setWindowWidth(1000);
    const { result } = renderHook(() => useColumnResize());
    const initialWidths = result.current.colWidths;

    setWindowWidth(800);
    dispatchWindowResize();

    const fixedWidth =
      initialWidths.path + initialWidths.size + initialWidths.modified + initialWidths.created;
    const availableWidth = window.innerWidth - CONTAINER_PADDING - SCROLLBAR_WIDTH;

    expect(result.current.colWidths).toEqual({
      ...initialWidths,
      filename: availableWidth - fixedWidth,
    });
  });

  it('expands the filename column to fill remaining space when columns fit the window', () => {
    setWindowWidth(1000);
    const { result } = renderHook(() => useColumnResize());
    const initialWidths = result.current.colWidths;

    setWindowWidth(1200);
    dispatchWindowResize();

    const fixedWidth =
      initialWidths.path + initialWidths.size + initialWidths.modified + initialWidths.created;
    const availableWidth = window.innerWidth - CONTAINER_PADDING - SCROLLBAR_WIDTH;

    expect(result.current.colWidths).toEqual({
      ...initialWidths,
      filename: availableWidth - fixedWidth,
    });
  });

  it('keeps all column widths independent in manual mode while the window resizes', () => {
    setWindowWidth(1000);
    const { result } = renderHook(() => useColumnResize());
    const initialWidths = result.current.colWidths;

    // Any direct column drag switches the hook into manual mode, where the
    // user's chosen widths are preserved across unrelated window resizes.
    act(() => {
      result.current.onResizeStart('path')(createResizeStartEvent());
      getLastResizeDragOptions().applyWidth(initialWidths.path + 50);
    });
    const manualWidths = result.current.colWidths;

    setWindowWidth(1500);
    dispatchWindowResize();
    expect(result.current.colWidths).toEqual(manualWidths);

    setWindowWidth(800);
    dispatchWindowResize();
    expect(result.current.colWidths).toEqual(manualWidths);
  });

  it('switches from manual back to auto when a window resize matches the total column width', () => {
    setWindowWidth(1000);
    const { result } = renderHook(() => useColumnResize());
    const initialWidths = result.current.colWidths;

    act(() => {
      result.current.onResizeStart('path')(createResizeStartEvent());
      getLastResizeDragOptions().applyWidth(initialWidths.path + 50);
    });

    const manualWidths = result.current.colWidths;
    const manualTotal = sumColumnWidths(manualWidths);
    // +3px is inside AUTO_SNAP_THRESHOLD, so a window resize should be treated
    // as returning the manual layout to a fitted, Finder-style auto layout.
    setWindowWidth(manualTotal + CONTAINER_PADDING + SCROLLBAR_WIDTH + 3);
    dispatchWindowResize();

    expect(result.current.colWidths.filename).toBe(manualWidths.filename + 3);

    // After snapping back to auto, later window resizes should again be absorbed
    // by the filename column.
    setWindowWidth(manualTotal + CONTAINER_PADDING + SCROLLBAR_WIDTH + 100);
    dispatchWindowResize();

    expect(result.current.colWidths.filename).toBe(manualWidths.filename + 100);
  });

  it('keeps manual mode when a window resize is outside the snap threshold', () => {
    setWindowWidth(1000);
    const { result } = renderHook(() => useColumnResize());
    const initialWidths = result.current.colWidths;

    act(() => {
      result.current.onResizeStart('path')(createResizeStartEvent());
      getLastResizeDragOptions().applyWidth(initialWidths.path + 50);
    });

    const manualWidths = result.current.colWidths;
    // +6px is just outside the 5px snap band; this preserves the manual layout
    // and verifies the boundary is intentional rather than any resize near total.
    setWindowWidth(sumColumnWidths(manualWidths) + CONTAINER_PADDING + SCROLLBAR_WIDTH + 6);
    dispatchWindowResize();

    expect(result.current.colWidths).toEqual(manualWidths);
  });

  it('keeps filename at its minimum width in auto mode when fixed columns overflow', () => {
    setWindowWidth(1000);
    const { result } = renderHook(() => useColumnResize());
    const initialWidths = result.current.colWidths;

    // Once fixed columns alone exceed available width, filename cannot absorb
    // any more shrinkage and should clamp at MIN_COL_WIDTH.
    setWindowWidth(fixedColumnWidth(initialWidths) + CONTAINER_PADDING + SCROLLBAR_WIDTH - 20);
    dispatchWindowResize();

    expect(result.current.colWidths).toEqual({
      ...initialWidths,
      filename: MIN_COL_WIDTH,
    });
  });

  it('switches to manual mode when the filename column is resized directly', () => {
    setWindowWidth(1000);
    const { result } = renderHook(() => useColumnResize());
    const initialWidths = result.current.colWidths;

    act(() => {
      result.current.onResizeStart('filename')(createResizeStartEvent());
      getLastResizeDragOptions().applyWidth(initialWidths.filename + 50);
    });
    const manualWidths = result.current.colWidths;

    setWindowWidth(1200);
    dispatchWindowResize();

    expect(result.current.colWidths).toEqual(manualWidths);
  });

  it('uses autoFitColumns to return to auto mode after manual resizing', () => {
    setWindowWidth(1000);
    const { result } = renderHook(() => useColumnResize());
    const initialWidths = result.current.colWidths;

    act(() => {
      result.current.onResizeStart('path')(createResizeStartEvent());
      getLastResizeDragOptions().applyWidth(initialWidths.path + 50);
    });

    setWindowWidth(1200);
    act(() => {
      // Header context-menu auto-fit is an explicit user action to return from
      // manual widths to the default auto filename layout.
      result.current.autoFitColumns();
    });
    const autoFitWidths = result.current.colWidths;

    setWindowWidth(1300);
    dispatchWindowResize();

    const fixedWidth = fixedColumnWidth(autoFitWidths);
    const availableWidth = window.innerWidth - CONTAINER_PADDING - SCROLLBAR_WIDTH;
    expect(result.current.colWidths).toEqual({
      ...autoFitWidths,
      filename: availableWidth - fixedWidth,
    });
  });
});
