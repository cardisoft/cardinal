import type { SearchResultItem } from '../types/search';
import type { SlabIndex } from '../types/slab';
import type { VirtualListHandle } from '../components/VirtualList';

type RangeSelectionOptions = {
  results: readonly SlabIndex[];
  virtualList: VirtualListHandle | null;
  startIndex: number;
  endIndex: number;
};

export function getAllPathsInRange({
  results,
  virtualList,
  startIndex,
  endIndex,
}: RangeSelectionOptions): string[] {
  if (!virtualList) {
    return [];
  }

  const start = Math.min(startIndex, endIndex);
  const end = Math.max(startIndex, endIndex);
  const paths: string[] = [];

  for (let i = start; i <= end; i++) {
    // It's possible the item isn't loaded in the virtual list yet.
    // We need to request it. This is a simplification; a real implementation
    // might need to handle async loading here. For now, we get what we can.
    const item = virtualList.getItem(i);
    if (item?.path) {
      paths.push(item.path);
    }
  }

  return paths;
}
