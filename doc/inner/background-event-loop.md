# Background Event Loop

`cardinal/src-tauri/src/background.rs` owns the long-lived background worker. It is the only place where the live `SearchCache`, `EventWatcher`, and periodic flush logic coexist.

## Process topology
- `lib.rs` creates crossbeam channels for search jobs, node-info requests, icon viewport updates, rescan/watch-config changes, window-state nudges, and final cache handoff.
- A dedicated thread batches `icon_update` payloads and emits them to the UI roughly every 100 ms.
- The main logic thread waits for `start_logic(...)` before doing any indexing work.

## Channels
```text
search_rx             query + SearchOptionsPayload + CancellationToken + result_tx
                      result_tx returns SearchJobResult
                      (Completed(Result<SearchOutcome>) | Indexing)

node_info_rx          slab indices -> SearchResultNode expansion
icon_viewport_rx      visible slab indices for thumbnail prefetch
icon_update_tx        IconPayload batches back to the UI

rescan_rx             manual full-rescan requests
watch_config_rx       watch-root / ignore-path changes
index_build_rx        IndexBuildResult from the index worker thread
update_window_state_rx
                      recompute foreground/hidden state for delayed flush
finish_rx             one-shot final cache handoff during shutdown
```

## Startup path
1. `wait_for_logic_start()` blocks until the frontend has permission and sends `start_logic`.
2. `run_logic_thread()` normalizes the watch config and immediately enters
   `run_background_event_loop(...)` with a **noop** cache and `Some(IndexingState { .. })`.
3. `spawn_index_load(...)` runs on its own thread: it tries
   `SearchCache::try_read_persistent_cache(...)` and falls back to `build_search_cache(...)`
   (which emits progress every 100 ms). Manual rescans pass `force_walk = true`.
4. The worker sends `IndexBuildResult` over `index_build_tx`; the loop installs it only when
   the build token still matches the active one (a cancelled/superseded walk is ignored).
5. On install: `indexing = None`, the launch snapshot is dropped, the lifecycle moves to
   `Updating`, and an `EventWatcher` starts at `cache.last_event_id()`. `HistoryDone` then
   moves the lifecycle to `Ready`.

## Searching while the index builds
- With `indexing.is_some()` the loop never blocks on the walk. A request returns
  `SearchJobResult::Indexing` (status `2`) or `SearchJobResult::ScopedIndexing { scope }`
  (status `3`), and the frontend retries.
- **Literal folder scopes** are walked on a dedicated `cardinal-scoped-index` thread:
  - `parse_scope_path` only expands `~`/quotes and never touches the disk; a network mount can
    block for seconds, so `is_dir()` runs on the worker.
  - `decide_search_action(scope, indexing, scoped, covered_by_full)` is the pure precedence
    rule: installed scope index → full index that covers the folder → start a new walk →
    wait. It is unit-tested because getting the order wrong makes scope searches return
    nothing.
  - One walk runs at a time. Changing the scope cancels the previous walk through its own
    leaked `AtomicBool`, so the full index is never cancelled by a scope change.
- **Slab indices are cache-local**, so a scope index temporarily *replaces* the active cache
  and the full index is parked in `ScopedIndexState::parked_full`. Any query that does not
  need the scope index restores it and restarts the `EventWatcher` from its `last_event_id`
  (FSEvents replays anything missed).
- While a scope index is active the watcher is `noop()`: FSEvents must not be applied to a
  cache whose slab does not contain those paths.
- A scope inside an ignored path (e.g. `/Volumes` is ignored by default) is *never* part of
  the full index. After indexing finishes such a scope still gets its own temporary index,
  because `node_index_for_path` returns `None` for it.
- `ignore_paths_for_scope` drops ignore entries that are ancestors of the scope. Without it a
  scope like `/Volumes/nfs_D` would filter out every child and produce an empty index (there
  is a regression test for exactly that).
- When the full index finishes while a scope index is active, the full index is parked instead
  of replacing it: swapping would invalidate the rows on screen and force another walk of the
  folder. The full index is installed by the next query that does not need the scope.
- Cache flushes and the shutdown snapshot are refused while a scope index is active, so a
  temporary index can never overwrite `cardinal.db`.
- Scope snapshots live in `<app config>/scopes/scope-<fnv1a(path)>.db`:
  - reused when younger than `SCOPE_CACHE_TTL` (30 min) and written with the same ignore list
  - written after a walk when the index is at most `SCOPE_CACHE_MAX_NODES` (500k) entries
  - snapshots untouched for `SCOPE_CACHE_RETENTION` (7 days) are pruned
  - the hash is FNV-1a, not `DefaultHasher`, so an upgrade cannot invalidate every snapshot
- The frontend polls every `SCOPE_INDEX_POLL_MS` (800 ms) while `indexingScope` is set, and
  re-runs the query when the lifecycle leaves `Initializing`.

## Flush behavior
- Flush checks run every 10 seconds.
- Fresh-index persistence:
  - a freshly walked (or re-walked) index is written by `spawn_index_load` on its worker thread
    before it is handed to the loop, so the first run is durable even if it is quit before
    `Ready`
  - a superseded build skips the write and leaves it to the newer build
- Hide flush:
  - when the main window leaves the foreground, a two-tick countdown starts
  - if the window is still hidden/unfocused on the second tick, the cache snapshot is written
- Idle flush:
  - `search_activity::note_search_activity()` records the last search time
  - after 5 minutes without searches, the next tick writes a snapshot
- Hide/idle flushes only run while the lifecycle is `Ready`.

## Shutdown
- `RunEvent::Exit` / `ExitRequested` set `APP_QUIT`.
- `flush_cache_to_file_once()` requests the final cache over `finish_rx`.
- No snapshot is written if the cache is still noop or the app never became `Ready`.
