# Background Event Loop

This chapter explains how the background thread coordinates search, metadata expansion, rescans, and icon loading.

---

## Channel topology
Background threads communicate with Tauri via crossbeam channels initialized in `cardinal/src-tauri/src/lib.rs`:
```
UI -> Tauri commands -> background thread

[search_tx]            search requests (query + options + cancel token)
[result_rx]            search outcomes (Vec<SlabIndex> + highlights)
[node_info_tx]         slab indices needing path/metadata/icon (oneshot response channel)
[icon_viewport_tx]     visible slab indices for QuickLook icon prefetch
[icon_update_tx]       pushes base64 PNG icons back to UI (event: icon_update)
[rescan_tx]            manual rescan requests
[watch_config_tx]      watch root + ignore-path updates
[update_window_state_rx] window focus/visibility changes for idle flush
[finish_tx/finalizer]  flush cache once on exit
```

---

## Main loop
Entry: `run_background_event_loop` in `cardinal/src-tauri/src/background.rs`.
```
loop select! {
  finish_rx        => persist cache and return
  search_rx        => cache.search_with_options -> result_tx
  node_info_rx     => cache.expand_file_nodes   -> respond via NodeInfoRequest.response_tx
  icon_viewport_rx => spawn QuickLook jobs; send IconPayload via icon_update_tx
  rescan_rx        => perform_rescan(...)
  watch_config_rx  => rebuild cache + watcher for new root/ignore paths
  window_state_rx  => recompute foreground state for idle flush
  flush_ticker     => periodic idle/hide cache flush
  event_watcher    => handle_fs_events; maybe trigger rescan; forward new events to UI
}
```

Event loop sketch:
```text
                  ┌─────────────┐
 search_tx  ─────▶│ search_rx   │
 node_info_tx ───▶│ node_info_rx│
 icon_viewport_tx▶│ icon_viewport_rx
 rescan_tx  ─────▶│ rescan_rx   │
 finish_tx  ─────▶│ finish_rx   │
 EventWatcher ───▶│ event_watcher
                  └─────┬───────┘
                        │
                        ▼
        ┌────────────────────────────────────┐
        │ run_background_event_loop          │
        │  - SearchCache                     │
        │  - rescan_with_walk_data           │
        │  - fs_icon::icon_of_path_ql        │
        └─────┬──────────────────────────────┘
              │
              ├─ emit status_bar_update
              ├─ emit fs_events_batch
              └─ send IconPayload via icon_update_tx
```

---

## FSEvents and incremental updates
- `EventWatcher` (from `cardinal-sdk`) streams `FsEvent { path, flag, id }`.
- Flags such as `HistoryDone` flip the lifecycle to Ready through `update_app_state`.
- Each batch is applied via `cache.handle_fs_events`; on `HandleFSEError::Rescan`, a full rebuild is performed.
- Recent events are sorted by `(timestamp, event_id)` and emitted as `fs_events_batch` for UI activity panes.

---

## Rescan flow
```
perform_rescan:
  stop EventWatcher (noop)
  set state -> Initializing; emit status_bar_update(0,0)
  rebuild cache with WalkData (respect ignore_paths)
    - a helper thread emits progress every 100ms (num_files + num_dirs)
  restart EventWatcher from last_event_id
  set state -> Updating
```

- Rescans are cancellable: if `rescan_with_walk_data` returns `None`, the previous cache is retained and `EventWatcher` is reset to `noop`.

---

## Icon pipeline (backend side)
- `icon_viewport_rx` receives the visible slab indices from the UI.
- Each path is filtered (skips OneDrive/iCloud paths) and spawned on a Rayon thread:
  - `fs_icon::icon_of_path_ql` uses QuickLook to fetch a bitmap.
  - Icons are encoded as `data:image/png;base64,...` and sent via `icon_update_tx`.
- The UI’s `useDataLoader` listens to `icon_update` events to patch row icons without refetching full node info.

---

## Shutdown
- `RunEvent::Exit` or `ExitRequested` set `APP_QUIT/EXIT_REQUESTED`, then `flush_cache_to_file_once` sends a final cache through `finish_tx` for persistence.
- Window close requests for the main window are intercepted in `lib.rs`; unless exit has been requested, the window is hidden instead of closed so the background loop and index remain alive.
