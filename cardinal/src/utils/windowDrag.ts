import { getCurrentWindow } from '@tauri-apps/api/window';

/**
 * Starts dragging the current OS window from a pointer-down on a custom header.
 *
 * `data-tauri-drag-region` only reacts on the exact element that carries it, so a header with
 * buttons and titles needs an explicit call instead.
 */
export const startNativeWindowDrag = async (): Promise<void> => {
  try {
    await getCurrentWindow().startDragging();
  } catch (error) {
    console.warn('Unable to start window drag', error);
  }
};
