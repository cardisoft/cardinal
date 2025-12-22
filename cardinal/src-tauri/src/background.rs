use crate::{
    commands::{NodeInfoRequest, SearchJob},
    lifecycle::{AppLifecycleState, load_app_state, update_app_state},
    search_activity,
    window_controls::is_main_window_foreground,
};
use anyhow::Result;
use base64::{Engine as _, engine::general_purpose};
use cardinal_sdk::{EventFlag, EventWatcher};
use crossbeam_channel::{Receiver, Sender};
use rayon::spawn;
use search_cache::{
    HandleFSEError, SearchCache, SearchOptions, SearchOutcome, SearchResultNode, SlabIndex,
};
use serde::Serialize;
use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Emitter};
use tracing::{error, info};

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StatusBarUpdate {
    pub scanned_files: usize,
    pub processed_events: usize,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct IconPayload {
    pub slab_index: SlabIndex,
    pub icon: String,
}

pub struct BackgroundLoopChannels {
    pub finish_rx: Receiver<Sender<Option<SearchCache>>>,
    pub update_window_state_rx: Receiver<()>,
    pub search_rx: Receiver<SearchJob>,
    pub result_tx: Sender<Result<SearchOutcome>>,
    pub node_info_rx: Receiver<NodeInfoRequest>,
    pub icon_viewport_rx: Receiver<(u64, Vec<SlabIndex>)>,
    pub rescan_rx: Receiver<()>,
    pub icon_update_tx: Sender<IconPayload>,
}

pub fn emit_status_bar_update(
    app_handle: &AppHandle,
    scanned_files: usize,
    processed_events: usize,
) {
    app_handle
        .emit(
            "status_bar_update",
            StatusBarUpdate {
                scanned_files,
                processed_events,
            },
        )
        .unwrap();
}

struct EventSnapshot {
    path: PathBuf,
    event_id: u64,
    flag: EventFlag,
    timestamp: i64,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct RecentEvent {
    path: String,
    flag_bits: u32,
    event_id: u64,
    timestamp: i64,
}

pub fn run_background_event_loop(
    app_handle: &AppHandle,
    mut cache: SearchCache,
    mut event_watcher: EventWatcher,
    channels: BackgroundLoopChannels,
    watch_root: &str,
    fse_latency_secs: f64,
    db_path: PathBuf,
) {
    let BackgroundLoopChannels {
        finish_rx,
        update_window_state_rx,
        search_rx,
        result_tx,
        node_info_rx,
        icon_viewport_rx,
        rescan_rx,
        icon_update_tx,
    } = channels;
    let mut processed_events = 0usize;
    let mut history_ready = load_app_state() == AppLifecycleState::Ready;

    let mut window_is_foreground = true;
    let mut hide_flush_remaining_ticks: u8 = 0;
    // Hide flush is polled on a 10s ticker; idle flush shares the same tick.
    let flush_ticker = crossbeam_channel::tick(Duration::from_secs(10));

    loop {
        crossbeam_channel::select! {
            recv(finish_rx) -> tx => {
                let tx = tx.expect("Finish channel closed");
                tx.send(Some(cache)).expect("Failed to send cache");
                return;
            }
            recv(update_window_state_rx) -> _ => {
                // Recompute foreground state on demand instead of mirroring events.
                let new_foreground = is_main_window_foreground(app_handle);
                if window_is_foreground && !new_foreground {
                    hide_flush_remaining_ticks = 2; // allow 10~20s before running hide flush
                } else if new_foreground {
                    hide_flush_remaining_ticks = 0;
                }
                window_is_foreground = new_foreground;
            }
            recv(flush_ticker) -> _ => {
                if load_app_state() != AppLifecycleState::Ready {
                    continue;
                }
                let mut flush_search_cache = FlushSearchCache {
                    cache: &mut cache,
                    db_path: &db_path,
                };
                let flushed = start_flush_checks(
                    || is_main_window_foreground(app_handle),
                    search_activity::search_idles,
                    &mut flush_search_cache,
                    &mut hide_flush_remaining_ticks,
                );
                if flushed {
                    search_activity::note_search_activity();
                }
            }
            recv(search_rx) -> job => {
                let SearchJob {
                    query,
                    options,
                    cancellation_token,
                } = job.expect("Search channel closed");
                let opts = SearchOptions::from(options);
                let payload = cache.search_with_options(&query, opts, cancellation_token);
                result_tx.send(payload).expect("Failed to send result");
            }
            recv(node_info_rx) -> request => {
                let request = request.expect("Node info channel closed");
                let NodeInfoRequest {
                    slab_indices,
                    response_tx,
                } = request;
                let node_info_results = cache.expand_file_nodes(&slab_indices);
                let _ = response_tx.send(node_info_results);
            }
            recv(icon_viewport_rx) -> update => {
                let (_request_id, viewport) = update.expect("Icon viewport channel closed");

                let nodes = cache.expand_file_nodes(&viewport);
                let icon_jobs: Vec<_> = viewport
                    .into_iter()
                    .zip(nodes.into_iter())
                    .map(|(slab_index, SearchResultNode { path, .. })| (slab_index, path))
                    .collect();

                if icon_jobs.is_empty() {
                    continue;
                }

                icon_jobs
                    .into_iter()
                    .map(|(slab_index, path)| (slab_index, path.to_string_lossy().into_owned()))
                    .filter(|(_, path)| !path.contains("OneDrive") && !path.contains("com~apple~CloudDocs"))
                    .for_each(|(slab_index, path)| {
                        let icon_update_tx = icon_update_tx.clone();
                        spawn(move || {
                            if let Some(icon) = fs_icon::icon_of_path(&path, 512.0).map(|data| format!(
                                "data:image/png;base64,{}",
                                general_purpose::STANDARD.encode(&data)
                            )) {
                                let _ = icon_update_tx.send(IconPayload { slab_index, icon });
                            }
                        });
                    });
            }
            recv(rescan_rx) -> request => {
                request.expect("Rescan channel closed");
                info!("Manual rescan requested");
                perform_rescan(
                    app_handle,
                    &mut cache,
                    &mut event_watcher,
                    watch_root,
                    fse_latency_secs,
                    &mut history_ready,
                    &mut processed_events
                );
            }
            recv(event_watcher) -> events => {
                let events = events.expect("Event stream closed");
                processed_events += events.len();

                emit_status_bar_update(app_handle, cache.get_total_files(), processed_events);

                let mut snapshots = Vec::with_capacity(events.len());
                for event in events.iter() {
                    if event.flag == EventFlag::HistoryDone {
                        history_ready = true;
                        update_app_state(app_handle, AppLifecycleState::Ready);
                    } else if history_ready {
                        snapshots.push(EventSnapshot {
                            path: event.path.clone(),
                            event_id: event.id,
                            flag: event.flag,
                            timestamp: unix_timestamp_now(),
                        });
                    }
                }

                let handle_result = cache.handle_fs_events(events);
                if let Err(HandleFSEError::Rescan) = handle_result {
                    info!("!!!!!!!!!! Rescan triggered !!!!!!!!");
                    perform_rescan(
                        app_handle,
                        &mut cache,
                        &mut event_watcher,
                        watch_root,
                        fse_latency_secs,
                        &mut history_ready,
                        &mut processed_events
                    );
                }

                if history_ready && !snapshots.is_empty() {
                    forward_new_events(app_handle, &snapshots);
                }
            }
        }
    }
}

fn perform_rescan(
    app_handle: &AppHandle,
    cache: &mut SearchCache,
    event_watcher: &mut EventWatcher,
    watch_root: &str,
    fse_latency_secs: f64,
    history_ready: &mut bool,
    processed_events: &mut usize,
) {
    *event_watcher = EventWatcher::noop();
    update_app_state(app_handle, AppLifecycleState::Initializing);
    emit_status_bar_update(app_handle, 0, 0);
    *history_ready = false;
    *processed_events = 0;

    let walk_data = cache.walk_data();
    let walking_done = AtomicBool::new(false);
    let stopped = std::thread::scope(|s| {
        s.spawn(|| {
            while !walking_done.load(Ordering::Relaxed) {
                let dirs = walk_data.num_dirs.load(Ordering::Relaxed);
                let files = walk_data.num_files.load(Ordering::Relaxed);
                let total = dirs + files;
                emit_status_bar_update(app_handle, total, 0);
                std::thread::sleep(Duration::from_millis(100));
            }
        });
        // If rescan is cancelled, we have nothing to do
        let stopped = cache.rescan_with_walk_data(&walk_data).is_none();
        walking_done.store(true, Ordering::Relaxed);
        stopped
    });

    *event_watcher = if stopped {
        EventWatcher::noop()
    } else {
        EventWatcher::spawn(
            watch_root.to_string(),
            cache.last_event_id(),
            fse_latency_secs,
        )
        .1
    };
    update_app_state(app_handle, AppLifecycleState::Updating);
}

fn unix_timestamp_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn forward_new_events(app_handle: &AppHandle, snapshots: &[EventSnapshot]) {
    if snapshots.is_empty() {
        return;
    }

    let mut ordered_events: Vec<&EventSnapshot> = snapshots.iter().collect();
    ordered_events.sort_unstable_by(|a, b| {
        a.timestamp
            .cmp(&b.timestamp)
            .then_with(|| a.event_id.cmp(&b.event_id))
    });
    let new_events: Vec<RecentEvent> = ordered_events
        .into_iter()
        .map(|event| RecentEvent {
            path: event.path.to_string_lossy().into_owned(),
            flag_bits: event.flag.bits(),
            event_id: event.event_id,
            timestamp: event.timestamp,
        })
        .collect();

    let _ = app_handle.emit("fs_events_batch", new_events);
}

struct FlushSearchCache<'cache> {
    cache: &'cache mut SearchCache,
    db_path: &'cache Path,
}

trait FlushSnapshot {
    fn flush_snapshot_to_file(&mut self) -> Result<()>;
    fn db_path(&self) -> &Path;
}

impl FlushSnapshot for FlushSearchCache<'_> {
    fn flush_snapshot_to_file(&mut self) -> Result<()> {
        SearchCache::flush_snapshot_to_file(self.cache, self.db_path)
    }
    fn db_path(&self) -> &Path {
        self.db_path
    }
}

/// This function should be called periodically to check if a flush is needed.
/// Returns true if a flush was performed (either hide or idle).
fn start_flush_checks<F, I, C>(
    is_foreground: F,
    is_idle: I,
    cache: &mut C,
    hide_flush_remaining_ticks: &mut u8,
) -> bool
where
    F: Fn() -> bool,
    I: Fn() -> bool,
    C: FlushSnapshot,
{
    let idle_flush = is_idle();
    let hide_flush = {
        // Consume the pending hide flush counter; only fire once.
        if *hide_flush_remaining_ticks > 0 {
            *hide_flush_remaining_ticks -= 1;
            *hide_flush_remaining_ticks == 0
        } else {
            false
        }
    };
    let hide_flush = hide_flush && !is_foreground();

    if hide_flush {
        let label = "hide_flush";
        match cache.flush_snapshot_to_file() {
            Ok(()) => info!(
                "Cache flushed successfully ({label}) to {:?}",
                cache.db_path()
            ),
            Err(e) => error!(
                "Cache flush failed ({label}) to {:?}: {e:?}",
                cache.db_path()
            ),
        }
        true
    } else if idle_flush {
        let label = "idle_flush";
        match cache.flush_snapshot_to_file() {
            Ok(()) => info!(
                "Cache flushed successfully ({label}) to {:?}",
                cache.db_path()
            ),
            Err(e) => error!(
                "Cache flush failed ({label}) to {:?}: {e:?}",
                cache.db_path()
            ),
        }
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct FakeCache {
        pub flushes: usize,
    }

    impl FlushSnapshot for FakeCache {
        fn flush_snapshot_to_file(&mut self) -> Result<()> {
            self.flushes += 1;
            Ok(())
        }

        fn db_path(&self) -> &Path {
            Path::new("db")
        }
    }

    #[test]
    fn hide_flush_resets_idle_window() {
        let mut cache = FakeCache::default();
        let mut pending = 1;
        let flushed = start_flush_checks(|| false, || true, &mut cache, &mut pending);

        assert_eq!(cache.flushes, 1, "hide flush should run once");
        assert_eq!(pending, 0, "pending hide flush should be consumed");
        assert!(
            flushed,
            "hide flush should satisfy idle window to avoid immediate idle flush"
        );
    }

    #[test]
    fn idle_flush_runs_when_due() {
        let mut cache = FakeCache::default();
        let mut pending = 0;
        let flushed = start_flush_checks(|| true, || true, &mut cache, &mut pending);

        assert_eq!(cache.flushes, 1, "idle flush should run once when due");
        assert!(flushed, "idle flush should advance idle window");
    }

    #[test]
    fn pending_consumed_but_no_hide_flush_if_foreground() {
        let mut cache = FakeCache::default();
        let mut pending = 1;
        // Foreground -> pending should be consumed but no hide flush should run
        let flushed = start_flush_checks(|| true, || false, &mut cache, &mut pending);
        assert!(!flushed, "no idle flush satisfied in foreground");

        assert_eq!(
            cache.flushes, 0,
            "no flush should run while window is foreground"
        );
        assert_eq!(
            pending, 0,
            "pending counter should be consumed even if foreground"
        );
    }

    #[test]
    fn hide_flush_requires_two_ticks() {
        // make sure idle flush is not accidentally due from other tests

        let mut cache = FakeCache::default();
        let mut pending = 2;

        // First tick consumes one counter, no flush yet
        let flushed = start_flush_checks(|| false, || false, &mut cache, &mut pending);
        assert!(!flushed, "no idle flush satisfied on first tick");
        assert_eq!(cache.flushes, 0, "no flush on first tick");
        assert_eq!(pending, 1, "pending decremented to 1");

        // Second tick should trigger the hide flush
        let flushed = start_flush_checks(|| false, || false, &mut cache, &mut pending);
        assert!(flushed, "hide flush was performed");
        assert_eq!(cache.flushes, 1, "flush should run on second tick");
        assert_eq!(pending, 0, "pending should be consumed after flush");
    }

    #[test]
    fn hide_preempts_idle_and_only_one_flush_when_both_pending() {
        let mut cache = FakeCache::default();
        let mut pending = 1;

        // When both an idle flush is due and a hide flush fires, hide flush
        // should run and it should satisfy the idle window so we don't double-flush.
        let flushed = start_flush_checks(|| false, || true, &mut cache, &mut pending);

        assert_eq!(cache.flushes, 1, "only one flush should run");
        assert_eq!(pending, 0, "pending hide flush should be consumed");
        assert!(flushed, "hide flush should satisfy idle window");
    }

    use anyhow::anyhow;

    struct FakeCacheErr;

    impl FlushSnapshot for FakeCacheErr {
        fn flush_snapshot_to_file(&mut self) -> Result<()> {
            Err(anyhow!("flush failed"))
        }

        fn db_path(&self) -> &Path {
            Path::new("db")
        }
    }

    #[test]
    fn flush_error_does_not_reset_idle() {
        let mut cache = FakeCacheErr;
        let mut pending = 1;

        // Hide flush attempt fails; the hide flush logic treats the flush as
        // satisfying the idle window (it bumps the idle timestamp even on errors).
        let flushed = start_flush_checks(|| false, || true, &mut cache, &mut pending);

        // pending should be consumed (counter is decremented), but since flush failed,
        // idle window should still be considered idle and search_idles() should be true.
        assert!(
            flushed,
            "failed hide flush should still satisfy idle window (bumped)"
        );
        assert_eq!(
            pending, 0,
            "pending should be consumed even on flush failure"
        );
    }

    // ===== Tests for foreground -> background transitions =====

    #[test]
    fn foreground_to_background_sets_pending_to_2() {
        // This test verifies the initial state when window goes to background
        // In the real event loop, `hide_flush_remaining_ticks` is set to 2
        // Note: if idle is already due, idle flush will trigger regardless of pending

        let mut cache = FakeCache::default();
        let mut pending = 2; // simulates what happens when window loses focus

        // First tick after going to background: pending=2, decrements to 1, no hide flush yet
        // idle not due, so no flush at all
        let flushed = start_flush_checks(|| false, || false, &mut cache, &mut pending);
        assert!(!flushed, "no idle flush on first tick");
        assert_eq!(
            cache.flushes, 0,
            "no flush on first tick after backgrounding (idle not due)"
        );
        assert_eq!(pending, 1, "pending should decrement from 2 to 1");
    }

    #[test]
    fn foreground_to_background_idle_not_due_completes_after_two_ticks() {
        // Window goes to background, idle is not due, should flush after 2 ticks (10-20s)

        let mut cache = FakeCache::default();
        let mut pending = 2;

        // Tick 1: decrement but no flush
        let flushed = start_flush_checks(|| false, || false, &mut cache, &mut pending);
        assert!(!flushed, "no idle flush on tick 1");
        assert_eq!(cache.flushes, 0);
        assert_eq!(pending, 1);

        // Tick 2: should trigger hide flush
        let flushed = start_flush_checks(|| false, || false, &mut cache, &mut pending);
        assert!(flushed, "hide flush was performed");
        assert_eq!(cache.flushes, 1, "flush should trigger on second tick");
        assert_eq!(pending, 0);
    }

    #[test]
    fn foreground_to_background_while_idle_due_flushes_on_second_tick() {
        // Window goes to background AND idle is due: idle_flush will run immediately
        // because the else-if checks idle regardless of hide_flush countdown
        let mut cache = FakeCache::default();
        let mut pending = 2;

        // Make sure idle is NOT due initially

        // Tick 1: pending=2, decrements to 1, idle not due, no flush
        let flushed = start_flush_checks(|| false, || false, &mut cache, &mut pending);
        assert!(!flushed, "no idle flush when idle not due");
        assert_eq!(cache.flushes, 0, "no flush when idle not due");
        assert_eq!(pending, 1, "pending still decrements");

        // Now make idle due

        // Tick 2: pending=1, becomes 0, hide_flush fires (takes priority over idle)
        let flushed = start_flush_checks(|| false, || false, &mut cache, &mut pending);
        assert_eq!(cache.flushes, 1, "hide flush fires on second tick");
        assert_eq!(pending, 0);
        assert!(flushed, "hide flush satisfies idle");
    }

    #[test]
    fn background_to_foreground_resets_pending_to_zero() {
        // Window is in background (pending > 0), then comes back to foreground
        // Make sure idle is NOT due to avoid interference

        let mut cache = FakeCache::default();
        let mut pending = 1; // simulates partial countdown

        // User brings window back to foreground before flush triggers
        let flushed = start_flush_checks(|| true, || false, &mut cache, &mut pending);
        assert!(!flushed, "no idle flush when returning to foreground");
        assert_eq!(
            cache.flushes, 0,
            "no flush when returning to foreground (idle not due)"
        );
        assert_eq!(pending, 0, "pending should be reset to 0");
    }

    #[test]
    fn multiple_background_entries_only_first_sets_pending() {
        // If window goes background -> foreground -> background quickly,
        // the second background entry should reset pending to 2 again.
        // This test simulates the second background entry.

        let mut cache = FakeCache::default();

        // First background entry
        let mut pending = 2;
        let flushed = start_flush_checks(|| false, || false, &mut cache, &mut pending);
        assert!(!flushed, "no idle flush on first background");
        assert_eq!(pending, 1);

        // User returns to foreground briefly
        let flushed = start_flush_checks(|| true, || false, &mut cache, &mut pending);
        assert!(!flushed, "no idle flush in foreground");
        assert_eq!(pending, 0);

        // Goes to background again - in real code, this sets pending=2 again
        pending = 2;
        let flushed = start_flush_checks(|| false, || false, &mut cache, &mut pending);
        assert!(!flushed, "no idle flush on countdown restart");
        assert_eq!(cache.flushes, 0, "countdown restarts");
        assert_eq!(pending, 1);

        let flushed = start_flush_checks(|| false, || false, &mut cache, &mut pending);
        assert!(flushed, "hide flush was performed");
        assert_eq!(cache.flushes, 1, "flush after second countdown");
        assert_eq!(pending, 0);
    }

    #[test]
    fn idle_flush_while_in_background_with_no_pending() {
        // Window is in background, pending already consumed (or was 0),
        // but idle becomes due -> idle flush should run

        let mut cache = FakeCache::default();
        let mut pending = 0;

        let flushed = start_flush_checks(|| false, || true, &mut cache, &mut pending);
        assert!(flushed, "idle flush should be satisfied");
        assert_eq!(cache.flushes, 1, "idle flush should run even in background");
        assert_eq!(pending, 0);
    }

    #[test]
    fn no_flush_when_not_idle_and_no_pending() {
        // Window is anywhere, no pending, idle not due -> no flush

        let mut cache = FakeCache::default();
        let mut pending = 0;

        let flushed = start_flush_checks(|| false, || false, &mut cache, &mut pending);
        assert!(!flushed, "no idle flush when conditions not met");
        assert_eq!(cache.flushes, 0, "no flush when neither condition is met");

        let flushed = start_flush_checks(|| true, || false, &mut cache, &mut pending);
        assert!(!flushed, "no idle flush in foreground");
        assert_eq!(cache.flushes, 0, "still no flush in foreground");
    }

    #[test]
    fn pending_equals_1_triggers_immediately_in_background() {
        // If pending is already 1 (second tick countdown), flush should trigger

        let mut cache = FakeCache::default();
        let mut pending = 1;

        let flushed = start_flush_checks(|| false, || false, &mut cache, &mut pending);
        assert!(flushed, "hide flush was performed");
        assert_eq!(cache.flushes, 1, "pending=1 in background should flush");
        assert_eq!(pending, 0);
    }

    #[test]
    fn pending_greater_than_2_decrements_correctly() {
        // Edge case: if pending is set to 3+ (shouldn't happen normally),
        // it should decrement each tick until reaching 0, then flush

        let mut cache = FakeCache::default();
        let mut pending = 3;

        let flushed = start_flush_checks(|| false, || false, &mut cache, &mut pending);
        assert!(!flushed, "no idle flush during countdown");
        assert_eq!(cache.flushes, 0);
        assert_eq!(pending, 2);

        let flushed = start_flush_checks(|| false, || false, &mut cache, &mut pending);
        assert!(!flushed, "no idle flush during countdown");
        assert_eq!(cache.flushes, 0);
        assert_eq!(pending, 1);

        let flushed = start_flush_checks(|| false, || false, &mut cache, &mut pending);
        assert!(flushed, "hide flush was performed");
        assert_eq!(cache.flushes, 1, "flush triggers when countdown reaches 0");
        assert_eq!(pending, 0);
    }

    #[test]
    fn rapid_foreground_background_transitions() {
        // Test rapid transitions: background -> foreground -> background

        let mut cache = FakeCache::default();

        // Go to background
        let mut pending = 2;
        let flushed = start_flush_checks(|| false, || false, &mut cache, &mut pending);
        assert!(!flushed, "no idle flush on background transition");
        assert_eq!(pending, 1);

        // Immediately back to foreground
        let flushed = start_flush_checks(|| true, || false, &mut cache, &mut pending);
        assert!(!flushed, "no idle flush when returning to foreground");
        assert_eq!(pending, 0);
        assert_eq!(cache.flushes, 0);

        // Back to background again
        pending = 2;
        let flushed = start_flush_checks(|| false, || false, &mut cache, &mut pending);
        assert!(!flushed, "no idle flush on re-background");
        assert_eq!(pending, 1);

        // Back to foreground before flush
        let flushed = start_flush_checks(|| true, || false, &mut cache, &mut pending);
        assert!(!flushed, "no idle flush on cancellation");
        assert_eq!(pending, 0);
        assert_eq!(cache.flushes, 0, "should never flush due to cancellations");
    }

    #[test]
    fn idle_triggers_during_hide_countdown() {
        // Window goes to background (pending=2), but before the countdown completes,
        // idle becomes due. The hide flush (on second tick) should preempt idle.
        let mut cache = FakeCache::default();
        let mut pending = 2;

        // First tick: pending=2->1, idle not quite due yet
        let flushed = start_flush_checks(|| false, || false, &mut cache, &mut pending);
        assert!(!flushed, "idle not due yet during countdown");
        assert_eq!(cache.flushes, 0);
        assert_eq!(pending, 1);

        // Second tick: pending=1->0, both background and idle are ready,
        // background should preempt
        let flushed = start_flush_checks(|| false, || true, &mut cache, &mut pending);
        assert_eq!(cache.flushes, 1, "single flush (background preempts idle)");
        assert_eq!(pending, 0);
        assert!(flushed, "idle satisfied by hide flush");
    }

    #[test]
    fn consecutive_hide_flushes_require_new_pending() {
        // After a hide flush completes, pending is 0.
        // Idle flush can trigger independently if conditions are met.
        let mut cache = FakeCache::default();
        let mut pending = 1;

        // Set up so we won't hit idle initially

        // First hide flush
        let flushed = start_flush_checks(|| false, || false, &mut cache, &mut pending);
        assert!(flushed, "hide flush was performed");
        assert_eq!(cache.flushes, 1);
        assert_eq!(pending, 0);

        // Ticks continue, but pending is 0 and idle not due -> no more flushes
        let flushed = start_flush_checks(|| false, || false, &mut cache, &mut pending);
        assert!(!flushed, "no idle flush without new trigger");
        assert_eq!(
            cache.flushes, 1,
            "no second flush without new pending or idle"
        );

        let flushed = start_flush_checks(|| false, || false, &mut cache, &mut pending);
        assert!(!flushed, "still no idle flush");
        assert_eq!(cache.flushes, 1, "still no flush");

        // Simulate window coming back and going to background again
        pending = 2;
        let flushed = start_flush_checks(|| false, || false, &mut cache, &mut pending);
        assert!(!flushed, "no idle flush during countdown");
        assert_eq!(pending, 1);
        assert_eq!(cache.flushes, 1, "no flush yet, still counting down");

        let flushed = start_flush_checks(|| false, || false, &mut cache, &mut pending);
        assert!(flushed, "second hide flush was performed");
        assert_eq!(cache.flushes, 2, "second hide flush after re-backgrounding");
        assert_eq!(pending, 0);
    }

    #[test]
    fn error_during_hide_flush_still_consumes_pending() {
        // Hide flush fails, but pending should still be consumed

        let mut cache = FakeCacheErr;
        let mut pending = 1;

        let flushed = start_flush_checks(|| false, || false, &mut cache, &mut pending);
        assert!(flushed, "hide flush was attempted");
        assert_eq!(pending, 0, "pending consumed even on flush error");

        // Should not re-attempt flush on next tick unless pending is set again
        let flushed = start_flush_checks(|| false, || false, &mut cache, &mut pending);
        assert!(!flushed, "no idle flush on retry");
        assert_eq!(pending, 0, "stays at 0");
    }

    #[test]
    fn idle_flush_in_foreground_when_due() {
        // Foreground window, idle is due -> idle flush should run

        let mut cache = FakeCache::default();
        let mut pending = 0;

        let flushed = start_flush_checks(|| true, || true, &mut cache, &mut pending);
        assert_eq!(cache.flushes, 1, "idle flush runs in foreground");
        assert!(flushed, "idle window advanced");
    }

    #[test]
    fn no_idle_flush_in_foreground_when_not_due() {
        // Foreground window, idle not due -> no flush

        let mut cache = FakeCache::default();
        let mut pending = 0;

        let flushed = start_flush_checks(|| true, || false, &mut cache, &mut pending);
        assert!(!flushed, "no idle flush when not due");
        assert_eq!(cache.flushes, 0, "no flush when idle not due");
    }

    #[test]
    fn hide_flush_after_exact_two_ticks_at_ten_seconds_each() {
        // Verifies the exact 10-20 second window: pending=2 means first tick at ~10s, second at ~20s

        let mut cache = FakeCache::default();
        let mut pending = 2;

        // Tick at ~10s
        let flushed = start_flush_checks(|| false, || false, &mut cache, &mut pending);
        assert!(!flushed, "no idle flush at ~10s mark");
        assert_eq!(cache.flushes, 0, "no flush at ~10s mark");
        assert_eq!(pending, 1);

        // Tick at ~20s
        let flushed = start_flush_checks(|| false, || false, &mut cache, &mut pending);
        assert!(flushed, "hide flush was performed at ~20s");
        assert_eq!(cache.flushes, 1, "flush at ~20s mark");
        assert_eq!(pending, 0);
    }

    #[test]
    fn idle_flush_error_still_advances_idle_window() {
        // Idle flush fails, but idle window should still be advanced

        let mut cache = FakeCacheErr;
        let mut pending = 0;

        let flushed = start_flush_checks(|| true, || true, &mut cache, &mut pending);

        // Even though flush failed, idle window should be advanced
        assert!(flushed, "idle window should be advanced even on error");
    }

    #[test]
    fn idle_at_exact_5_minute_boundary() {
        let mut cache = FakeCache::default();
        let mut pending = 0;

        // At exactly 5 minutes, should trigger
        let flushed = start_flush_checks(|| true, || true, &mut cache, &mut pending);
        assert!(flushed, "idle flush should be satisfied at exact boundary");
        assert_eq!(cache.flushes, 1, "flush should trigger at exact boundary");
    }

    #[test]
    fn idle_just_under_5_minute_boundary() {
        // Test behavior when idle is 1 second under threshold

        let mut cache = FakeCache::default();
        let mut pending = 0;

        let flushed = start_flush_checks(|| true, || false, &mut cache, &mut pending);
        assert!(!flushed, "no idle flush when under threshold");
        assert_eq!(cache.flushes, 0, "no flush when under threshold");
    }

    #[test]
    fn background_to_foreground_with_idle_due() {
        // Window returns to foreground while idle is due and pending>0
        // Pending gets consumed, and idle flush triggers

        let mut cache = FakeCache::default();
        let mut pending = 1; // was in background countdown

        // Returns to foreground with idle due
        // Since we're in foreground, hide_flush won't fire (even if pending becomes 0)
        // But idle_flush will fire because it's due
        let flushed = start_flush_checks(|| true, || true, &mut cache, &mut pending);
        assert!(flushed, "idle flush should be satisfied");
        assert_eq!(cache.flushes, 1, "idle flush should trigger in foreground");
        assert_eq!(pending, 0, "pending should be consumed");
    }

    #[test]
    fn pending_u8_max_decrements_correctly() {
        // Edge case: pending at u8::MAX should decrement without overflow

        let mut cache = FakeCache::default();
        let mut pending = u8::MAX;

        let flushed = start_flush_checks(|| false, || false, &mut cache, &mut pending);
        assert!(!flushed, "no idle flush during MAX countdown");
        assert_eq!(pending, u8::MAX - 1, "should decrement from MAX");
        assert_eq!(cache.flushes, 0, "no flush at MAX");

        // Continue decrementing
        for _ in 0..(u8::MAX - 2) {
            let _flushed = start_flush_checks(|| false, || false, &mut cache, &mut pending);
        }
        assert_eq!(pending, 1, "should reach 1");

        // Final tick triggers flush
        let flushed = start_flush_checks(|| false, || false, &mut cache, &mut pending);
        assert!(flushed, "hide flush was performed");
        assert_eq!(cache.flushes, 1, "flush when reaching 0");
        assert_eq!(pending, 0);
    }

    #[test]
    fn consecutive_idle_flushes() {
        // Multiple idle flushes should each reset the idle window
        // Note: Each set_idle_over_5m call sets the timestamp independently
        let mut cache = FakeCache::default();
        let mut pending = 0;

        // First idle flush

        let flushed = start_flush_checks(|| true, || true, &mut cache, &mut pending);
        assert!(flushed, "first idle flush should be satisfied");
        assert_eq!(cache.flushes, 1, "first idle flush");

        // Second idle flush - set idle again

        let flushed = start_flush_checks(|| true, || true, &mut cache, &mut pending);
        assert!(flushed, "second idle flush should be satisfied");
        assert_eq!(cache.flushes, 2, "second idle flush");

        // Third idle flush

        let flushed = start_flush_checks(|| true, || true, &mut cache, &mut pending);
        assert!(flushed, "third idle flush should be satisfied");
        assert_eq!(cache.flushes, 3, "third idle flush");
    }

    #[test]
    fn hide_flush_pending_1_with_idle_also_due() {
        // Both hide (pending=1) and idle are ready, hide should win

        let mut cache = FakeCache::default();
        let mut pending = 1;

        let flushed = start_flush_checks(|| false, || true, &mut cache, &mut pending);
        assert!(flushed, "hide flush satisfies idle when both ready");
        assert_eq!(cache.flushes, 1, "single flush (hide wins)");
        assert_eq!(pending, 0);
        assert!(flushed, "idle satisfied by hide flush");
    }

    #[test]
    fn idle_flush_error_then_success() {
        // Idle flush fails, then succeeds on retry

        let mut cache_err = FakeCacheErr;
        let mut pending = 0;

        let flushed = start_flush_checks(|| true, || true, &mut cache_err, &mut pending);
        assert!(flushed, "idle advanced even on error");

        // Time passes again

        let mut cache_ok = FakeCache::default();
        let flushed = start_flush_checks(|| true, || true, &mut cache_ok, &mut pending);
        assert!(flushed, "idle flush satisfied on success");
        assert_eq!(cache_ok.flushes, 1, "should succeed on retry");
    }

    #[test]
    fn pending_zero_stays_zero() {
        // Calling with pending=0 should keep it at 0

        let mut cache = FakeCache::default();
        let mut pending = 0;

        for _ in 0..10 {
            let _flushed = start_flush_checks(|| false, || false, &mut cache, &mut pending);
            assert_eq!(pending, 0, "pending should stay at 0");
        }
        assert_eq!(
            cache.flushes, 0,
            "no flushes when pending=0 and idle not due"
        );
    }

    #[test]
    fn foreground_with_pending_and_idle_both_not_ready() {
        // Foreground, pending=5, idle not due -> pending decrements, no flush

        let mut cache = FakeCache::default();
        let mut pending = 5;

        let flushed = start_flush_checks(|| true, || false, &mut cache, &mut pending);
        assert!(!flushed, "no idle flush in foreground");
        assert_eq!(pending, 4, "pending decrements in foreground");
        assert_eq!(cache.flushes, 0, "no flush");

        let flushed = start_flush_checks(|| true, || false, &mut cache, &mut pending);
        assert!(!flushed, "no idle flush in foreground");
        assert_eq!(pending, 3);
        assert_eq!(cache.flushes, 0);
    }

    #[test]
    fn hide_flush_success_then_immediate_idle_due() {
        // Hide flush completes, then immediately idle becomes due (edge case)
        let mut cache = FakeCache::default();
        let mut pending = 1;

        // Hide flush triggers
        let flushed = start_flush_checks(|| false, || true, &mut cache, &mut pending);
        assert!(flushed, "hide flush satisfies idle");
        assert_eq!(cache.flushes, 1);
        assert_eq!(pending, 0);

        // Now idle becomes due

        let flushed = start_flush_checks(|| false, || true, &mut cache, &mut pending);
        assert!(flushed, "idle flush should be satisfied after hide");
        assert_eq!(cache.flushes, 2, "idle flush should trigger after hide");
    }

    #[test]
    fn alternating_hide_and_idle_flushes() {
        // Alternate between hide and idle flushes
        let mut cache = FakeCache::default();

        // Hide flush (make sure idle is not due)

        let mut pending = 1;
        let flushed = start_flush_checks(|| false, || true, &mut cache, &mut pending);
        assert!(flushed, "hide flush satisfies idle");
        assert_eq!(cache.flushes, 1, "first hide flush");
        assert_eq!(pending, 0);

        // Idle flush (set idle to be due)
        pending = 0;

        let flushed = start_flush_checks(|| true, || true, &mut cache, &mut pending);
        assert!(flushed, "idle flush should be satisfied");
        assert_eq!(cache.flushes, 2, "first idle flush");

        // Hide flush again (reset idle to not due)

        pending = 1;
        let flushed = start_flush_checks(|| false, || true, &mut cache, &mut pending);
        assert!(flushed, "hide flush satisfies idle");
        assert_eq!(cache.flushes, 3, "second hide flush");
        assert_eq!(pending, 0);

        // Idle again (set idle to be due)
        pending = 0;

        let flushed = start_flush_checks(|| false, || true, &mut cache, &mut pending);
        assert!(flushed, "idle flush should be satisfied");
        assert_eq!(cache.flushes, 4, "second idle flush");
    }
}
