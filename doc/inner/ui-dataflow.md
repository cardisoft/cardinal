# UI Dataflow

This chapter maps the React-side state graph to the Tauri command/event layer.

## Search pipeline
```text
SearchBar / keyboard submit
  -> useFilesTabState.queueSearch(...)
  -> useFileSearch.handleSearch(...)
  -> invoke('search', { query, options, version })
  -> SearchResponse { results, highlights }
  -> useRemoteSort(...)
  -> <VirtualList results={displayedResults} ... />
```

- `useFileSearch` owns the authoritative search state: raw `results`, `resultsVersion`, highlight terms, status counters, lifecycle state, loading UI, timing, and error state.
- Each request increments `searchVersionRef` and passes that number as `version`.
- The backend turns `version` into `CancellationToken::new(version)`.
- React still discards stale responses locally if their request id is no longer current.
- Loading UI is immediate for the first search and delayed by 150 ms for later searches.

## Result projection and sorting
- `useRemoteSort` decides whether sorting stays enabled based on `sortThreshold` (default `20000`).
- When sorting is active, it calls `get_sorted_view` and receives a reordered `SlabIndex[]`.
- The hook exposes two different version tokens:
  - `resultsVersion`: raw backend result-set changes
  - `displayedResultsVersion`: UI projection changes, including sort on/off flips
- `VirtualList` uses both:
  - `dataResultsVersion` resets hydrated row data
  - `displayedResultsVersion` resets viewport/icon tracking

## Row hydration
```text
VirtualList visible window [start, end]
  -> useDataLoader.ensureRangeLoaded(start, end)
  -> invoke('get_nodes_info', { results: slabIndices })
  -> cache rows by SlabIndex
```

- `useDataLoader` caches `SearchResultItem` by `SlabIndex`, not by visible row index.
- `versionRef` is bumped when `dataResultsVersion` changes; old fetches are ignored.
- `loadingRef` prevents duplicate fetches for the same slab index.
- `fromNodeInfo(...)` normalizes both legacy top-level fields and nested `metadata`.

## Icon updates
```text
VirtualList
  -> useIconViewport({ results: displayedResults, start, end })
  -> invoke('update_icon_viewport', { id, viewport })
  -> backend emits icon_update[]
  -> useDataLoader patches cached icons in place
```

- `useIconViewport` batches updates with `requestAnimationFrame`.
- It deduplicates unchanged ranges and sends an empty viewport once when the list becomes empty or unmounts.
- `iconOverridesRef` ensures pushed Quick Look thumbnails win over older `get_nodes_info` responses.

## Window-level event runtime
`cardinal/src/runtime/tauriEventRuntime.ts` registers shared listeners for:
- `status_bar_update`
- `app_lifecycle_state`
- `quick_launch`
- `fs_events_batch`
- `icon_update`
- `quicklook-keydown`
- Tauri drag-drop events from the current window

`useAppWindowListeners` wires those events into app state:
- status counters -> `useFileSearch`
- lifecycle -> `useFileSearch`
- quick launch -> focus/select the search input
- drag-drop -> quote the dropped path and route it to either file search or event filtering

## Recent FSEvents tab
- `useRecentFSEvents` keeps an in-memory buffer of up to `10000` recent events.
- The hook only forces re-rendering while the Events tab is active; otherwise it keeps buffering silently.
- Filtering happens in memory against both full path and basename, with optional case sensitivity.

## Quick Look flow
- `useSelection` owns the current file selection.
- `useQuickLook` turns selected paths into `QuickLookItemPayload[]`, including screen-space icon rects when the corresponding row DOM exists.
- `toggle_quicklook` / `update_quicklook` keep `QLPreviewPanel` in sync.
- Native Up/Down arrow handling inside Quick Look is reflected back to the UI through `quicklook-keydown`.

## Practical rule of thumb
- Commands are used for request/response work (`search`, `get_nodes_info`, `get_sorted_view`).
- Tauri events are used for push work (`status_bar_update`, `fs_events_batch`, `icon_update`, `quick_launch`).
- Version tokens exist at both the backend and React layers because either side can observe stale work independently.
