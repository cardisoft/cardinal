# VirtualList Deep Dive

This chapter describes the virtualized list component in `cardinal/src/components/VirtualList.tsx` and its supporting hooks.

## Why this is custom
- Cardinal does not use a stock "big spacer div + native vertical scroll" virtualizer for the files tab.
- Safari/WebKit has a practical maximum element height in the ~33.5M px range. WebKit bug 198291 reproduces the issue with a `div` at `33554428px` and explicitly calls out the impact on virtual scrolling when the total scrollable area exceeds the maximum permitted element height.
- With Cardinal's `ROW_HEIGHT = 24`, that ceiling is only about `floor(33554428 / 24) = 1,398,101` rows. A multi-million-file result set can exceed that easily, so a traditional virtualizer that sets `height = rowCount * rowHeight` on a spacer element is not robust enough here.
- For that reason, `totalHeight` in Cardinal is used for scrollbar math only. The DOM never creates a native vertical scroll region with millions of rows' worth of pixel height.
- Metadata hydration also has to be lazy. Search results are initially just `SlabIndex[]`; row metadata is fetched on demand for the visible range via `get_nodes_info`. Eagerly hydrating all visible search results would make latency and IPC cost much harder to control.
- The same visible-range signal is also reused to tell the backend which icons/thumbnails matter right now, so virtualization doubles as a backend work scheduler.

## Pieces involved
- `VirtualList.tsx`: headless virtualizer that owns scroll state, range math, and the imperative row API.
- `useDataLoader`: hydrates visible rows via `get_nodes_info`, caches them by `SlabIndex`, and patches icons from `icon_update` events.
- `useIconViewport`: throttles the visible slab-index slice to the backend so Quick Look thumbnails are only fetched for the current viewport.
- `Scrollbar`: custom vertical scrollbar bound to the virtual height.

## Render flow
```text
displayedResults (SlabIndex[])
  -> VirtualList
     - track scrollTop + viewportHeight
     - compute visible [start, end] with overscan
     - ensureRangeLoaded(start, end)
     - useIconViewport(start, end)
     - render absolute-positioned rows only for that window
     - mirror horizontal scroll to the header
```

Two version tokens matter:
- `dataResultsVersion`: resets row hydration state
- `displayedResultsVersion`: resets viewport/icon tracking when visible ordering changes

That split is what keeps backend sorting from reusing stale viewport state while still allowing row data to be cached across pure projection changes when appropriate.

## Scroll model
- Vertical scrolling is fully controlled by React state (`scrollTop`) and the custom `Scrollbar`.
- This avoids relying on a giant scrollable spacer element whose height would be `rowCount * rowHeight`.
- Mouse wheel input is normalized across `deltaMode` values and clamped to `[0, maxScrollTop]`.
- Horizontal scrolling still happens on the inner viewport element and is mirrored upward through `onScrollSync`.
- `ResizeObserver` watches the container height so the visible range can be recalculated without a window resize listener.

## Data hydration
```text
ensureRangeLoaded(start, end):
  collect visible slab indices not in cache and not already loading
  invoke('get_nodes_info', { results: needLoading })
  merge each response into Map<SlabIndex, SearchResultItem>
```

- `useDataLoader` caches by `SlabIndex`, not by array position.
- `versionRef` is bumped whenever `dataResultsVersion` changes; old fetches are discarded.
- `loadingRef` prevents duplicate in-flight requests for the same slab index.
- `iconOverridesRef` lets pushed thumbnail updates override older row payloads cleanly.

## Icon viewport updates
```text
useIconViewport:
  dedupe unchanged [start, end]
  slice visible SlabIndex values
  batch invoke('update_icon_viewport', { id, viewport }) with requestAnimationFrame
```

- The hook is driven by `displayedResultsVersion`, not raw search results, so backend sorting changes the icon viewport immediately.
- It sends an empty viewport once when the list becomes empty or unmounts.
- The backend ignores the request id today; it is only used to make viewport updates monotonic on the frontend side.

## Imperative API
`VirtualListHandle` exposes:
- `scrollToTop()`
- `scrollToRow(rowIndex, align)`
- `ensureRangeLoaded(startIndex, endIndex)`
- `getItem(index)`

`scrollToRow(...)` supports `nearest`, `start`, `end`, and `center`.

## Layout math
```text
rowCount = results.length
totalHeight = rowCount * rowHeight
start = floor(scrollTop / rowHeight) - overscan
end   = ceil((scrollTop + viewportHeight) / rowHeight) + overscan - 1
```

- Rendered rows are absolutely positioned inside `.virtual-list-items`.
- `totalHeight` is virtual math, not DOM height. The custom scrollbar uses it to size the thumb and convert between scrollbar position and virtual row position.
- `scrollTop` is re-clamped whenever the result set shrinks.
- `overscan` is the main tuning knob for smoothness versus render pressure.

## Practical guidance
- Keep `renderRow(...)` pure so virtualization can stay cheap.
- Add new per-row payloads by extending `useDataLoader`, not by teaching `VirtualList` about application data directly.
- If sort/view changes should invalidate icon loading but not raw data hydration, update `displayedResultsVersion` only.

## Reference
- WebKit bug 198291: <https://bugs.webkit.org/show_bug.cgi?id=198291>
