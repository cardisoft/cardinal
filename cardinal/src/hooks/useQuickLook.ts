import { useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { currentMonitor, getCurrentWindow, primaryMonitor } from '@tauri-apps/api/window';

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
 * It resolves window geometry when needed and translates DOM rects
 * into screen coordinates suitable for macOS' Quick Look APIs.
 */
export const useQuickLook = ({ getPaths }: UseQuickLookConfig) => {
  const resolveWindowGeometry = useCallback(async () => {
    try {
      const currentWindow = getCurrentWindow();
      const [position, scaleFactor, monitor] = await Promise.all([
        currentWindow.innerPosition(),
        currentWindow.scaleFactor(),
        currentMonitor(),
      ]);
      const resolvedMonitor = monitor ?? (await primaryMonitor());

      if (!resolvedMonitor) {
        return null;
      }

      const scale = scaleFactor || resolvedMonitor.scaleFactor || window.devicePixelRatio || 1;
      return {
        windowOrigin: {
          x: position.x / scale,
          y: position.y / scale,
        },
        screenHeight: resolvedMonitor.size.height / scale,
      };
    } catch (error) {
      console.warn('Failed to resolve window metrics for Quick Look', error);
      return null;
    }
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

    // This compensates for a coordinate system mismatch on macOS:
    // - `geometry.windowOrigin.y` (from Tauri's `innerPosition`) is relative to the *visible* screen area (below the menu bar).
    // - `geometry.screenHeight` is the *full* height of the monitor hosting the window.
    // - `window.screen.availTop` provides the height of the menu bar, allowing us to correctly adjust `logicalYTop`
    //   to be relative to the absolute top of the screen for `QLPreviewPanel`'s `sourceFrameOnScreenForPreviewItem`.
    const screenTopOffset = window.screen.availTop ?? 0;

    const buildItem = (path: string): QuickLookItemPayload => {
      const selector = `[data-row-path="${escapePathForSelector(path)}"]`;
      const row = document.querySelector<HTMLElement>(selector);
      const anchor = row?.querySelector<HTMLElement>('.file-icon, .file-icon-placeholder');
      const iconImage = row?.querySelector<HTMLImageElement>('img.file-icon');
      if (!row || !anchor || !iconImage) {
        return { path };
      }

      const rect = anchor.getBoundingClientRect();
      const logicalX = geometry.windowOrigin.x + rect.left;
      const logicalYTop = geometry.windowOrigin.y + screenTopOffset + rect.top;
      const logicalWidth = rect.width;
      const logicalHeight = rect.height;

      const x = logicalX;
      const y = geometry.screenHeight - (logicalYTop + logicalHeight);

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
