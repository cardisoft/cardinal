import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useAppWindowListeners } from '../useAppWindowListeners';

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(),
}));

vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: vi.fn(),
}));

const mockedListen = vi.mocked(listen);
const mockedGetCurrentWindow = vi.mocked(getCurrentWindow);

type HookProps = {
  activeTab: 'files' | 'events';
  focusSearchInput: () => void;
  handleStatusUpdate: (scannedFiles: number, processedEvents: number, rescanErrors: number) => void;
  setLifecycleState: (status: 'Initializing' | 'Updating' | 'Ready') => void;
  queueSearch: (
    query: string,
    options?: { immediate?: boolean; onSearchCommitted?: (query: string) => void },
  ) => void;
  setEventFilterQuery: (query: string) => void;
  updateHistoryFromInput: (query: string) => void;
};

describe('useAppWindowListeners', () => {
  const statusUnlisten = vi.fn();
  const lifecycleUnlisten = vi.fn();
  const quickLaunchUnlisten = vi.fn();
  const dragDropUnlisten = vi.fn();

  const focusSearchInput = vi.fn();
  const handleStatusUpdate = vi.fn();
  const setLifecycleState = vi.fn();
  const queueSearch = vi.fn();
  const setEventFilterQuery = vi.fn();
  const updateHistoryFromInput = vi.fn();

  let tauriListeners: Record<string, (event: any) => void>;
  let dragDropListener: ((event: any) => void) | null;

  const renderWindowListeners = (overrides: Partial<HookProps> = {}) =>
    renderHook((props: HookProps) => useAppWindowListeners(props), {
      initialProps: {
        activeTab: 'files',
        focusSearchInput,
        handleStatusUpdate,
        setLifecycleState,
        queueSearch,
        setEventFilterQuery,
        updateHistoryFromInput,
        ...overrides,
      },
    });

  beforeEach(() => {
    vi.clearAllMocks();
    tauriListeners = {};
    dragDropListener = null;
    document.documentElement.removeAttribute('data-window-focused');

    mockedListen.mockImplementation(async (eventName: string, callback: (event: any) => void) => {
      tauriListeners[eventName] = callback;
      if (eventName === 'status_bar_update') return statusUnlisten;
      if (eventName === 'app_lifecycle_state') return lifecycleUnlisten;
      if (eventName === 'quick_launch') return quickLaunchUnlisten;
      return vi.fn();
    });

    mockedGetCurrentWindow.mockReturnValue({
      onDragDropEvent: vi.fn(async (callback: (event: any) => void) => {
        dragDropListener = callback;
        return dragDropUnlisten;
      }),
    } as unknown as ReturnType<typeof getCurrentWindow>);
  });

  it('subscribes to tauri events and dispatches payloads to handlers', async () => {
    renderWindowListeners();

    await waitFor(() => {
      expect(mockedListen).toHaveBeenCalledTimes(3);
    });

    act(() => {
      tauriListeners.status_bar_update?.({
        payload: { scannedFiles: 11, processedEvents: 22, rescanErrors: 3 },
      });
    });
    expect(handleStatusUpdate).toHaveBeenCalledWith(11, 22, 3);

    act(() => {
      tauriListeners.app_lifecycle_state?.({ payload: 'Ready' });
    });
    expect(setLifecycleState).toHaveBeenCalledWith('Ready');

    act(() => {
      tauriListeners.quick_launch?.({});
    });
    expect(focusSearchInput).toHaveBeenCalledTimes(1);
  });

  it('handles drag-drop search routing for files and events tabs', async () => {
    const { rerender } = renderWindowListeners();

    await waitFor(() => {
      expect(dragDropListener).not.toBeNull();
    });

    act(() => {
      dragDropListener?.({
        payload: { type: 'drop', paths: [' /tmp/file-a '] },
      });
    });
    expect(queueSearch).toHaveBeenCalledWith('"/tmp/file-a"', {
      immediate: true,
      onSearchCommitted: updateHistoryFromInput,
    });

    rerender({
      activeTab: 'events',
      focusSearchInput,
      handleStatusUpdate,
      setLifecycleState,
      queueSearch,
      setEventFilterQuery,
      updateHistoryFromInput,
    });

    act(() => {
      dragDropListener?.({
        payload: { type: 'drop', paths: [' /tmp/file-b '] },
      });
    });
    expect(setEventFilterQuery).toHaveBeenCalledWith('"/tmp/file-b"');
  });

  it('syncs window focus attribute and cleans up listeners on unmount', async () => {
    const { unmount } = renderWindowListeners();

    await waitFor(() => {
      expect(dragDropListener).not.toBeNull();
    });

    act(() => {
      window.dispatchEvent(new Event('blur'));
    });
    await waitFor(() => {
      expect(document.documentElement.dataset.windowFocused).toBe('false');
    });

    act(() => {
      window.dispatchEvent(new Event('focus'));
    });
    await waitFor(() => {
      expect(document.documentElement.dataset.windowFocused).toBe('true');
    });

    unmount();

    expect(statusUnlisten).toHaveBeenCalledTimes(1);
    expect(lifecycleUnlisten).toHaveBeenCalledTimes(1);
    expect(quickLaunchUnlisten).toHaveBeenCalledTimes(1);
    expect(dragDropUnlisten).toHaveBeenCalledTimes(1);
  });
});
