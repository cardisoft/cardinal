import { act, renderHook, waitFor } from '@testing-library/react';
import type { RefObject } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import {
  subscribeLifecycleState,
  subscribeQuickLaunch,
  subscribeStatusBarUpdate,
  subscribeWindowDragDrop,
} from '../../runtime/tauriEventRuntime';
import { useAppWindowListeners } from '../useAppWindowListeners';

vi.mock('../../runtime/tauriEventRuntime', () => ({
  subscribeStatusBarUpdate: vi.fn(),
  subscribeLifecycleState: vi.fn(),
  subscribeQuickLaunch: vi.fn(),
  subscribeWindowDragDrop: vi.fn(),
}));

const mockedSubscribeStatusBarUpdate = vi.mocked(subscribeStatusBarUpdate);
const mockedSubscribeLifecycleState = vi.mocked(subscribeLifecycleState);
const mockedSubscribeQuickLaunch = vi.mocked(subscribeQuickLaunch);
const mockedSubscribeWindowDragDrop = vi.mocked(subscribeWindowDragDrop);

type HookProps = {
  activeTab: 'files' | 'events';
  searchInputRef: RefObject<HTMLInputElement | null>;
  focusSearchInput: () => void;
  handleStatusUpdate: (scannedFiles: number, processedEvents: number, rescanErrors: number) => void;
  setLifecycleState: (status: 'Initializing' | 'Updating' | 'Ready') => void;
  submitFilesQuery: (query: string, options?: { immediate?: boolean }) => void;
  setEventFilterQuery: (query: string) => void;
};

describe('useAppWindowListeners', () => {
  const statusUnlisten = vi.fn();
  const lifecycleUnlisten = vi.fn();
  const quickLaunchUnlisten = vi.fn();
  const dragDropUnlisten = vi.fn();

  const focusSearchInput = vi.fn();
  const handleStatusUpdate = vi.fn();
  const setLifecycleState = vi.fn();
  const submitFilesQuery = vi.fn();
  const setEventFilterQuery = vi.fn();
  const searchInputRef: { current: HTMLInputElement | null } = { current: null };
  let searchInputElement: HTMLInputElement;
  let elementFromPointMock: ReturnType<typeof vi.fn>;

  let statusCallback:
    | ((payload: { scannedFiles: number; processedEvents: number; rescanErrors: number }) => void)
    | null;
  let lifecycleCallback: ((status: 'Initializing' | 'Updating' | 'Ready') => void) | null;
  let quickLaunchCallback: (() => void) | null;
  let dragDropCallback: ((event: any) => void) | null;

  const renderWindowListeners = (overrides: Partial<HookProps> = {}) =>
    renderHook((props: HookProps) => useAppWindowListeners(props), {
      initialProps: {
        activeTab: 'files',
        searchInputRef,
        focusSearchInput,
        handleStatusUpdate,
        setLifecycleState,
        submitFilesQuery,
        setEventFilterQuery,
        ...overrides,
      },
    });

  beforeEach(() => {
    vi.clearAllMocks();
    statusCallback = null;
    lifecycleCallback = null;
    quickLaunchCallback = null;
    dragDropCallback = null;
    document.documentElement.removeAttribute('data-window-focused');
    searchInputElement = document.createElement('input');
    searchInputRef.current = searchInputElement;
    elementFromPointMock = vi.fn(() => searchInputElement);
    Object.defineProperty(document, 'elementFromPoint', {
      configurable: true,
      value: elementFromPointMock,
    });

    mockedSubscribeStatusBarUpdate.mockImplementation((listener) => {
      statusCallback = listener;
      return statusUnlisten;
    });
    mockedSubscribeLifecycleState.mockImplementation((listener) => {
      lifecycleCallback = listener;
      return lifecycleUnlisten;
    });
    mockedSubscribeQuickLaunch.mockImplementation((listener) => {
      quickLaunchCallback = listener;
      return quickLaunchUnlisten;
    });
    mockedSubscribeWindowDragDrop.mockImplementation((listener) => {
      dragDropCallback = listener;
      return dragDropUnlisten;
    });
  });

  it('subscribes to runtime events and dispatches payloads to handlers', async () => {
    renderWindowListeners();

    await waitFor(() => {
      expect(mockedSubscribeStatusBarUpdate).toHaveBeenCalledTimes(1);
      expect(mockedSubscribeLifecycleState).toHaveBeenCalledTimes(1);
      expect(mockedSubscribeQuickLaunch).toHaveBeenCalledTimes(1);
      expect(mockedSubscribeWindowDragDrop).toHaveBeenCalledTimes(1);
    });

    act(() => {
      statusCallback?.({ scannedFiles: 11, processedEvents: 22, rescanErrors: 3 });
    });
    expect(handleStatusUpdate).toHaveBeenCalledWith(11, 22, 3);

    act(() => {
      lifecycleCallback?.('Ready');
    });
    expect(setLifecycleState).toHaveBeenCalledWith('Ready');

    act(() => {
      quickLaunchCallback?.();
    });
    expect(focusSearchInput).toHaveBeenCalledTimes(1);
  });

  it('handles drag-drop search routing for files and events tabs', async () => {
    const { rerender } = renderWindowListeners();

    await waitFor(() => {
      expect(dragDropCallback).not.toBeNull();
    });

    act(() => {
      dragDropCallback?.({
        payload: { type: 'drop', paths: [' /tmp/file-a '], position: { x: 40, y: 80 } },
      });
    });
    expect(elementFromPointMock).toHaveBeenCalledWith(40, 80);
    expect(submitFilesQuery).toHaveBeenCalledWith('"/tmp/file-a"', {
      immediate: true,
    });

    rerender({
      activeTab: 'events',
      searchInputRef,
      focusSearchInput,
      handleStatusUpdate,
      setLifecycleState,
      submitFilesQuery,
      setEventFilterQuery,
    });

    act(() => {
      dragDropCallback?.({
        payload: { type: 'drop', paths: [' /tmp/file-b '], position: { x: 20, y: 30 } },
      });
    });
    expect(setEventFilterQuery).toHaveBeenCalledWith('"/tmp/file-b"');
  });

  it('ignores drops outside the search input', async () => {
    renderWindowListeners();

    await waitFor(() => {
      expect(dragDropCallback).not.toBeNull();
    });

    elementFromPointMock.mockReturnValue(document.createElement('div'));

    act(() => {
      dragDropCallback?.({
        payload: { type: 'drop', paths: [' /tmp/file-a '], position: { x: 10, y: 15 } },
      });
    });

    expect(submitFilesQuery).not.toHaveBeenCalled();
    expect(setEventFilterQuery).not.toHaveBeenCalled();
  });

  it('syncs window focus attribute and cleans up runtime subscriptions on unmount', async () => {
    const { unmount } = renderWindowListeners();

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
