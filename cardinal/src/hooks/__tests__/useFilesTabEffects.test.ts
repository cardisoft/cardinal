import { act, renderHook } from '@testing-library/react';
import type { RefObject } from 'react';
import { describe, expect, it, vi } from 'vitest';
import type { FSEventsPanelHandle } from '../../components/FSEventsPanel';
import type { VirtualListHandle } from '../../components/VirtualList';
import { useFilesTabEffects } from '../useFilesTabEffects';

type HookProps = {
  activeTab: 'files' | 'events';
  selectedIndices: number[];
  activeRowIndex: number | null;
  closeQuickLook: () => void;
  updateQuickLook: () => void;
  clearSelection: () => void;
  resultsVersion: number;
  virtualListRef: RefObject<VirtualListHandle | null>;
  eventsPanelRef: RefObject<FSEventsPanelHandle | null>;
};

const createVirtualListRef = () =>
  ({
    current: {
      scrollToRow: vi.fn(),
      scrollToTop: vi.fn(),
    },
  }) as unknown as RefObject<VirtualListHandle | null>;

const createEventsPanelRef = () =>
  ({
    current: {
      scrollToBottom: vi.fn(),
    },
  }) as unknown as RefObject<FSEventsPanelHandle | null>;

describe('useFilesTabEffects', () => {
  it('clears selection and closes Quick Look when switching to events tab', async () => {
    const clearSelection = vi.fn();
    const closeQuickLook = vi.fn();
    const updateQuickLook = vi.fn();
    const virtualListRef = createVirtualListRef();
    const eventsPanelRef = createEventsPanelRef();

    renderHook((props: HookProps) => useFilesTabEffects(props), {
      initialProps: {
        activeTab: 'events',
        selectedIndices: [0],
        activeRowIndex: null,
        closeQuickLook,
        updateQuickLook,
        clearSelection,
        resultsVersion: 1,
        virtualListRef,
        eventsPanelRef,
      },
    });

    expect(clearSelection).toHaveBeenCalledTimes(2);
    expect(closeQuickLook).toHaveBeenCalledTimes(1);
    expect(updateQuickLook).not.toHaveBeenCalled();

    await act(async () => {
      await Promise.resolve();
    });
    expect(eventsPanelRef.current?.scrollToBottom).toHaveBeenCalledTimes(1);
  });

  it('updates Quick Look while files tab is active and there is a selection', () => {
    const clearSelection = vi.fn();
    const closeQuickLook = vi.fn();
    const updateQuickLook = vi.fn();
    const virtualListRef = createVirtualListRef();
    const eventsPanelRef = createEventsPanelRef();

    const { rerender } = renderHook((props: HookProps) => useFilesTabEffects(props), {
      initialProps: {
        activeTab: 'files',
        selectedIndices: [0],
        activeRowIndex: null,
        closeQuickLook,
        updateQuickLook,
        clearSelection,
        resultsVersion: 1,
        virtualListRef,
        eventsPanelRef,
      },
    });

    expect(updateQuickLook).toHaveBeenCalledTimes(1);
    expect(closeQuickLook).not.toHaveBeenCalled();

    rerender({
      activeTab: 'files',
      selectedIndices: [0, 1],
      activeRowIndex: null,
      closeQuickLook,
      updateQuickLook,
      clearSelection,
      resultsVersion: 1,
      virtualListRef,
      eventsPanelRef,
    });
    expect(updateQuickLook).toHaveBeenCalledTimes(2);
  });

  it('keeps list viewport in sync with active row and results version', () => {
    const clearSelection = vi.fn();
    const closeQuickLook = vi.fn();
    const updateQuickLook = vi.fn();
    const virtualListRef = createVirtualListRef();
    const eventsPanelRef = createEventsPanelRef();

    const { rerender } = renderHook((props: HookProps) => useFilesTabEffects(props), {
      initialProps: {
        activeTab: 'files',
        selectedIndices: [],
        activeRowIndex: null,
        closeQuickLook,
        updateQuickLook,
        clearSelection,
        resultsVersion: 1,
        virtualListRef,
        eventsPanelRef,
      },
    });

    expect(virtualListRef.current?.scrollToTop).toHaveBeenCalledTimes(1);

    rerender({
      activeTab: 'files',
      selectedIndices: [],
      activeRowIndex: 3,
      closeQuickLook,
      updateQuickLook,
      clearSelection,
      resultsVersion: 1,
      virtualListRef,
      eventsPanelRef,
    });
    expect(virtualListRef.current?.scrollToRow).toHaveBeenCalledWith(3, 'nearest');

    rerender({
      activeTab: 'files',
      selectedIndices: [],
      activeRowIndex: 3,
      closeQuickLook,
      updateQuickLook,
      clearSelection,
      resultsVersion: 2,
      virtualListRef,
      eventsPanelRef,
    });
    expect(clearSelection).toHaveBeenCalledTimes(2);
    expect(virtualListRef.current?.scrollToTop).toHaveBeenCalledTimes(2);
  });
});
