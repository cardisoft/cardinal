# Background Event Loop

`cardinal/src-tauri/src/background.rs` owns the long-lived background worker. It is the only place where the live `SearchCache`, `EventWatcher`, and periodic flush logic coexist.

## Process topology
- `lib.rs` creates crossbeam channels for search jobs, node-info requests, icon viewport updates, rescan/watch-config changes, window-state nudges, and final cache handoff.
- A dedicated thread batches `icon_update` payloads and emits them to the UI roughly every 100 ms.
- The main logic thread waits for `start_logic(...)` before doing any indexing work.

## Channels
```text
search_rx             query + SearchOptionsPayload + CancellationToken
result_tx             SearchOutcome back to the command handler

node_info_rx          slab indices -> SearchResultNode expansion
icon_viewport_rx      visible slab indices for thumbnail prefetch
icon_update_tx        IconPayload batches back to the UI

rescan_rx             manual full-rescan requests
watch_config_rx       watch-root / ignore-path changes
update_window_state_rx
                      recompute foreground/hidden state for delayed flush
finish_rx             one-shot final cache handoff during shutdown
```

## Startup path
1. `wait_for_logic_start()` blocks until the frontend has permission and sends `start_logic`.
2. `run_logic_thread()` normalizes the watch config.
3. The backend tries `SearchCache::try_read_persistent_cache(...)`.
4. If that fails, `build_search_cache(...)` runs `fswalk` with a scan cancellation token and emits progress every 100 ms.
5. Once a non-noop cache exists, the lifecycle moves to `Updating` and an `EventWatcher` starts at `cache.last_event_id()`.

## Main select loop
```text
finish_rx              -> return the final cache snapshot
update_window_state_rx -> recompute foreground status
flush_ticker           -> idle/hide flush checks
search_rx              -> cache.search_with_options(...)
node_info_rx           -> cache.expand_file_nodes(...)
icon_viewport_rx       -> Quick Look thumbnail jobs
rescan_rx              -> perform_rescan(...)
watch_config_rx        -> rebuild cache/watcher for new config
event_watcher          -> apply FSEvents, emit recent activity, advance lifecycle
```

## FSEvents handling
- `handle_event_watcher_events(...)` increments `processed_events` and emits `status_bar_update`.
- `HistoryDone` flips `history_ready` and moves the lifecycle to `Ready`.
- Only events received after `history_ready` are forwarded to the UI as `fs_events_batch`.
- `SearchCache::handle_fs_events(...)` performs incremental subtree rescans through `scan_path_recursive(...)`.
- If the cache returns `HandleFSEError::Rescan`, Cardinal increments `rescan_count` and surfaces that in the status bar. It does **not** automatically run a full rebuild today.

## Rescans and watch-config changes
- `trigger_rescan()` sends a new scan token to `perform_rescan(...)`.
- A manual rescan:
  - swaps the watcher to `EventWatcher::noop()`
  - resets lifecycle and progress counters
  - rebuilds the current watch root with `cache.rescan_with_walk_data(...)`
  - restarts the watcher if the rebuild completed
- If a manual rescan is cancelled, the old cache is kept, but the watcher remains `noop()` until a later rebuild/restart.
- A watch-config change is stricter:
  - paths are renormalized in `commands.rs`
  - a cancelled rebuild falls back to `SearchCache::noop(...)`
  - a successful rebuild replaces both cache and watcher completely

## Icon pipeline
- `update_icon_viewport` sends the visible `SlabIndex` slice to the background loop.
- `handle_icon_viewport_update(...)` expands those indices to paths, drops cloud-backed paths such as OneDrive/iCloud/Google Drive/Dropbox, and spawns Rayon jobs.
- Each job calls `fs_icon::icon_of_path_ql(...)` and pushes a base64 PNG through `icon_update_tx`.
- `lib.rs` batches those payloads and emits a single `icon_update` event containing an array.

## Flush behavior
- Flush checks run every 10 seconds.
- Hide flush:
  - when the main window leaves the foreground, a two-tick countdown starts
  - if the window is still hidden/unfocused on the second tick, the cache snapshot is written
- Idle flush:
  - `search_activity::note_search_activity()` records the last search time
  - after 5 minutes without searches, the next tick writes a snapshot
- Flushes only run while the lifecycle is `Ready`.

## Shutdown
- `RunEvent::Exit` / `ExitRequested` set `APP_QUIT`.
- `flush_cache_to_file_once()` requests the final cache over `finish_rx`.
- No snapshot is written if the cache is still noop or the app never became `Ready`.
