# Cardinal Project Overview

Cardinal is a macOS desktop file search app with a React/Tauri UI and a Rust indexing engine. The codebase splits cleanly into three layers: frontend UI, the Tauri command shell, and a long-lived background indexing loop.

## High-level architecture
- **Frontend (`cardinal/src`)**: search UI, files/events tabs, preferences, tray/menu integration, Quick Look coordination, and shared Tauri event subscriptions.
- **Tauri shell (`cardinal/src-tauri`)**: registers commands/plugins, owns crossbeam channels, exposes native window/tray/clipboard/Quick Look operations, and waits for `start_logic(...)` before any indexing work begins.
- **Background loop (`cardinal/src-tauri/src/background.rs`)**: owns `SearchCache`, `EventWatcher`, rescans, icon prefetch, status events, and periodic cache flushes.

```text
React UI
  ├─ invoke(search / get_nodes_info / get_sorted_view / ...)
  └─ listen(status_bar_update / app_lifecycle_state / fs_events_batch /
            icon_update / quick_launch / quicklook-keydown)
        │
        ▼
Tauri shell
  ├─ commands.rs
  ├─ window_controls.rs / quicklook.rs / sort.rs
  └─ crossbeam channels into background.rs
        │
        ▼
Background loop
  ├─ SearchCache
  ├─ cardinal-sdk EventWatcher
  ├─ fswalk initial/full scan
  └─ fs-icon viewport thumbnail workers
```

## Startup sequence
1. `cardinal/src/main.tsx` boots the UI, theme, menu, and tray helpers.
2. `App.tsx` checks Full Disk Access, loads watch preferences, and calls `start_logic(watchRoot, ignorePaths)` once permission is granted.
3. `cardinal/src-tauri/src/lib.rs` waits on `LOGIC_START`, then either loads the persistent cache or builds a fresh `SearchCache`.
4. The background thread starts an `EventWatcher` at `cache.last_event_id()` and moves the app lifecycle from `Initializing` to `Updating`, then to `Ready` after `HistoryDone`.

## Main user flows
- **Search**: `useFileSearch` invokes `search`; `SearchCache::search_with_options` parses the query, evaluates it, and returns slab indices plus highlight terms.
- **Row hydration**: `VirtualList` asks `get_nodes_info` only for visible rows; `useDataLoader` caches hydrated rows by `SlabIndex`.
- **Sorting**: `useRemoteSort` optionally calls `get_sorted_view`; the backend expands nodes, sorts them, and returns reordered `SlabIndex` values.
- **Icons**: `get_nodes_info` supplies baseline NSWorkspace icons; `update_icon_viewport` triggers higher-fidelity Quick Look thumbnails for the current viewport.
- **Recent activity**: the background loop forwards post-history FSEvent batches to the UI, where `useRecentFSEvents` keeps an in-memory buffer.
- **Quick launch / window control**: global shortcut, tray, and menu actions call `toggle_main_window`, `activate_main_window`, or `hide_main_window`.
- **Quick Look**: the frontend computes row screen rects, sends them through `toggle_quicklook` / `update_quicklook`, and receives arrow-key navigation back through `quicklook-keydown`.

## Important workspace crates
- `search-cache/`: slab-backed index, query evaluation, persistence, incremental updates.
- `cardinal-sdk/`: macOS FSEvents wrapper (`EventStream`, `EventWatcher`, `EventFlag`).
- `fswalk/`: parallel filesystem walk used for initial scans and full rescans.
- `fs-icon/`: NSWorkspace icons and Quick Look thumbnails as PNG bytes.
- `cardinal-syntax/`: Everything-style query parser and optimizer.
- `query-segmentation/`: slash-aware segment parsing for path-like matching.
- `search-cancel/`: versioned cancellation tokens for searches and rescans.
- `namepool/`: string interner used by `search-cache::NAME_POOL`.
- `file-tags/`: Finder tag reading and Spotlight (`mdfind`) fallback for `tag:` filters.
- `slab-mmap/`: mmap-backed slab primitives used by persistence-oriented storage.
- `lsf/` and `was/`: CLI helpers for `SearchCache` and FSEvents debugging.

## Operational notes
- Background logic is gated by Full Disk Access. If permission is denied, the UI still loads but indexing does not start.
- Watch config normalization is centralized in `commands.rs`; `/System/Volumes/Data` is always added to the ignore list.
- Cache snapshots are flushed on hide/idle and once on exit, but only after the lifecycle reaches `Ready`.
- `HandleFSEError::Rescan` currently increments a counter and surfaces the need for a rebuild; the background loop does not automatically force a full rescan.
