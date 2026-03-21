# FS Events SDK (cardinal-sdk)

`cardinal-sdk/` is the macOS-only wrapper around FSEvents used by Cardinal's background loop.

## Public surface
Re-exports from `cardinal-sdk/src/lib.rs`:
- `FsEvent`
- `EventFlag`, `EventType`, `ScanType`
- `EventStream`, `EventWatcher`
- `FSEventStreamEventId`
- `current_event_id()` and `event_id_to_timestamp()`

## Event semantics
- `EventFlag::event_type()` classifies a batch item as file, dir, symlink, hardlink, or unknown.
- `EventFlag::scan_type()` reduces raw FSEvent flags to Cardinal's four-way handling model:
  - `SingleNode`
  - `Folder`
  - `ReScan`
  - `Nop`

Current rules are intentionally simple:
- `HistoryDone` and `EventIdsWrapped` -> `Nop`
- `RootChanged` -> `ReScan`
- directory-like events -> `Folder`
- everything else -> `SingleNode`

`FsEvent::should_rescan(root)` adds one extra rule: any root-level mutation also forces a full rescan.

## EventStream
`EventStream::new(...)`:
- builds the watched path `CFArray`
- boxes a Rust callback inside `FSEventStreamContext`
- creates the stream with:
  - `kFSEventStreamCreateFlagNoDefer`
  - `kFSEventStreamCreateFlagFileEvents`
  - `kFSEventStreamCreateFlagWatchRoot`

`EventStream::spawn()` attaches the stream to a serial GCD queue and starts it. Dropping `EventStreamWithQueue` stops and invalidates the stream.

## EventWatcher
`EventWatcher` is the ergonomic wrapper used by the rest of the app. It contains:
- `Receiver<Vec<FsEvent>>`
- a cancellation sender that keeps the worker thread alive until drop

`EventWatcher::spawn(path, since_event_id, latency, ignore_paths)`:
- starts an `EventStream`
- filters ignored paths before delivery
- keeps `HistoryDone` even when the path itself would otherwise be ignored
- returns `(dev_t, EventWatcher)`

`EventWatcher::noop()` returns a shared inert watcher whose receiver times out instead of disconnecting. Cardinal uses it during cancelled scans and temporary watcher shutdowns.

## Helpers
- `current_event_id()` captures the system-wide current event id.
- `event_id_to_timestamp()` uses repeated `FSEventsGetLastEventIdForDeviceBeforeTime(...)` calls to approximate a wall-clock time for diagnostics.

## Integration with Cardinal
- `lib.rs` / `background.rs` start the watcher at `cache.last_event_id()`.
- `background.rs` consumes batches directly in `crossbeam_channel::select!`.
- `search-cache` decides whether a batch can be handled incrementally or must surface `HandleFSEError::Rescan`.
