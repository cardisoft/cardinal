# Background Event Loop

`cardinal/src-tauri/src/background.rs` owns the long-lived background worker. It is the only place where the live `SearchCache`, `EventWatcher`, and periodic flush logic coexist.

## Process topology
- `lib.rs` creates crossbeam channels for search jobs, node-info requests, icon viewport updates, rescan/watch-config changes, window-state nudges, and final cache handoff.
- A dedicated thread batches `icon_update` payloads and emits them to the UI roughly every 100 ms.
- The main logic thread waits for `start_logic(...)` before doing any indexing work.

## Channels
```text
search_rx             query + SearchOptionsPayload + CancellationToken + result_tx
                      result_tx returns SearchOutcome back to the command handler

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
Every snapshot rewrites the whole cache file (~100 MB for a full-disk index), so writes are scheduled as *checkpoints* (`search_activity.rs`, `background.rs::decide_flush`). Before this policy existed an idle Mac rewrote the file every 5 minutes, again right after every wake, and on every hide of the window (8.6 GB/day was observed).

- Flush checks run every 10 seconds, only while the lifecycle is `Ready`.
- A checkpoint is **due** when `SearchCache::is_dirty()` (something changed since the last load/flush: FSEvents applied, removals, rescans, rescan counter, lazily fetched metadata) and at least `CHECKPOINT_INTERVAL` (6 hours) has passed since the previous write attempt. Failed attempts count, so a broken disk cannot cause a retry storm. A freshly walked index has never been persisted, so its baseline is due immediately; a snapshot loaded at startup starts the interval at launch.
- A due checkpoint is written at the next **quiet moment**: the window was just hidden (two-tick countdown, 10-20 s), or no search has run for 5 minutes, or, once the checkpoint is already waiting, a 30-second gap between searches. If no quiet moment occurs within `CHECKPOINT_DEFERRAL` (5 minutes) it is written anyway, so continuous searching cannot starve persistence.
- A clean index is never written, including at exit.
- All clocks are monotonic (`Instant`, `CLOCK_UPTIME_RAW` on macOS), so time asleep never counts as idle time and never brings a checkpoint forward.
- Worst case under continuous change: about 4 automatic snapshots per awake day (~400 MB) plus one at a dirty exit.
- Foreground state comes from an atomic maintained on the main thread (`window_controls::main_window_is_foreground`); the background loop never calls blocking window getters.

## Shutdown
- `RunEvent::Exit` / `ExitRequested` set `APP_QUIT`.
- `flush_cache_to_file_once()` requests the final cache over `finish_rx`.
- No snapshot is written if the cache is still noop or the app never became `Ready`.
