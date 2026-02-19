import { useEffect } from 'react';
import type { RefObject } from 'react';
import type { StatusTabKey } from '../components/StatusBar';
import type { FSEventsPanelHandle } from '../components/FSEventsPanel';
import type { VirtualListHandle } from '../components/VirtualList';

type UseFilesTabEffectsOptions = {
  activeTab: StatusTabKey;
  selectedIndices: number[];
  activeRowIndex: number | null;
  closeQuickLook: () => void;
  updateQuickLook: () => void;
  clearSelection: () => void;
  resultsVersion: number;
  virtualListRef: RefObject<VirtualListHandle | null>;
  eventsPanelRef: RefObject<FSEventsPanelHandle | null>;
};

/**
 * Centralizes files/events tab effects so App focuses on composing state and rendering.
 */
export function useFilesTabEffects({
  activeTab,
  selectedIndices,
  activeRowIndex,
  closeQuickLook,
  updateQuickLook,
  clearSelection,
  resultsVersion,
  virtualListRef,
  eventsPanelRef,
}: UseFilesTabEffectsOptions): void {
  useEffect(() => {
    if (activeTab !== 'files') {
      clearSelection();
    }
  }, [activeTab, clearSelection]);

  useEffect(() => {
    if (activeTab !== 'files') {
      closeQuickLook();
      return;
    }

    if (selectedIndices.length > 0) {
      updateQuickLook();
    }
  }, [activeTab, selectedIndices, closeQuickLook, updateQuickLook]);

  useEffect(() => {
    if (activeRowIndex == null) {
      return;
    }

    virtualListRef.current?.scrollToRow?.(activeRowIndex, 'nearest');
  }, [activeRowIndex, virtualListRef]);

  useEffect(() => {
    clearSelection();
    virtualListRef.current?.scrollToTop?.();
  }, [resultsVersion, clearSelection, virtualListRef]);

  useEffect(() => {
    if (activeTab === 'events') {
      queueMicrotask(() => {
        eventsPanelRef.current?.scrollToBottom?.();
      });
    }
  }, [activeTab, eventsPanelRef]);
}
