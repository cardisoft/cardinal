# VirtualList Deep Dive

This chapter describes the virtualized list component in `cardinal/src/components/VirtualList.tsx` and its supporting hooks.

---

## Components and hooks
- `VirtualList.tsx`: headless virtualizer that renders only visible rows, exposes imperative controls, and owns scroll math.
- `useDataLoader`: hydrates rows lazily by calling Tauri `get_nodes_info` for visible ranges; caches results per result-set version; listens for `icon_update` push events to patch icons.
- `useIconViewport`: throttles the visible slab indices to the backend via `update_icon_viewport` so the backend can prefetch icons for the window.
- `Scrollbar`: custom vertical scrollbar bound to the virtual height.

---

## Render flow
```
results (SlabIndex[]) from search
   ↓ VirtualList
      - track scrollTop + viewportHeight (ResizeObserver)
      - compute visible [start, end] with overscan
      - call useIconViewport(start,end)
      - ensureRangeLoaded(start,end) to hydrate rows
      - render absolute-positioned rows via renderRow(...)
      - emit horizontal scroll for header sync
      - custom Scrollbar drives scrollTop
```

Timing diagram for one scroll tick:
```text
user scrolls wheel
        │
        ▼
VirtualList.handleWheel
  → updateScrollAndRange(scrollTop)
        │
        ▼
compute [start,end] window
        │
        ├─ useIconViewport(start,end)
        │    → schedule update_icon_viewport(id, viewport)
        │
        └─ ensureRangeLoaded(start,end)
             → invoke('get_nodes_info', { results: slice })
             → merge into cache
        │
        ▼
renderRow(rowIndex, item, style) for visible rows only
```

`VirtualList` is responsible for:
- Tracking `scrollTop` and viewport height with a `ResizeObserver`.
- Computing a window `[start, end]` of visible rows with configurable `overscan`.
- Delegating data loading and icon prefetch to hooks.
- Exposing `scrollToTop`, `scrollToRow`, `ensureRangeLoaded`, and `getItem` via an imperative handle.

---

## Data hydration
```
ensureRangeLoaded(start,end):
  identify indices needing fetch (not in cache, not loading)
  invoke('get_nodes_info', { results: slice })
  merge responses into cache (respect icon overrides)

icon_update listener:
  map slabIndex -> row index via indexMapRef
  apply icon overrides to cached rows
```

- `versionRef` in `useDataLoader` guards against races: if the results array identity changes mid-fetch, the response is discarded.
- `iconOverridesRef` lets pushed icons override stale cache entries without re-fetching node info.

---

## Icon viewport throttling
```
useIconViewport:
  track lastRange; schedule viewport slice via requestAnimationFrame
  invoke('update_icon_viewport', { id, viewport }) with current visible slab indices
  on unmount -> flush pending (also sends empty range once when list clears)
```

- Backend uses this to QuickLook-load icons only for visible rows, reducing I/O.
- The hook deduplicates identical ranges and sends a final empty viewport on teardown.

---

## Scroll and imperative API
- Wheel handling normalizes `deltaMode` to pixels and clamps to `[0, maxScrollTop]`.
- `scrollToRow(rowIndex, align)` supports `nearest`, `start`, `end`, and `center`.
- `scrollToTop`, `ensureRangeLoaded`, and `getItem` are exposed via `forwardRef` so parent components can drive preloading and focus jumps.
- Horizontal scroll is forwarded via `onScrollSync` to keep headers aligned with columns.

---

## Layout math
```
rowCount = results.length
totalHeight = rowCount * rowHeight
start = floor(scrollTop / rowHeight) - overscan
end   = ceil((scrollTop + viewportHeight)/rowHeight) + overscan - 1
rendered rows: absolute positioned inside .virtual-list-items
```

- `scrollTop` is reclamped when the result set shrinks to avoid blank regions.
- `overscan` tunes the trade-off between scroll smoothness and render cost.

---

## Extension tips
- Keep `renderRow` pure and stable to maximize memoization benefits.
- Prefer extending `useDataLoader` for additional per-row data (e.g., extra metadata) so it remains versioned, cancellable, and cache-aware.
- Adjust `rowHeight` and `overscan` carefully; extremely small heights or high overscan will increase rendering pressure.
