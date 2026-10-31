import type { SearchResultItem } from './search';

export type ScrollAlign = 'nearest' | 'start' | 'end' | 'center';

export type ResultsViewMode = 'list' | 'grid';

// Common imperative surface for the two results renderers (VirtualList and IconGrid).
export type ResultsViewHandle = {
  scrollToTop: () => void;
  scrollToRow: (rowIndex: number, align?: ScrollAlign) => void;
  ensureRangeLoaded: (startIndex: number, endIndex: number) => Promise<void> | void;
  getItem: (index: number) => SearchResultItem | undefined;
};
