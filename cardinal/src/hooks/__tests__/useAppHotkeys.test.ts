import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { openResultPath } from '../../utils/openResultPath';
import { useAppHotkeys } from '../useAppHotkeys';

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

vi.mock('../../utils/openResultPath', () => ({
  openResultPath: vi.fn(),
}));

const mockedListen = vi.mocked(listen);
const mockedInvoke = vi.mocked(invoke);
const mockedOpenResultPath = vi.mocked(openResultPath);

type HookProps = {
  activeTab: 'files' | 'events';
  selectedPaths: string[];
  selectedIndicesRef: { current: number[] };
  focusSearchInput: () => void;
  navigateSelection: (delta: 1 | -1, options?: { extend?: boolean }) => void;
  triggerQuickLook: () => void;
};

describe('useAppHotkeys', () => {
  const quickLookUnlisten = vi.fn();
  const focusSearchInput = vi.fn();
  const navigateSelection = vi.fn();
  const triggerQuickLook = vi.fn();

  let quickLookListener: ((event: any) => void) | null;

  const renderHotkeys = (overrides: Partial<HookProps> = {}) =>
    renderHook((props: HookProps) => useAppHotkeys(props), {
      initialProps: {
        activeTab: 'files',
        selectedPaths: ['/tmp/a', '/tmp/b'],
        selectedIndicesRef: { current: [0] },
        focusSearchInput,
        navigateSelection,
        triggerQuickLook,
        ...overrides,
      },
    });

  beforeEach(() => {
    vi.clearAllMocks();
    quickLookListener = null;
    mockedInvoke.mockResolvedValue(undefined);

    mockedListen.mockImplementation(async (eventName: string, callback: (event: any) => void) => {
      if (eventName === 'quicklook-keydown') {
        quickLookListener = callback;
        return quickLookUnlisten;
      }
      return vi.fn();
    });
  });

  it('handles Meta+F, Meta+R, Meta+O, and Meta+C shortcuts on files tab', async () => {
    renderHotkeys();

    const findEvent = new KeyboardEvent('keydown', {
      key: 'f',
      metaKey: true,
      cancelable: true,
    });
    act(() => {
      window.dispatchEvent(findEvent);
    });
    expect(focusSearchInput).toHaveBeenCalledTimes(1);
    expect(findEvent.defaultPrevented).toBe(true);

    const openEvent = new KeyboardEvent('keydown', {
      key: 'o',
      metaKey: true,
      cancelable: true,
    });
    act(() => {
      window.dispatchEvent(openEvent);
    });
    expect(mockedOpenResultPath).toHaveBeenCalledWith('/tmp/a');
    expect(mockedOpenResultPath).toHaveBeenCalledWith('/tmp/b');

    const revealEvent = new KeyboardEvent('keydown', {
      key: 'r',
      metaKey: true,
      cancelable: true,
    });
    act(() => {
      window.dispatchEvent(revealEvent);
    });
    expect(mockedInvoke).toHaveBeenCalledWith('open_in_finder', { path: '/tmp/a' });
    expect(mockedInvoke).toHaveBeenCalledWith('open_in_finder', { path: '/tmp/b' });

    const copyEvent = new KeyboardEvent('keydown', {
      key: 'c',
      metaKey: true,
      cancelable: true,
    });
    act(() => {
      window.dispatchEvent(copyEvent);
    });
    expect(mockedInvoke).toHaveBeenCalledWith('copy_files_to_clipboard', {
      paths: ['/tmp/a', '/tmp/b'],
    });
  });

  it('handles space and arrow navigation on files tab', () => {
    renderHotkeys();

    const spaceEvent = new KeyboardEvent('keydown', {
      key: ' ',
      code: 'Space',
      cancelable: true,
    });
    act(() => {
      window.dispatchEvent(spaceEvent);
    });
    expect(triggerQuickLook).toHaveBeenCalledTimes(1);
    expect(spaceEvent.defaultPrevented).toBe(true);

    const downEvent = new KeyboardEvent('keydown', {
      key: 'ArrowDown',
      shiftKey: true,
      cancelable: true,
    });
    act(() => {
      window.dispatchEvent(downEvent);
    });
    expect(navigateSelection).toHaveBeenCalledWith(1, { extend: true });

    const upEvent = new KeyboardEvent('keydown', {
      key: 'ArrowUp',
      cancelable: true,
    });
    act(() => {
      window.dispatchEvent(upEvent);
    });
    expect(navigateSelection).toHaveBeenCalledWith(-1, { extend: false });
  });

  it('handles Quick Look native keydown events and cleanup', async () => {
    const { rerender, unmount } = renderHotkeys();

    await waitFor(() => {
      expect(quickLookListener).not.toBeNull();
    });

    act(() => {
      quickLookListener?.({
        payload: {
          keyCode: 125,
          modifiers: {
            shift: true,
            control: false,
            option: false,
            command: false,
          },
        },
      });
    });
    expect(navigateSelection).toHaveBeenCalledWith(1, { extend: true });

    rerender({
      activeTab: 'events',
      selectedPaths: ['/tmp/a', '/tmp/b'],
      selectedIndicesRef: { current: [0] },
      focusSearchInput,
      navigateSelection,
      triggerQuickLook,
    });

    navigateSelection.mockClear();
    act(() => {
      quickLookListener?.({
        payload: {
          keyCode: 126,
          modifiers: {
            shift: false,
            control: false,
            option: false,
            command: false,
          },
        },
      });
    });
    expect(navigateSelection).not.toHaveBeenCalled();

    unmount();
    expect(quickLookUnlisten).toHaveBeenCalledTimes(1);
  });
});
