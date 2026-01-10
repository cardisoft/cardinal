# Runtime Lifecycle

This chapter describes Cardinal’s lifecycle states and how they are surfaced to the UI.

---

## States
- **Initializing**: initial scan or rebuilding; UI should expect partial data.
- **Updating**: background loop is live after a scan/rebuild and is waiting for the FSEvents history boundary (HistoryDone) before marking Ready.
- **Ready**: steady state; FSEvents-driven incremental updates.

Implementation: `cardinal/src-tauri/src/lifecycle.rs`
- `APP_LIFECYCLE_STATE` is an atomic `u8`; helpers emit `app_lifecycle_state` events to the UI.
- `APP_QUIT` and `EXIT_REQUESTED` are atomics that guard shutdown ordering.

---

## Emission
- `load_app_state` reads the current `AppLifecycleState`.
- `store_app_state` writes new values and establishes the ordering for the rest of the app.
- `update_app_state` changes the state only when it actually differs, then calls `emit_app_state`.
- `emit_app_state` sends the current state as a string over the `app_lifecycle_state` Tauri event.

The frontend can subscribe to `app_lifecycle_state` to gate features or show banners while indexing or rescans are underway.

---

## Debugging tips
- If the UI appears stuck in “Initializing”, inspect logs for FSEvent errors or rescan loops.
- Use the event stream for `app_lifecycle_state` to confirm transitions line up with status bar output and search readiness.
