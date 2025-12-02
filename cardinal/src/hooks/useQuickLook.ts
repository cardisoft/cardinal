import { useCallback, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow, primaryMonitor } from '@tauri-apps/api/window';

type QuickLookRect = {
  x: number;
  y: number;
  width: number;
  height: number;
};

type QuickLookItemPayload = {
  path: string;
  rect?: QuickLookRect;
  transitionImage?: string;
};

const escapePathForSelector = (value: string): string => {
  return window.CSS.escape(value);
};

type UseQuickLookConfig = {
  getPaths: () => string[];
};

/**
 * Provides Quick Look helpers for the file list. Given a function that returns the currently
 * selected paths, the hook exposes memoized callbacks to toggle/update/close the Quick Look panel.
 * It caches window geometry so repeated lookups don't thrash Tauri APIs, and translates DOM rects
 * into screen coordinates suitable for macOS' Quick Look APIs.
 */
export const useQuickLook = ({ getPaths }: UseQuickLookConfig) => {
  const geometryCacheRef = useRef<{
    windowOrigin: { x: number; y: number };
    mainScreenHeight: number;
  } | null>(null);

  const resolveWindowGeometry = useCallback(async () => {
    if (geometryCacheRef.current !== null) {
      return geometryCacheRef.current;
    }

    if (typeof window === 'undefined') {
      geometryCacheRef.current = null;
      return geometryCacheRef.current;
    }

    try {
      const currentWindow = getCurrentWindow();
      const [position, monitor, scaleFactor] = await Promise.all([
        currentWindow.innerPosition(),
        primaryMonitor(),
        currentWindow.scaleFactor(),
      ]);

      if (!monitor) {
        geometryCacheRef.current = null;
        return geometryCacheRef.current;
      }

      const scale = scaleFactor || monitor.scaleFactor || window.devicePixelRatio || 1;
      geometryCacheRef.current = {
        windowOrigin: {
          x: position.x / scale,
          y: position.y / scale,
        },
        mainScreenHeight: monitor.size.height / scale,
      };
    } catch (error) {
      console.warn('Failed to resolve window metrics for Quick Look', error);
      geometryCacheRef.current = null;
    }

    return geometryCacheRef.current;
  }, []);

  const getQuickLookItems = useCallback(async (): Promise<QuickLookItemPayload[]> => {
    const paths = getPaths();
    if (!paths.length) {
      return [];
    }

    const geometry = await resolveWindowGeometry();
    if (!geometry) {
      return paths.map((path) => ({ path }));
    }

    const screenTopOffset = window.screen.availTop ?? 0;

    const buildItem = (path: string): QuickLookItemPayload => {
      const selector = `[data-row-path="${escapePathForSelector(path)}"]`;
      const row = document.querySelector<HTMLElement>(selector);
      if (!row) {
        return { path };
      }
      const anchor = row.querySelector<HTMLElement>('.file-icon, .file-icon-placeholder');
      if (!anchor) {
        return { path };
      }
      const iconImage = row.querySelector<HTMLImageElement>('img.file-icon');
      if (!iconImage) {
        return { path };
      }

      const rect = anchor.getBoundingClientRect();
      const logicalX = geometry.windowOrigin.x + rect.left;
      const logicalYTop = geometry.windowOrigin.y + screenTopOffset + rect.top;
      const logicalWidth = rect.width;
      const logicalHeight = rect.height;

      const x = logicalX;
      const y = geometry.mainScreenHeight - (logicalYTop + logicalHeight);

      return {
        path,
        rect: {
          x,
          y,
          width: logicalWidth,
          height: logicalHeight,
        },
        transitionImage: iconImage.src,
      };
    };

    return paths.map((path) => buildItem(path));
  }, [getPaths, resolveWindowGeometry]);

  const toggleQuickLook = useCallback(async () => {
    const items = await getQuickLookItems();
    if (!items.length) {
      return;
    }
    try {
      await invoke('toggle_quicklook', { items });
    } catch (error) {
      console.error('Failed to preview file with Quick Look', error);
    }
  }, [getQuickLookItems]);

  const updateQuickLook = useCallback(async () => {
    const items = await getQuickLookItems();
    if (!items.length) {
      return;
    }
    try {
      await invoke('update_quicklook', { items });
    } catch (error) {
      console.error('Failed to update Quick Look', error);
    }
  }, [getQuickLookItems]);

  const closeQuickLook = useCallback(() => {
    invoke('close_quicklook').catch((error) => {
      console.error('Failed to close Quick Look', error);
    });
  }, []);

  return {
    toggleQuickLook,
    updateQuickLook,
    closeQuickLook,
  };
};
