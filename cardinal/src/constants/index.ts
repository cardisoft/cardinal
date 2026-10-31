// UI constants

// Column width ratios derived from the viewport width percentages.
export const COL_WIDTH_RATIOS = {
  filename: 0.25, // 25%
  path: 0.38, // 38%
  size: 0.08, // 8%
  modified: 0.145, // 14.5%
  created: 0.145, // 14.5%
} as const;

export type ColumnKey = keyof typeof COL_WIDTH_RATIOS;

// Derive the initial column widths given the available window width.
export const calculateInitialColWidths = (windowWidth: number): Record<ColumnKey, number> => {
  // Reserve space for padding and the custom scrollbar so column math aligns with CSS.
  const availableWidth = windowWidth - CONTAINER_PADDING - SCROLLBAR_WIDTH;

  const calculatedWidths = {} as Record<ColumnKey, number>;

  for (const [key, ratio] of Object.entries(COL_WIDTH_RATIOS) as [ColumnKey, number][]) {
    const calculatedWidth = Math.floor(availableWidth * ratio);
    calculatedWidths[key] = Math.max(calculatedWidth, MIN_COL_WIDTH);
  }

  return calculatedWidths;
};

export const ROW_HEIGHT = 24;
export const CONTAINER_PADDING = 10;
// Keep in sync with the CSS variable --virtual-scrollbar-width.
export const SCROLLBAR_WIDTH = 14;

// Minimum thumb height for the virtual scrollbar (in px). Keep this in sync with
// the CSS variable --virtual-scrollbar-thumb-min in src/App.css.
export const SCROLLBAR_THUMB_MIN = 24;

// Cache and performance tuning
export const CACHE_SIZE = 1000;
export const SEARCH_DEBOUNCE_MS = 300;
/** How often a search is retried while a temporary folder-scope index is being built. */
export const SCOPE_INDEX_POLL_MS = 800;
export const STATUS_FADE_DELAY_MS = 2000;
export const OVERSCAN_ROW_COUNT = 1;

export const MIN_COL_WIDTH = 30;
export const MAX_COL_WIDTH = 10000;

// Launch list ("most recent files") page size. The backend returns this many items per
// request; scrolling near the bottom asks for the next page.
export const RECENT_PAGE_SIZE = 200;
