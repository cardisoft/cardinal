use crate::FsEvent;
use crossbeam_channel::{Receiver, Sender, bounded, unbounded};
use dispatch2::{DispatchQueue, DispatchQueueAttr, DispatchRetained};
use libc::dev_t;
use objc2_core_foundation::{CFArray, CFString, CFTimeInterval};
use objc2_core_services::{
    ConstFSEventStreamRef, FSEventStreamContext, FSEventStreamCreate, FSEventStreamEventFlags,
    FSEventStreamEventId, FSEventStreamGetDeviceBeingWatched, FSEventStreamInvalidate,
    FSEventStreamRef, FSEventStreamRelease, FSEventStreamSetDispatchQueue, FSEventStreamStart,
    FSEventStreamStop, kFSEventStreamCreateFlagFileEvents, kFSEventStreamCreateFlagNoDefer,
    kFSEventStreamCreateFlagWatchRoot,
};
use std::{
    ffi::c_void,
    ops::{Deref, DerefMut},
    ptr::NonNull,
    slice,
    sync::LazyLock,
};

type EventsCallback = Box<dyn FnMut(Vec<FsEvent>) + Send>;

pub struct EventStream {
    stream: FSEventStreamRef,
}

unsafe impl Send for EventStream {}

impl Drop for EventStream {
    fn drop(&mut self) {
        unsafe {
            FSEventStreamRelease(self.stream);
        }
    }
}

impl EventStream {
    pub fn new(
        paths: &[&str],
        since_event_id: FSEventStreamEventId,
        latency: CFTimeInterval,
        callback: EventsCallback,
    ) -> Self {
        unsafe extern "C-unwind" fn drop_callback(info: *const c_void) {
            let _cb: Box<EventsCallback> = unsafe { Box::from_raw(info as _) };
        }

        unsafe extern "C-unwind" fn raw_callback(
            _stream: ConstFSEventStreamRef, // ConstFSEventStreamRef streamRef
            callback_info: *mut c_void,     // void *clientCallBackInfo
            num_events: usize,              // size_t numEvents
            event_paths: NonNull<c_void>,   // void *eventPaths
            event_flags: NonNull<FSEventStreamEventFlags>, // const FSEventStreamEventFlags eventFlags[]
            event_ids: NonNull<FSEventStreamEventId>,      // const FSEventStreamEventId eventIds[]
        ) {
            let event_paths = unsafe {
                slice::from_raw_parts(event_paths.as_ptr() as *const *const i8, num_events)
            };
            let event_flags = unsafe { slice::from_raw_parts(event_flags.as_ptr(), num_events) };
            let event_ids = unsafe { slice::from_raw_parts(event_ids.as_ptr(), num_events) };
            let events: Vec<_> = event_paths
                .iter()
                .zip(event_flags)
                .zip(event_ids)
                .map(|((&path, &flag), &id)| unsafe { FsEvent::from_raw(path, flag, id) })
                .collect();

            let callback = unsafe { (callback_info as *mut EventsCallback).as_mut() }.unwrap();
            callback(events);
        }

        let paths: Vec<_> = paths.iter().map(|&x| CFString::from_str(x)).collect();
        let paths = CFArray::from_retained_objects(&paths);
        let mut context = FSEventStreamContext {
            version: 0,
            info: Box::leak(Box::new(callback)) as *mut _ as *mut _,
            retain: None,
            release: Some(drop_callback),
            copyDescription: None,
        };

        let stream: FSEventStreamRef = unsafe {
            FSEventStreamCreate(
                None,
                Some(raw_callback),
                &mut context,
                paths.as_opaque(),
                since_event_id,
                latency,
                kFSEventStreamCreateFlagNoDefer
                    | kFSEventStreamCreateFlagFileEvents
                    | kFSEventStreamCreateFlagWatchRoot,
            )
        };
        Self { stream }
    }

    // Start the FSEventStream with a dispatch queue.
    pub fn spawn(self) -> Option<EventStreamWithQueue> {
        let queue = DispatchQueue::new("cardinal-sdk-queue", DispatchQueueAttr::SERIAL);
        unsafe { FSEventStreamSetDispatchQueue(self.stream, Some(&queue)) };
        let result = unsafe { FSEventStreamStart(self.stream) };
        if !result {
            unsafe { FSEventStreamStop(self.stream) };
            unsafe { FSEventStreamInvalidate(self.stream) };
            return None;
        }
        let stream = self.stream;
        Some(EventStreamWithQueue { stream, queue })
    }

    // Get device id being watched by this event stream.
    pub fn dev(&self) -> dev_t {
        unsafe { FSEventStreamGetDeviceBeingWatched(self.stream.cast_const()) }
    }
}

/// FSEventStream with dispatch queue.
///
/// Dropping this struct will stop the FSEventStream and release the dispatch queue.
pub struct EventStreamWithQueue {
    stream: FSEventStreamRef,
    #[allow(dead_code)]
    queue: DispatchRetained<DispatchQueue>,
}

impl Drop for EventStreamWithQueue {
    fn drop(&mut self) {
        unsafe {
            FSEventStreamStop(self.stream);
            FSEventStreamInvalidate(self.stream);
        }
    }
}

pub struct EventWatcher {
    receiver: Receiver<Vec<FsEvent>>,
    _cancellation_token: Sender<()>,
}

impl Deref for EventWatcher {
    type Target = Receiver<Vec<FsEvent>>;

    fn deref(&self) -> &Self::Target {
        &self.receiver
    }
}

impl DerefMut for EventWatcher {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.receiver
    }
}

impl EventWatcher {
    pub fn noop() -> Self {
        #[allow(clippy::type_complexity)]
        static BLACK_HOLE1: LazyLock<(Sender<Vec<FsEvent>>, Receiver<Vec<FsEvent>>)> =
            LazyLock::new(unbounded);
        static BLACK_HOLE2: LazyLock<(Sender<()>, Receiver<()>)> = LazyLock::new(|| bounded(1));
        Self {
            receiver: BLACK_HOLE1.1.clone(),
            _cancellation_token: BLACK_HOLE2.0.clone(),
        }
    }

    pub fn spawn(
        path: String,
        since_event_id: FSEventStreamEventId,
        latency: f64,
    ) -> (dev_t, EventWatcher) {
        let (_cancellation_token, cancellation_token_rx) = bounded::<()>(1);
        let (sender, receiver) = unbounded();
        let stream = EventStream::new(
            &[&path],
            since_event_id,
            latency,
            Box::new(move |events| {
                let _ = sender.send(events);
            }),
        );
        let dev = stream.dev();
        std::thread::Builder::new()
            .name("cardinal-sdk-event-watcher".to_string())
            .spawn(move || {
                let _stream_and_queue = stream.spawn().expect("failed to spawn event stream");
                let _ = cancellation_token_rx.recv();
            })
            .unwrap();
        (
            dev,
            EventWatcher {
                receiver,
                _cancellation_token,
            },
        )
    }
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;
    use crate::{EventFlag, utils::current_event_id};
    use crossbeam_channel::RecvTimeoutError;
    use std::time::{Duration, Instant};
    use tempfile::tempdir;

    #[test]
    fn noop_event_watcher_recv_timeout_never_disconnects() {
        let watcher = EventWatcher::noop();
        let result = watcher.recv_timeout(Duration::from_millis(50));
        assert!(
            matches!(result, Err(RecvTimeoutError::Timeout)),
            "noop watcher should block waiting for events instead of disconnecting"
        );
    }

    #[test]
    fn event_watcher_on_non_existent_path() {
        let (_dev, watcher) = EventWatcher::spawn("/e mm".to_string(), current_event_id(), 0.05);
        let initial_events = watcher.recv().unwrap();
        assert!(initial_events.len() == 1);
        assert!(initial_events[0].flag.contains(EventFlag::HistoryDone));

        let deadline = Instant::now() + Duration::from_secs(2);
        let mut received_any = false;
        while Instant::now() < deadline {
            match watcher.recv_timeout(Duration::from_millis(200)) {
                Ok(_batch) => {
                    received_any = true;
                    break;
                }
                Err(RecvTimeoutError::Timeout) => continue,
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }

        drop(watcher);
        assert!(
            !received_any,
            "event watcher on non-existent path should not deliver events"
        );
    }

    #[test]
    fn drop_then_respawn_event_watcher_delivers_events() {
        let temp_dir = tempdir().expect("failed to create tempdir");
        let watched_root = temp_dir.path().to_path_buf();
        // canonicalize /var -> /private/var
        let watched_root = watched_root.canonicalize().expect("failed to canonicalize");
        let watch_path = watched_root
            .to_str()
            .expect("tempdir path should be utf8")
            .to_string();

        let (_, initial_watcher) =
            EventWatcher::spawn(watch_path.clone(), current_event_id(), 0.05);
        drop(initial_watcher);

        // Give the background thread a moment to observe the drop.
        std::thread::sleep(Duration::from_millis(500));

        let (_, respawned_watcher) = EventWatcher::spawn(watch_path, current_event_id(), 0.05);

        // Allow the stream to start before triggering filesystem activity.
        std::thread::sleep(Duration::from_millis(500));

        let created_file = watched_root.join("respawn_event.txt");
        std::fs::write(&created_file, "cardinal").expect("failed to write test file");

        let deadline = Instant::now() + Duration::from_secs(5);
        let mut observed_change = false;
        while Instant::now() < deadline {
            match respawned_watcher.recv_timeout(Duration::from_millis(200)) {
                Ok(batch) => {
                    if batch
                        .iter()
                        .any(|event| event.path.starts_with(&created_file))
                    {
                        observed_change = true;
                        break;
                    }
                }
                Err(RecvTimeoutError::Timeout) => continue,
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }

        drop(respawned_watcher);
        assert!(
            observed_change,
            "respawned watcher failed to deliver file change event"
        );
    }
}
