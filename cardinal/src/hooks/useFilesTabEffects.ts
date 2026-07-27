import { useEffect } from 'react';
import type { RefObject } from 'react';
import type { StatusTabKey } from '../components/StatusBar';
import type { FSEventsPanelHandle } from '../components/FSEventsPanel';
import type { VirtualListHandle } from '../components/VirtualList';

type UseFilesTabEffectsOptions = {
  isWorkspaceActive?: boolean;
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
  isWorkspaceActive = true,
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
    if (!isWorkspaceActive) {
      return;
    }
    return () => closeQuickLook();
  }, [closeQuickLook, isWorkspaceActive]);

  useEffect(() => {
    if (isWorkspaceActive && activeTab !== 'files') {
      clearSelection();
    }
  }, [activeTab, clearSelection, isWorkspaceActive]);

  useEffect(() => {
    if (!isWorkspaceActive) {
      return;
    }

    if (activeTab !== 'files') {
      closeQuickLook();
      return;
    }

    if (selectedIndices.length > 0) {
      updateQuickLook();
    }
  }, [activeTab, selectedIndices, closeQuickLook, isWorkspaceActive, updateQuickLook]);

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
    if (isWorkspaceActive && activeTab === 'events') {
      queueMicrotask(() => {
        eventsPanelRef.current?.scrollToBottom?.();
      });
    }
  }, [activeTab, eventsPanelRef, isWorkspaceActive]);
}
