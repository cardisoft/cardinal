import { useCallback, useRef, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { UnlistenFn } from '@tauri-apps/api/event';
import type { NodeInfoResponse, SearchResultItem } from '../types/search';
import type { SlabIndex } from '../types/slab';
import type { IconUpdateWirePayload } from '../types/ipc';

type IconUpdateEventPayload = readonly IconUpdateWirePayload[] | null | undefined;

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
    let unlistenIconUpdate: UnlistenFn | undefined;
    (async () => {
      try {
        unlistenIconUpdate = await listen<IconUpdateEventPayload>('icon_update', (event) => {
          const updates = event?.payload;
          if (!Array.isArray(updates) || updates.length === 0) {
            return;
          }

          setCache((prev) => {
            let nextCache: DataLoaderCache | null = null;

            updates.forEach((update) => {
              if (!update || typeof update.slabIndex !== 'number') {
                return;
              }

              const slabIndex = update.slabIndex as SlabIndex;
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
      } catch (error) {
        console.error('Failed to listen icon_update', error);
      }
    })();
    return () => {
      unlistenIconUpdate?.();
    };
  }, []);

  const ensureRangeLoaded = useCallback(async (start: number, end: number) => {
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
      needLoading.forEach((slabIndex) => loadingRef.current.delete(slabIndex));
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
  }, []);

  return { cache, ensureRangeLoaded };
}
