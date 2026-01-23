# UI Dataflow

This chapter maps the React-side components and hooks to the IPC layer.

---

## Search execution
```
Search input change
  -> debounce (unless immediate) -> invoke('search', { query, options, version })
  -> receive { results: SlabIndex[], highlights }
  -> store results in state and pass to <VirtualList>
```

- Each `search` call carries a `version` used to build a `CancellationToken`.
- The UI can trigger immediate searches (initial load and Enter key) to bypass debounce.
- When a new query starts, older tokens are considered cancelled inside the engine; loops exit early and the Tauri command returns an empty `results` list for those searches.
- Independently, the React hook uses its own `searchVersionRef` to ignore any `search` responses whose `version` does not match the latest request.

---

## Row hydration
```
<VirtualList> computes visible range [start,end] with overscan
  -> useDataLoader.ensureRangeLoaded(start,end)
      -> invoke('get_nodes_info', { results: slice })
      -> cache rows by array index (guarded by versionRef)
  -> renderRow uses cached item (path, metadata, icon)
```

- `useDataLoader` increments `versionRef` whenever the results array identity changes.
- Responses for old versions are discarded to prevent stale rows from populating the cache.

End-to-end path overview:
```text
SearchBar (React)
  └─ queueSearch(query)
        ↓
   useFileSearch.handleSearch
        ↓
   invoke('search', { query, options, version })
        ↓
   Tauri command → SearchCache::search_with_options
        ↓
   { results: Vec<SlabIndex>, highlights }
        ↓
VirtualList (results)
  ├─ ensureRangeLoaded(start,end)
  │     → invoke('get_nodes_info', { results: slice })
  │     → cache rows
  └─ useIconViewport(start,end)
        → invoke('update_icon_viewport', { id, viewport })
        → backend emits icon_update → useDataLoader patches icons
```

---

## Icon pipeline
```
<VirtualList> useIconViewport(start,end)
  -> invoke('update_icon_viewport', { id, viewport: slab indices })
Backend QuickLook loads icons for those paths
  -> emits icon_update [{ slabIndex, icon }]
useDataLoader listens to icon_update
  -> map slabIndex -> row index; patch cache with override icon
```

- Icon updates only override the `icon` field of cached rows; other metadata stays intact.
- `iconOverridesRef` ensures that pushed icons win over older node-info results.

---

## Window and shortcuts
```
Global shortcut (Cmd+Shift+Space)
  -> invoke('toggle_main_window')
  -> backend activates window and emits quick_launch
  -> UI focuses search input upon quick_launch event
```

- `hide_main_window`, `activate_main_window`, and `toggle_main_window` are also bound to menu items and escape keys for full keyboard control.

---

## Adding new flows
- Define Tauri commands first and document them in `ipc-commands.md`.
- Create hooks that encapsulate IPC details and expose a clean React interface.
- Use versioned state or tokens for any long-running or cancellable work.
- Prefer event listeners for push-style updates to avoid polling loops in React.
