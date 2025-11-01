import { useCallback, useRef, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { UnlistenFn } from '@tauri-apps/api/event';
import type { SearchResultItem } from '../components/FileRow';

type IconUpdate = {
  slabIndex: unknown;
  icon?: string;
};

type IconUpdateEventPayload = IconUpdate[] | undefined;

export type DataLoaderCache = Map<number, SearchResultItem>;

const getIcon = (item: SearchResultItem | undefined): string | undefined =>
  typeof item === 'object' && item !== null ? item.icon : undefined;

const mergeIcon = (
  item: SearchResultItem | undefined,
  icon: string | undefined,
): SearchResultItem => {
  if (item && typeof item === 'object') {
    return { ...item, icon };
  }
  if (typeof item === 'string') {
    return icon === undefined ? item : ({ path: item, icon } as SearchResultItem);
  }
  return { icon } as SearchResultItem;
};

export function useDataLoader(results: SearchResultItem[] | null | undefined) {
  const loadingRef = useRef<Set<number>>(new Set());
  const versionRef = useRef(0);
  const cacheRef = useRef<DataLoaderCache>(new Map());
  const indexMapRef = useRef<Map<unknown, number>>(new Map());
  const [cache, setCache] = useState<DataLoaderCache>(() => {
    const initial = new Map<number, SearchResultItem>();
    cacheRef.current = initial;
    return initial;
  });
  const resultsRef = useRef<SearchResultItem[]>([]);

  // Reset loading state whenever the result source changes.
  useEffect(() => {
    versionRef.current += 1;
    loadingRef.current.clear();
    const nextCache = new Map<number, SearchResultItem>();
    cacheRef.current = nextCache;
    resultsRef.current = Array.isArray(results) ? results : [];
    const indexMap = new Map<unknown, number>();
    resultsRef.current.forEach((value, index) => {
      if (value != null) {
        indexMap.set(value, index);
      }
    });
    indexMapRef.current = indexMap;
    setCache(nextCache);
  }, [results]);

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
            // Collect items that truly changed before creating a fresh Map.
            const changes: Array<{
              index: number;
              current: SearchResultItem | undefined;
              newIcon?: string;
            }> = [];
            updates.forEach((update) => {
              const index = indexMapRef.current.get(update.slabIndex);
              if (index === undefined) return;
              const current = prev.get(index);
              const newIcon = update.icon;
              if (getIcon(current) !== newIcon) {
                changes.push({ index, current, newIcon });
              }
            });

            if (changes.length === 0) return prev;

            const next = new Map(prev);
            changes.forEach(({ index, current, newIcon }) => {
              next.set(index, mergeIcon(current, newIcon));
            });

            cacheRef.current = next;
            return next;
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

  const ensureRangeLoaded = useCallback(
    async (start: number, end: number) => {
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
      try {
        const slice = needLoading.map((i) => list[i]);
        const fetched = await invoke<SearchResultItem[]>('get_nodes_info', { results: slice });
        if (versionRef.current !== versionAtRequest) {
          needLoading.forEach((i) => loadingRef.current.delete(i));
          return;
        }
        setCache((prev) => {
          if (versionRef.current !== versionAtRequest) return prev;
          const newCache = new Map(prev);
          needLoading.forEach((originalIndex, idx) => {
            const fetchedItem = fetched[idx];
            if (fetchedItem !== undefined) {
              const existing = newCache.get(originalIndex);
              const preferredIcon = getIcon(existing) ?? getIcon(fetchedItem);
              const merged = mergeIcon(fetchedItem, preferredIcon);
              newCache.set(originalIndex, merged);
            }
            loadingRef.current.delete(originalIndex);
          });
          cacheRef.current = newCache;
          return newCache;
        });
      } catch (err) {
        needLoading.forEach((i) => loadingRef.current.delete(i));
        console.error('Failed loading rows', err);
      }
    },
    [results],
  );

  return { cache, ensureRangeLoaded };
}
