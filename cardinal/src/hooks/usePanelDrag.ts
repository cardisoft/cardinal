import { useCallback, useEffect, useLayoutEffect, useRef, useState } from 'react';

export type PanelPosition = { x: number; y: number };
export type PanelSize = { width: number; height: number };

const POSITION_KEY = 'cardinal.bookmarks.panelPosition.v1';
const SIZE_KEY = 'cardinal.bookmarks.panelSize.v1';
const EDGE_MARGIN = 8;
/** The panel must stay usable: below this the bookmark editor can no longer lay out. */
export const MIN_PANEL_WIDTH = 420;
export const MIN_PANEL_HEIGHT = 260;

const readStoredPosition = (): PanelPosition | null => {
  if (typeof window === 'undefined') {
    return null;
  }
  try {
    const raw = window.localStorage.getItem(POSITION_KEY);
    if (!raw) {
      return null;
    }
    const parsed = JSON.parse(raw) as Partial<PanelPosition>;
    if (typeof parsed.x !== 'number' || typeof parsed.y !== 'number') {
      return null;
    }
    return { x: parsed.x, y: parsed.y };
  } catch {
    return null;
  }
};

const readStoredSize = (): PanelSize | null => {
  if (typeof window === 'undefined') {
    return null;
  }
  try {
    const raw = window.localStorage.getItem(SIZE_KEY);
    if (!raw) {
      return null;
    }
    const parsed = JSON.parse(raw) as Partial<PanelSize>;
    if (typeof parsed.width !== 'number' || typeof parsed.height !== 'number') {
      return null;
    }
    return { width: parsed.width, height: parsed.height };
  } catch {
    return null;
  }
};

const persist = (key: string, value: unknown): void => {
  try {
    window.localStorage.setItem(key, JSON.stringify(value));
  } catch {
    // Storage failures are not actionable; the panel still behaves correctly this session.
  }
};

const clampToViewport = (
  position: PanelPosition,
  width: number,
  height: number,
): PanelPosition => ({
  x: Math.min(
    Math.max(EDGE_MARGIN, position.x),
    Math.max(EDGE_MARGIN, window.innerWidth - width - EDGE_MARGIN),
  ),
  y: Math.min(
    Math.max(EDGE_MARGIN, position.y),
    Math.max(EDGE_MARGIN, window.innerHeight - height - EDGE_MARGIN),
  ),
});

const clampSize = (size: PanelSize): PanelSize => ({
  width: Math.min(
    Math.max(MIN_PANEL_WIDTH, size.width),
    Math.max(MIN_PANEL_WIDTH, window.innerWidth - EDGE_MARGIN * 2),
  ),
  height: Math.min(
    Math.max(MIN_PANEL_HEIGHT, size.height),
    Math.max(MIN_PANEL_HEIGHT, window.innerHeight - EDGE_MARGIN * 2),
  ),
});

type DragState = { pointerId: number; offsetX: number; offsetY: number };
/** Which edges the pointer grabs: `n`/`s` change the height, `e`/`w` the width. */
export type ResizeEdge = 'n' | 's' | 'e' | 'w' | 'ne' | 'nw' | 'se' | 'sw';

/** The handle vocabulary exposed to the panel, indexed by edge combination. */
export type ResizeHandleName =
  | 'top'
  | 'bottom'
  | 'left'
  | 'right'
  | 'topLeft'
  | 'topRight'
  | 'bottomLeft'
  | 'bottomRight';

const HANDLE_EDGES: Record<ResizeHandleName, ResizeEdge> = {
  top: 'n',
  bottom: 's',
  left: 'w',
  right: 'e',
  topLeft: 'nw',
  topRight: 'ne',
  bottomLeft: 'sw',
  bottomRight: 'se',
};

type ResizeState = {
  pointerId: number;
  edge: ResizeEdge;
  startX: number;
  startY: number;
  startWidth: number;
  startHeight: number;
  /** Panel box at pointer-down, in viewport coordinates. */
  startLeft: number;
  startTop: number;
};

/**
 * Drag-to-move and drag-to-resize support for a floating panel.
 *
 * Returns inline style values to apply and handlers to spread on the drag handle (header) and
 * the resize handles (left / bottom edges).
 */
export function usePanelDrag(panelRef: React.RefObject<HTMLElement | null>) {
  const [position, setPosition] = useState<PanelPosition | null>(() => readStoredPosition());
  const [size, setSize] = useState<PanelSize | null>(() => readStoredSize());
  const dragRef = useRef<DragState | null>(null);
  const resizeRef = useRef<ResizeState | null>(null);

  const handlePointerDown = useCallback(
    (event: React.PointerEvent<HTMLElement>) => {
      // Let header buttons/inputs keep their normal behavior.
      const target = event.target as HTMLElement;
      if (target.closest('button, input, select, textarea')) {
        return;
      }
      const panel = panelRef.current;
      if (!panel) {
        return;
      }
      const rect = panel.getBoundingClientRect();
      dragRef.current = {
        pointerId: event.pointerId,
        offsetX: event.clientX - rect.left,
        offsetY: event.clientY - rect.top,
      };
      setPosition({ x: rect.left, y: rect.top });
      event.currentTarget.setPointerCapture(event.pointerId);
      event.preventDefault();
    },
    [panelRef],
  );

  const handlePointerMove = useCallback(
    (event: React.PointerEvent<HTMLElement>) => {
      const drag = dragRef.current;
      const panel = panelRef.current;
      if (!drag || !panel || drag.pointerId !== event.pointerId) {
        return;
      }
      setPosition(
        clampToViewport(
          { x: event.clientX - drag.offsetX, y: event.clientY - drag.offsetY },
          panel.offsetWidth,
          panel.offsetHeight,
        ),
      );
    },
    [panelRef],
  );

  const endDrag = useCallback((event: React.PointerEvent<HTMLElement>) => {
    if (!dragRef.current) {
      return;
    }
    dragRef.current = null;
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
    setPosition((current) => {
      if (current) {
        persist(POSITION_KEY, current);
      }
      return current;
    });
  }, []);

  const startResize = useCallback(
    (handle: ResizeHandleName) => (event: React.PointerEvent<HTMLElement>) => {
      const edge = HANDLE_EDGES[handle];
      const panel = panelRef.current;
      if (!panel) {
        return;
      }
      const rect = panel.getBoundingClientRect();
      resizeRef.current = {
        pointerId: event.pointerId,
        edge,
        startX: event.clientX,
        startY: event.clientY,
        startWidth: rect.width,
        startHeight: rect.height,
        startLeft: rect.left,
        startTop: rect.top,
      };
      // Resizing anchors the panel in viewport coordinates so the opposite edge stays put.
      setPosition({ x: rect.left, y: rect.top });
      setSize({ width: rect.width, height: rect.height });
      event.currentTarget.setPointerCapture(event.pointerId);
      event.preventDefault();
      event.stopPropagation();
    },
    [panelRef],
  );

  const handleResizeMove = useCallback((event: React.PointerEvent<HTMLElement>) => {
    const resize = resizeRef.current;
    if (!resize || resize.pointerId !== event.pointerId) {
      return;
    }

    const dx = event.clientX - resize.startX;
    const dy = event.clientY - resize.startY;
    const growsFromWest = resize.edge.includes('w');
    const growsFromEast = resize.edge.includes('e');
    const growsFromNorth = resize.edge.includes('n');
    const growsFromSouth = resize.edge.includes('s');

    // Each edge moves independently; a corner handle simply sets two of these flags, which is
    // why both dimensions follow the pointer diagonally.
    const rawWidth = growsFromWest
      ? resize.startWidth - dx
      : growsFromEast
        ? resize.startWidth + dx
        : resize.startWidth;
    const rawHeight = growsFromNorth
      ? resize.startHeight - dy
      : growsFromSouth
        ? resize.startHeight + dy
        : resize.startHeight;

    const next = clampSize({ width: rawWidth, height: rawHeight });
    setSize(next);

    // Only the anchored-away edge moves: dragging a west/north handle shifts the origin so
    // the opposite edge stays exactly where it was.
    setPosition({
      x: growsFromWest ? resize.startLeft + (resize.startWidth - next.width) : resize.startLeft,
      y: growsFromNorth ? resize.startTop + (resize.startHeight - next.height) : resize.startTop,
    });
  }, []);

  const endResize = useCallback((event: React.PointerEvent<HTMLElement>) => {
    if (!resizeRef.current) {
      return;
    }
    resizeRef.current = null;
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
    setSize((current) => {
      if (current) {
        persist(SIZE_KEY, current);
      }
      return current;
    });
    setPosition((current) => {
      if (current) {
        persist(POSITION_KEY, current);
      }
      return current;
    });
  }, []);

  // A stored position may come from a larger window or an older session; clamp it on mount
  // so the panel can never be restored entirely off-screen.
  useLayoutEffect(() => {
    const panel = panelRef.current;
    if (!panel) {
      return;
    }
    setSize((current) => (current ? clampSize(current) : current));
    setPosition((current) =>
      current ? clampToViewport(current, panel.offsetWidth, panel.offsetHeight) : current,
    );
  }, []);

  // Keep the panel inside the viewport when the window shrinks.
  useEffect(() => {
    if (!position && !size) {
      return;
    }
    const handleResize = () => {
      const panel = panelRef.current;
      if (!panel) {
        return;
      }
      setSize((current) => (current ? clampSize(current) : current));
      setPosition((current) =>
        current ? clampToViewport(current, panel.offsetWidth, panel.offsetHeight) : current,
      );
    };
    window.addEventListener('resize', handleResize);
    return () => window.removeEventListener('resize', handleResize);
  }, [panelRef, position, size]);

  return {
    position,
    size,
    dragHandleProps: {
      onPointerDown: handlePointerDown,
      onPointerMove: handlePointerMove,
      onPointerUp: endDrag,
      onPointerCancel: endDrag,
    },
    /** Pointer handlers for one of the eight resize handles. */
    resizeHandleProps: (handle: ResizeHandleName) => ({
      onPointerDown: startResize(handle),
      onPointerMove: handleResizeMove,
      onPointerUp: endResize,
      onPointerCancel: endResize,
    }),
  };
}
