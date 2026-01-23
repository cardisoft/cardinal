# FS Events SDK (cardinal-sdk)

This chapter documents the `cardinal-sdk/` crate, which wraps macOS FSEvents for Cardinal.

---

## Public surface

`cardinal-sdk/src/lib.rs` re-exports:
- `FsEvent` — a single filesystem event (path, flag, id).
- `EventFlag`, `EventType`, `ScanType` — bitflags and enums describing event semantics.
- `EventStream`, `EventWatcher` — types that own the FSEvent stream and dispatch queue.
- `FSEventStreamEventId` — underlying event ID type.
- Helpers from `utils`:
  - `current_event_id()` — current FSEvent ID for the system.
  - `event_id_to_timestamp()` — convert event IDs into wall-clock timestamps.

`SearchCache` and the Tauri backend use these to track incremental changes and rescan boundaries.

---

## EventStream and dispatch queue

`EventStream` wraps a raw `FSEventStreamRef`:

- `EventStream::new(paths, since_event_id, latency, callback)`:
  - Creates a `CFArray` of watch paths.
  - Configures `FSEventStreamContext` with a boxed Rust callback (`EventsCallback`).
  - Calls `FSEventStreamCreate` with flags:
    - `kFSEventStreamCreateFlagNoDefer`
    - `kFSEventStreamCreateFlagFileEvents`
    - `kFSEventStreamCreateFlagWatchRoot`
  - The callback converts raw C pointers into a `Vec<FsEvent>` and invokes the Rust closure.
- `spawn`:
  - Attaches the stream to a serial `DispatchQueue`.
  - Starts the stream (`FSEventStreamStart`) and returns `EventStreamWithQueue`.
  - On failure, stops and invalidates the stream.
- `dev`:
  - Returns the `dev_t` for the device being watched via `FSEventStreamGetDeviceBeingWatched`.

`EventStreamWithQueue` stops and invalidates the stream on drop, ensuring proper resource cleanup.

Creation flow:
```text
paths (&[&str]) + since_event_id + latency
        │
        ▼
 EventStream::new(...)
        │   (wrap Rust closure as EventsCallback,
        │    configure FSEventStreamContext)
        ▼
 FSEventStreamCreate(...)
        │
        ▼
 EventStream::spawn()
        │   (attach to DispatchQueue, start stream)
        ▼
 EventStreamWithQueue
        │
        └─ incoming C callbacks → Vec<FsEvent> → Rust closure
```

---

## EventWatcher

`EventWatcher` is the high-level type used by the rest of the codebase:

- Holds:
  - `receiver: Receiver<Vec<FsEvent>>` — batches of events.
  - `_cancellation_token: Sender<()>` — used to end the watcher thread.
- Implements `Deref<Target = Receiver<Vec<FsEvent>>>` so callers can use `recv` or `select!` directly.

### Spawning a watcher

```rust
let (dev, watcher) = EventWatcher::spawn(path, since_event_id, latency);
```

- Creates a bounded cancellation channel and an unbounded events channel.
- Builds an `EventStream` with a callback that sends event batches into the `sender`.
- Spawns a thread that:
  - Calls `stream.spawn()` to start the FSEvent stream attached to a dispatch queue.
  - Blocks on the cancellation receiver, keeping the stream active until dropped.

`EventWatcher::noop()` returns a watcher with a disconnected receiver and a dummy cancellation token, used when rescans are cancelled or disabled.

---

## Tests and guarantees

The `drop_then_respawn_event_watcher_delivers_events` test ensures:

- Dropping an `EventWatcher` properly tears down the underlying FSEvent stream.
- A new watcher on the same path receives fresh events as expected.

This behavior is critical for rescan flows and for cases where the watcher must be restarted after errors.

---

## Integration notes

- The Tauri backend uses `EventWatcher::spawn` to:
  - Feed `SearchCache::handle_fs_events` with live changes.
  - Trigger `perform_rescan` when FSEvent flags or paths suggest the index may be out of sync.
  - Compute a minimal set of paths to rescan (`scan_paths`) based on `ScanType` and ancestry rather than trusting individual “create/delete/modify” flags, which can arrive in unexpected combinations.
- `current_event_id()` is used to:
  - Capture the event ID at the time of a full walk, so subsequent watchers can start from a known boundary.
  - Advance `last_event_id` in the cache as events are processed.
