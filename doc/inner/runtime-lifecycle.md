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
  - cache loaded from disk or fresh walk completed -> `run_logic_thread` sets `Updating`
  - first `HistoryDone` batch -> `handle_event_watcher_events` sets `Ready`
- **Manual rescan or watch-config change**:
  - `perform_rescan` / `handle_watch_config_update` set `Initializing`, reset counters, rebuild state, then move back to `Updating`
  - the next `HistoryDone` promotes the app back to `Ready`

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
- `flush_cache_to_file_once()` refuses to flush if the app never reached `Ready`.
- On close/exit, `APP_QUIT` is set first, then the background loop is asked for the final cache snapshot.
