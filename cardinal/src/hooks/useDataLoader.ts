import { useCallback, useRef, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { UnlistenFn } from '@tauri-apps/api/event';
import type { NodeInfoResponse, SearchResultItem } from '../types/search';
import type { SlabIndex } from '../types/slab';
import type { IconUpdatePayload, IconUpdateWirePayload } from '../types/ipc';

type IconUpdateEventPayload = readonly IconUpdateWirePayload[] | null | undefined;

export type DataLoaderCache = Map<number, SearchResultItem>;
type IconOverrideValue = string | null;

const normalizeIcon = (icon: string | null | undefined): string | undefined => icon ?? undefined;

const fromNodeInfo = (node: NodeInfoResponse): SearchResultItem => {
  const metadata = node.metadata ?? undefined;
  const base: SearchResultItem = {
    path: node.path,
    metadata,
    size: node.size ?? metadata?.size,
    mtime: node.mtime ?? metadata?.mtime,
    ctime: node.ctime ?? metadata?.ctime,
    icon: normalizeIcon(node.icon),
  };
  return base;
};

export function useDataLoader(results: SlabIndex[], resultsVersion: number) {
  const loadingRef = useRef<Set<number>>(new Set());
  const versionRef = useRef(0);
  const cacheRef = useRef<DataLoaderCache>(new Map());
  const indexMapRef = useRef<Map<SlabIndex, number>>(new Map());
  const iconOverridesRef = useRef<Map<number, IconOverrideValue>>(new Map());
  const [cache, setCache] = useState<DataLoaderCache>(() => {
    const initial = new Map<number, SearchResultItem>();
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
    const nextCache = new Map<number, SearchResultItem>();
    cacheRef.current = nextCache;
    const indexMap = new Map<SlabIndex, number>();
    resultsRef.current.forEach((value, index) => {
      if (value != null) {
        indexMap.set(value, index);
      }
    });
    indexMapRef.current = indexMap;
    setCache(nextCache);
  }, [resultsVersion]);

  useEffect(() => {
    let unlistenIconUpdate: UnlistenFn | undefined;
    (async () => {
      try {
        unlistenIconUpdate = await listen<IconUpdateEventPayload>('icon_update', (event) => {
          const updates = event?.payload;
          if (!Array.isArray(updates) || updates.length === 0) {
            return;
          }

          const normalized: IconUpdatePayload[] = [];
          updates.forEach((update) => {
            if (update && typeof update.slabIndex === 'number') {
              normalized.push({
                slabIndex: update.slabIndex as SlabIndex,
                icon: update.icon,
              });
            }
          });

          if (normalized.length === 0) {
            return;
          }

          setCache((prev) => {
            let nextCache: DataLoaderCache | null = null;

            normalized.forEach((update) => {
              const index = indexMapRef.current.get(update.slabIndex);
              if (index === undefined) return;

              const overrideValue: IconOverrideValue = update.icon ?? null;
              iconOverridesRef.current.set(index, overrideValue);

              const current = prev.get(index);
              if (!current) return;

              const nextIcon = normalizeIcon(overrideValue);
              if (current.icon === nextIcon) return;

              if (nextCache === null) {
                nextCache = new Map(prev);
              }

              nextCache.set(index, { ...current, icon: nextIcon });
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
    const needLoading: number[] = [];
    for (let i = start; i <= end && i < total; i++) {
      if (!cacheRef.current.has(i) && !loadingRef.current.has(i) && list[i] != null) {
        needLoading.push(i);
        loadingRef.current.add(i);
      }
    }
    if (needLoading.length === 0) return;
    const versionAtRequest = versionRef.current;
    const slice = needLoading.map((i) => list[i]);
    const fetched = await invoke<NodeInfoResponse[]>('get_nodes_info', { results: slice });
    if (versionRef.current !== versionAtRequest) {
      needLoading.forEach((i) => loadingRef.current.delete(i));
      return;
    }
    setCache((prev) => {
      if (versionRef.current !== versionAtRequest) return prev;
      let nextCache: DataLoaderCache | null = null;

      needLoading.forEach((originalIndex, idx) => {
        const fetchedItem = fetched[idx];
        loadingRef.current.delete(originalIndex);
        if (!fetchedItem) {
          return;
        }

        const normalizedItem = fromNodeInfo(fetchedItem);
        const existing = prev.get(originalIndex);
        const hasOverride = iconOverridesRef.current.has(originalIndex);
        const override = hasOverride ? iconOverridesRef.current.get(originalIndex) : undefined;

        const preferredIcon = hasOverride
          ? normalizeIcon(override)
          : (existing?.icon ?? normalizedItem.icon);

        const mergedItem =
          preferredIcon === normalizedItem.icon
            ? normalizedItem
            : { ...normalizedItem, icon: preferredIcon };

        if (nextCache === null) {
          nextCache = new Map(prev);
        }

        nextCache.set(originalIndex, mergedItem);
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
