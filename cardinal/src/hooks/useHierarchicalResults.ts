import { useCallback, useEffect, useMemo, useReducer, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { NodeInfoResponse } from '../types/search';
import type { SlabIndex } from '../types/slab';

const TREE_RESULT_THRESHOLD = 20_000;

type TreeNodeInfo = {
  path: string;
  isDirectory: boolean;
};

export type HierarchicalResultRow = {
  slabIndex: SlabIndex;
  depth: number;
  hasChildren: boolean;
  isExpanded: boolean;
};

type HierarchicalResults = {
  results: SlabIndex[];
  rows: HierarchicalResultRow[];
  resultsVersion: number;
  toggleExpanded: (slabIndex: SlabIndex) => void;
};

const normalizePath = (path: string): string =>
  path === '/' ? path : path.replace(/\/+$/, '');

const getParentPath = (path: string): string | null => {
  const normalized = normalizePath(path);
  if (normalized === '/') {
    return null;
  }

  const separatorIndex = normalized.lastIndexOf('/');
  if (separatorIndex < 0) {
    return null;
  }
  return separatorIndex === 0 ? '/' : normalized.slice(0, separatorIndex);
};

/**
 * Projects flat search matches into a Finder-style tree when a matched directory is also
 * an ancestor of other matches. Large result sets keep the existing flat, lazy-loaded view.
 */
export function useHierarchicalResults(
  searchResults: SlabIndex[],
  orderedResults: SlabIndex[],
  dataResultsVersion: number,
  orderedResultsVersion: number,
): HierarchicalResults {
  const requestRef = useRef(0);
  const [loadedNodes, setLoadedNodes] = useState<{
    version: number;
    bySlabIndex: Map<SlabIndex, TreeNodeInfo>;
  } | null>(null);
  const [collapsedDirectories, setCollapsedDirectories] = useState<Set<SlabIndex>>(
    () => new Set(),
  );
  const [projectionVersion, bumpProjectionVersion] = useReducer(
    (version: number) => version + 1,
    0,
  );

  useEffect(() => {
    const requestId = requestRef.current + 1;
    requestRef.current = requestId;
    setCollapsedDirectories(new Set());

    if (searchResults.length === 0 || searchResults.length > TREE_RESULT_THRESHOLD) {
      setLoadedNodes(null);
      bumpProjectionVersion();
      return;
    }

    void (async () => {
      try {
        const nodes = await invoke<NodeInfoResponse[]>('get_nodes_info', {
          results: searchResults,
          includeIcons: false,
        });
        if (requestRef.current !== requestId) {
          return;
        }

        const bySlabIndex = new Map<SlabIndex, TreeNodeInfo>();
        searchResults.forEach((slabIndex, index) => {
          const node = nodes[index];
          if (!node) {
            return;
          }
          bySlabIndex.set(slabIndex, {
            path: normalizePath(node.path),
            isDirectory: node.metadata?.type === 1,
          });
        });
        setLoadedNodes({ version: dataResultsVersion, bySlabIndex });
        bumpProjectionVersion();
      } catch {
        // Keep the existing flat list when hierarchy metadata cannot be loaded.
        if (requestRef.current === requestId) {
          setLoadedNodes(null);
        }
      }
    })();
  }, [dataResultsVersion, searchResults]);

  const projection = useMemo(() => {
    if (!loadedNodes || loadedNodes.version !== dataResultsVersion) {
      return {
        results: orderedResults,
        rows: orderedResults.map((slabIndex) => ({
          slabIndex,
          depth: 0,
          hasChildren: false,
          isExpanded: false,
        })),
      };
    }

    const nodeBySlabIndex = loadedNodes.bySlabIndex;
    const directoryByPath = new Map<string, SlabIndex>();
    orderedResults.forEach((slabIndex) => {
      const node = nodeBySlabIndex.get(slabIndex);
      if (node?.isDirectory) {
        directoryByPath.set(node.path, slabIndex);
      }
    });

    const childrenByParent = new Map<SlabIndex, SlabIndex[]>();
    const roots: SlabIndex[] = [];

    orderedResults.forEach((slabIndex) => {
      const node = nodeBySlabIndex.get(slabIndex);
      let ancestorPath = node ? getParentPath(node.path) : null;
      let parentSlabIndex: SlabIndex | undefined;

      while (ancestorPath !== null) {
        parentSlabIndex = directoryByPath.get(ancestorPath);
        if (parentSlabIndex !== undefined) {
          break;
        }
        ancestorPath = getParentPath(ancestorPath);
      }

      if (parentSlabIndex === undefined) {
        roots.push(slabIndex);
        return;
      }

      const siblings = childrenByParent.get(parentSlabIndex);
      if (siblings) {
        siblings.push(slabIndex);
      } else {
        childrenByParent.set(parentSlabIndex, [slabIndex]);
      }
    });

    // A containing folder acts as the group header and is promoted above unrelated flat matches.
    roots.sort((left, right) => {
      const leftIsGroup = childrenByParent.has(left);
      const rightIsGroup = childrenByParent.has(right);
      return Number(rightIsGroup) - Number(leftIsGroup);
    });

    const rows: HierarchicalResultRow[] = [];
    const appendNode = (slabIndex: SlabIndex, depth: number) => {
      const children = childrenByParent.get(slabIndex) ?? [];
      const hasChildren = children.length > 0;
      const isExpanded = hasChildren && !collapsedDirectories.has(slabIndex);
      rows.push({ slabIndex, depth, hasChildren, isExpanded });
      if (isExpanded) {
        children.forEach((child) => appendNode(child, depth + 1));
      }
    };
    roots.forEach((root) => appendNode(root, 0));

    return {
      results: rows.map((row) => row.slabIndex),
      rows,
    };
  }, [collapsedDirectories, dataResultsVersion, loadedNodes, orderedResults]);

  const toggleExpanded = useCallback((slabIndex: SlabIndex) => {
    setCollapsedDirectories((current) => {
      const next = new Set(current);
      if (next.has(slabIndex)) {
        next.delete(slabIndex);
      } else {
        next.add(slabIndex);
      }
      return next;
    });
    bumpProjectionVersion();
  }, []);

  return {
    ...projection,
    resultsVersion: orderedResultsVersion + projectionVersion,
    toggleExpanded,
  };
}
