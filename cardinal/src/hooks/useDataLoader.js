import { useCallback, useRef, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';

/**
 * 数据加载 hook，用于按需加载虚拟列表的数据
 */
export function useDataLoader(results) {
    const loadingRef = useRef(new Set());
    const versionRef = useRef(0);

    // 当 results 变化时清除加载状态
    useEffect(() => {
        versionRef.current += 1;
        loadingRef.current.clear();
    }, [results]);

    const ensureRangeLoaded = useCallback(async (start, end, cache, setCache) => {
        const total = results?.length ?? 0;
        if (!results || start < 0 || end < start || total === 0) return;
        const needLoading = [];
        for (let i = start; i <= end && i < total; i++) {
            if (!cache.has(i) && !loadingRef.current.has(i)) {
                needLoading.push(i);
                loadingRef.current.add(i);
            }
        }
        if (needLoading.length === 0) return;
        const versionAtRequest = versionRef.current;
        try {
            const slice = needLoading.map(i => results[i]);
            const fetched = await invoke('get_nodes_info', { results: slice });
            if (versionRef.current !== versionAtRequest) {
                needLoading.forEach(i => loadingRef.current.delete(i));
                return;
            }
            setCache(prev => {
                if (versionRef.current !== versionAtRequest) return prev;
                const newCache = new Map(prev);
                needLoading.forEach((originalIndex, idx) => {
                    newCache.set(originalIndex, fetched[idx]);
                    loadingRef.current.delete(originalIndex);
                });
                return newCache;
            });
        } catch (err) {
            needLoading.forEach(i => loadingRef.current.delete(i));
            console.error('Failed loading rows', err);
        }
    }, [results]);

    return { ensureRangeLoaded };
}
