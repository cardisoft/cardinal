# Runtime Lifecycle

Cardinal exposes a small lifecycle state machine to the UI. The state is stored in `cardinal/src-tauri/src/lifecycle.rs` and broadcast as the `app_lifecycle_state` Tauri event.

## States
- **Initializing**: no usable steady-state watcher yet. This covers the initial filesystem walk, full rescans, and watch-config rebuilds.
- **Updating**: a cache exists and the `EventWatcher` is running, but the app is still waiting for the FSEvents history boundary (`HistoryDone`).
- **Ready**: steady state. Search, incremental updates, and periodic cache flushes all operate normally.

There is intentionally no separate `Stopped` or `Error` state. If background logic never starts because `start_logic` is not sent, the process simply remains at `Initializing`.

## Where transitions happen
- **Process boot**: `APP_LIFECYCLE_STATE` starts at `Initializing`.
- **Initial startup**:
  - an index worker (`spawn_index_load`) loads the cache from disk or walks the filesystem
  - the loop installs its `IndexBuildResult` and sets `Updating` when it arrives
  - first `HistoryDone` batch -> `handle_event_watcher_events` sets `Ready`
- **Manual rescan or watch-config change**:
  - `perform_rescan` / `handle_watch_config_update` set `Initializing`, reset counters, then rebuild (asynchronously via `spawn_index_load` for watch-config changes)
  - installation of the rebuilt cache moves the app back to `Updating`
  - the next `HistoryDone` promotes the app back to `Ready`

While the state is `Initializing` the backend is fully responsive: searches answer
`INDEXING` (status code 2) instead of blocking, and literal folder scopes are served from a
temporary index. The frontend re-runs the current query once the state leaves
`Initializing`.

## Implementation details
- `APP_LIFECYCLE_STATE: AtomicU8` stores the enum value.
- `APP_QUIT` and `EXIT_REQUESTED` coordinate shutdown and cache-flush ordering.
- `load_app_state()` returns the current state.
- `store_app_state()` writes the atomic with release ordering.
- `update_app_state()` is the guarded transition helper; it emits only when the state actually changes.
- `emit_app_state()` publishes the string form (`"Initializing"`, `"Updating"`, `"Ready"`) over Tauri.

## Frontend consumers
- `useFileSearch` fetches the current state once via `get_app_status()` on startup.
- `useAppWindowListeners` subscribes to `app_lifecycle_state` and updates React state incrementally.
- The UI combines lifecycle state with `status_bar_update` counters to decide when to show indexing/rebuild progress.

## Shutdown and persistence
- `flush_cache_to_file_once()` refuses to flush only while the app is `Initializing`; a
  complete cache that is still replaying history (`Updating`) is persisted.
- A freshly built index is persisted by the index worker before the loop installs it, so a run
  that is quit before `Ready` does not force a full re-walk on the next launch.
- On close/exit, `APP_QUIT` is set first, then the background loop is asked for the final cache snapshot.
- The loop only returns a snapshot when no index build is in flight and the cache is not a noop,
  so a temporary folder-scope index never replaces the persisted full index.
