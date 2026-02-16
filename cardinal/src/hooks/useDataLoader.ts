import { useCallback, useRef, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { subscribeIconUpdate } from '../runtime/tauriEventRuntime';
import type { NodeInfoResponse, SearchResultItem } from '../types/search';
import type { SlabIndex } from '../types/slab';
import type { IconUpdatePayload } from '../types/ipc';

export type DataLoaderCache = Map<SlabIndex, SearchResultItem>;
type IconOverrideValue = string | undefined;

const fromNodeInfo = (node: NodeInfoResponse): SearchResultItem => ({
  path: node.path,
  metadata: node.metadata ?? undefined,
  size: node.size ?? node.metadata?.size,
  mtime: node.mtime ?? node.metadata?.mtime,
  ctime: node.ctime ?? node.metadata?.ctime,
  icon: node.icon ?? undefined,
});

export function useDataLoader(results: SlabIndex[], dataResultsVersion: number) {
  const loadingRef = useRef<Set<SlabIndex>>(new Set());
  const versionRef = useRef(0);
  const cacheRef = useRef<DataLoaderCache>(new Map());
  const iconOverridesRef = useRef<Map<SlabIndex, IconOverrideValue>>(new Map());
  const [cache, setCache] = useState<DataLoaderCache>(() => {
    const initial = new Map<SlabIndex, SearchResultItem>();
    cacheRef.current = initial;
    return initial;
  });
  const resultsRef = useRef<SlabIndex[]>([]);
  resultsRef.current = results;

  // Reset loading state whenever the result source changes.
  useEffect(() => {
    versionRef.current += 1;
    loadingRef.current.clear();
    iconOverridesRef.current.clear();
    const nextCache = new Map<SlabIndex, SearchResultItem>();
    cacheRef.current = nextCache;
    setCache(nextCache);
  }, [dataResultsVersion]);

  useEffect(() => {
    const unlistenIconUpdate = subscribeIconUpdate((updates: readonly IconUpdatePayload[]) => {
      if (updates.length === 0) {
        return;
      }

      setCache((prev) => {
        let nextCache: DataLoaderCache | null = null;

        updates.forEach((update) => {
          const slabIndex = update.slabIndex;
          const nextIcon = update.icon;
          iconOverridesRef.current.set(slabIndex, nextIcon);

          const current = prev.get(slabIndex);
          if (!current || current.icon === nextIcon) {
            return;
          }

          if (nextCache === null) {
            nextCache = new Map(prev);
          }

          nextCache.set(slabIndex, { ...current, icon: nextIcon });
        });

        if (nextCache === null) {
          return prev;
        }

        cacheRef.current = nextCache;
        return nextCache;
      });
    });
    return unlistenIconUpdate;
  }, []);

  const releaseLoadingBatch = useCallback((slabIndices: readonly SlabIndex[]) => {
    slabIndices.forEach((slabIndex) => loadingRef.current.delete(slabIndex));
  }, []);

  const ensureRangeLoaded = useCallback(
    async (start: number, end: number) => {
      const list = resultsRef.current;
      const total = list.length;
      if (start < 0 || end < start || total === 0) return;
      const needLoading: SlabIndex[] = [];
      for (let i = start; i <= end && i < total; i++) {
        const slabIndex = list[i];
        if (!cacheRef.current.has(slabIndex) && !loadingRef.current.has(slabIndex)) {
          needLoading.push(slabIndex);
          loadingRef.current.add(slabIndex);
        }
      }
      if (needLoading.length === 0) return;
      const versionAtRequest = versionRef.current;
      const fetched = await invoke<NodeInfoResponse[]>('get_nodes_info', { results: needLoading });
      if (versionRef.current !== versionAtRequest) {
        releaseLoadingBatch(needLoading);
        return;
      }
      setCache((prev) => {
        if (versionRef.current !== versionAtRequest) return prev;
        let nextCache: DataLoaderCache | null = null;

        needLoading.forEach((slabIndex, idx) => {
          const fetchedItem = fetched[idx];
          loadingRef.current.delete(slabIndex);
          if (!fetchedItem) {
            return;
          }

          const normalizedItem = fromNodeInfo(fetchedItem);
          const existing = prev.get(slabIndex);
          const hasOverride = iconOverridesRef.current.has(slabIndex);
          const preferredIcon = hasOverride
            ? iconOverridesRef.current.get(slabIndex)
            : (existing?.icon ?? normalizedItem.icon);

          const mergedItem =
            preferredIcon === normalizedItem.icon
              ? normalizedItem
              : { ...normalizedItem, icon: preferredIcon };

          if (nextCache === null) {
            nextCache = new Map(prev);
          }

          nextCache.set(slabIndex, mergedItem);
        });

        if (nextCache === null) {
          return prev;
        }

        cacheRef.current = nextCache;
        return nextCache;
      });
    },
    [releaseLoadingBatch],
  );

  return { cache, ensureRangeLoaded };
}
