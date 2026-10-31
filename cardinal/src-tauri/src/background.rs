use crate::{
    commands::{
        NodeInfoRequest, RecentNodesRequest, SearchJob, SearchJobResult, WatchConfigUpdate,
    },
    lifecycle::{APP_QUIT, AppLifecycleState, load_app_state, update_app_state},
    search_activity,
    window_controls::is_main_window_foreground,
};
use anyhow::Result;
use base64::{Engine as _, engine::general_purpose};
use cardinal_sdk::{EventFlag, EventWatcher, FsEvent};
use crossbeam_channel::{Receiver, Sender};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use rayon::spawn;
use search_cache::{
    HandleFSEError, SearchCache, SearchOptions, SearchResultNode, SlabIndex, WalkData,
};
use search_cancel::CancellationToken;
use serde::Serialize;
use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Emitter};
use tracing::{error, info, warn};

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StatusBarUpdate {
    pub scanned_files: usize,
    pub processed_events: usize,
    pub rescan_errors: usize,
}

/// Icon request for the rows currently on screen. `icon_size` is the raster edge length in
/// points requested by the frontend preset (32 for the list, up to 384 for tiled previews).
#[derive(Debug, Clone)]
pub struct IconViewportUpdate {
    /// Echoed request id; kept for parity with the command signature and future deduplication.
    #[allow(dead_code)]
    pub id: u64,
    pub viewport: Vec<SlabIndex>,
    pub icon_size: f64,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct IconPayload {
    pub slab_index: SlabIndex,
    pub icon: String,
}

/// Result of the asynchronous initial index load/build.
pub struct IndexBuildResult {
    /// Token of the build that produced this result. A result is only installed when it
    /// still matches the active build, so a cancelled/superseded walk cannot replace a
    /// newer cache.
    pub cancellation_token: CancellationToken,
    pub cache: Option<SearchCache>,
}

/// Result of a temporary folder-scope walk.
pub enum ScopedIndexResult {
    Built {
        scope: PathBuf,
        cache: Box<SearchCache>,
    },
    /// The scope input was not an existing directory (or the walk was cancelled).
    Unusable { scope: PathBuf },
}

/// State of the temporary per-scope index used while a folder is not in the main index.
///
/// Slab indices are only meaningful for the cache that produced them, so a scope index
/// temporarily *replaces* the active cache. The full index is parked here and restored as
/// soon as a query does not need the scope index.
#[derive(Default)]
pub struct ScopedIndexState {
    /// Scope whose index is currently installed as the active cache.
    pub ready: Option<PathBuf>,
    /// Scope being walked right now, plus the flag that cancels that walk.
    pub building: Option<(PathBuf, &'static AtomicBool)>,
    /// Scope that turned out not to be a directory; do not retry it until it changes.
    pub unusable: Option<PathBuf>,
    /// Full index parked while a scope index is active.
    pub parked_full: Option<SearchCache>,
    /// Scope index parked when the full index was restored. Network folders are expensive to
    /// walk, so switching back to the same scope reuses it instead of walking again.
    pub parked_scope: Option<(PathBuf, SearchCache)>,
}

impl ScopedIndexState {
    /// Cancels an in-flight walk and forgets the installed index root (the cache itself is
    /// replaced by the caller).
    pub fn reset(&mut self) {
        if let Some((_, cancel)) = self.building.take() {
            cancel.store(true, Ordering::Relaxed);
        }
        self.ready = None;
        self.unusable = None;
        self.parked_full = None;
        self.parked_scope = None;
    }

    fn is_ready_for(&self, scope: &Path) -> bool {
        self.ready.as_deref() == Some(scope)
    }

    fn is_unusable(&self, scope: &Path) -> bool {
        self.unusable.as_deref() == Some(scope)
    }

    fn is_building(&self, scope: &Path) -> bool {
        self.building
            .as_ref()
            .is_some_and(|(building, _)| building == scope)
    }

    fn has_parked_scope(&self, scope: &Path) -> bool {
        self.parked_scope
            .as_ref()
            .is_some_and(|(parked, _)| parked == scope)
    }
}

/// Installs `scoped_cache` as the active cache, parking the full index when there is one.
///
/// Returns `true` when the full index got parked, i.e. the event watcher must be stopped so
/// FSEvents are not applied to the scope index (slab indices are cache-local).
fn activate_scoped_cache(
    cache: &mut SearchCache,
    scoped: &mut ScopedIndexState,
    scoped_cache: SearchCache,
    scope: PathBuf,
    full_index_ready: bool,
) -> bool {
    // A previously installed scope index is not the full index; it must not be parked as one.
    let previous_scope = scoped.ready.clone();
    let previous = std::mem::replace(cache, scoped_cache);
    scoped.ready = Some(scope);
    scoped.building = None;
    scoped.unusable = None;
    scoped.parked_scope = None;
    if full_index_ready && previous_scope.is_none() {
        scoped.parked_full = Some(previous);
        true
    } else {
        // Keep the previous scope index so going back to that folder is instant.
        if let Some(previous_scope) = previous_scope {
            scoped.parked_scope = Some((previous_scope, previous));
        }
        false
    }
}

/// Restores the parked full index. Returns `true` when the active cache changed, which means
/// the event watcher has to be restarted.
fn restore_full_cache(cache: &mut SearchCache, scoped: &mut ScopedIndexState) -> bool {
    let Some(full) = scoped.parked_full.take() else {
        return false;
    };
    let previous = std::mem::replace(cache, full);
    if let Some(scope) = scoped.ready.take() {
        scoped.parked_scope = Some((scope, previous));
    }
    true
}

/// Installs a previously used scope index again (no walk needed).
///
/// Returns `true` when the full index got parked, i.e. the event watcher must be stopped.
fn reuse_parked_scope(
    cache: &mut SearchCache,
    scoped: &mut ScopedIndexState,
    scope: &Path,
) -> bool {
    let Some((parked_scope, parked_cache)) =
        scoped.parked_scope.take_if(|(parked, _)| parked == scope)
    else {
        return false;
    };
    // Park whatever is active now: the full index, or a different scope index.
    let is_full_active = scoped.ready.is_none() && scoped.parked_full.is_none();
    let previous = std::mem::replace(cache, parked_cache);
    scoped.ready = Some(parked_scope);
    if is_full_active {
        scoped.parked_full = Some(previous);
        true
    } else {
        false
    }
}

/// What a search request needs from the caches.
#[derive(Debug, PartialEq, Eq)]
enum SearchAction {
    /// The active cache can answer right away.
    Answer,
    /// Restore the parked full index first, then answer.
    RestoreFullAndAnswer,
    /// Swap a previously built scope index back in, then answer.
    ReuseScope,
    /// The scope is not a directory; tell the frontend instead of retrying.
    ScopeUnavailable,
    /// Start a temporary walk of this folder.
    BuildScope(PathBuf),
    /// Nothing can answer yet; the frontend retries later.
    Wait,
}

/// Decides how to serve a query without touching the filesystem or blocking.
///
/// `covered_by_full` tells whether the ready full index already contains the scope folder.
/// Keeping this pure makes the precedence (installed scope index > full index > new walk)
/// testable, which matters because getting it wrong makes scope searches return nothing.
fn decide_search_action(
    scope: Option<&Path>,
    indexing: bool,
    scoped: &ScopedIndexState,
    covered_by_full: bool,
) -> SearchAction {
    if let Some(scope) = scope {
        if scoped.is_ready_for(scope) {
            return SearchAction::Answer;
        }
        if scoped.has_parked_scope(scope) {
            // Walking a network folder costs minutes; reuse the index built earlier.
            return SearchAction::ReuseScope;
        }
        if covered_by_full {
            return SearchAction::RestoreFullAndAnswer;
        }
        if scoped.is_unusable(scope) {
            // Not a directory: polling again would spin forever.
            return SearchAction::ScopeUnavailable;
        }
        if scoped.is_building(scope) {
            return SearchAction::Wait;
        }
        return SearchAction::BuildScope(scope.to_path_buf());
    }

    if indexing {
        // No usable scope and no full index yet.
        return SearchAction::Wait;
    }
    if scoped.ready.is_some() {
        return SearchAction::RestoreFullAndAnswer;
    }
    SearchAction::Answer
}

/// While `Some`, the loop has no complete index yet and is waiting for `IndexBuildResult`.
#[derive(Clone, Copy)]
pub struct IndexingState {
    pub cancellation_token: CancellationToken,
}

pub struct BackgroundLoopChannels {
    pub finish_rx: Receiver<Sender<Option<SearchCache>>>,
    pub update_window_state_rx: Receiver<()>,
    pub search_rx: Receiver<SearchJob>,
    pub node_info_rx: Receiver<NodeInfoRequest>,
    pub icon_viewport_rx: Receiver<IconViewportUpdate>,
    pub recent_nodes_rx: Receiver<RecentNodesRequest>,
    pub rescan_rx: Receiver<CancellationToken>,
    pub watch_config_rx: Receiver<WatchConfigUpdate>,
    pub index_build_rx: Receiver<IndexBuildResult>,
    pub index_build_tx: Sender<IndexBuildResult>,
    pub scoped_index_rx: Receiver<ScopedIndexResult>,
    pub scoped_index_tx: Sender<ScopedIndexResult>,
    pub icon_update_tx: Sender<IconPayload>,
}

pub fn reset_status_bar(app_handle: &AppHandle) {
    app_handle
        .emit(
            "status_bar_update",
            StatusBarUpdate {
                scanned_files: 0,
                processed_events: 0,
                rescan_errors: 0,
            },
        )
        .ok();
}

pub fn emit_status_bar_update(
    app_handle: &AppHandle,
    scanned_files: usize,
    processed_events: usize,
    rescan_errors: usize,
) {
    static LAST_EMIT: Lazy<Mutex<Instant>> =
        Lazy::new(|| Mutex::new(Instant::now() - Duration::from_secs(1)));

    {
        let mut last_emit = LAST_EMIT.lock();
        if Instant::now().duration_since(*last_emit) < Duration::from_millis(100) {
            return;
        }
        // Never panic on a failed emit: this runs on the walk/worker threads and used to be
        // able to kill them (and, through `thread::scope`, the caller).
        let _ = app_handle.emit(
            "status_bar_update",
            StatusBarUpdate {
                scanned_files,
                processed_events,
                rescan_errors,
            },
        );
        *last_emit = Instant::now();
    }
}

/// Runs `f`, converting a panic into a logged error instead of letting it kill the loop.
///
/// The background loop owns the search cache and the only `search_rx` receiver. A panic in a
/// single query used to unwind the whole thread: afterwards every `search` call failed with
/// `RecvError` and the app stayed on "Updating" forever, with no evidence in any log (a bundle
/// launched from Finder has no stderr). Catching here keeps the app usable and records the
/// cause.
pub(crate) fn catch_panic<T>(label: &str, f: impl FnOnce() -> T) -> Result<T, String> {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(value) => Ok(value),
        Err(payload) => {
            let message = panic_payload_message(&payload);
            error!("{label} panicked: {message}");
            Err(message)
        }
    }
}

fn panic_payload_message(payload: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(text) = payload.downcast_ref::<&str>() {
        (*text).to_string()
    } else if let Some(text) = payload.downcast_ref::<String>() {
        text.clone()
    } else {
        "unknown panic".to_string()
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_watch_config_update(
    app_handle: &AppHandle,
    update: WatchConfigUpdate,
    cache: &mut SearchCache,
    event_watcher: &mut EventWatcher,
    watch_root: &mut String,
    history_ready: &mut bool,
    processed_events: &mut usize,
    indexing: &mut Option<IndexingState>,
    scoped: &mut ScopedIndexState,
    db_path: &Path,
    index_build_tx: &Sender<IndexBuildResult>,
) {
    info!("Received watch config update: {:?}", update);
    let WatchConfigUpdate {
        watch_root: next_watch_root,
        ignore_paths,
        include_paths,
        scan_cancellation_token,
    } = update;

    let next_ignore_paths = ignore_paths
        .into_iter()
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    let next_include_paths = include_paths
        .into_iter()
        .map(PathBuf::from)
        .collect::<Vec<_>>();

    // The old cache belongs to the old watch config and cannot be reused. Start the rebuild
    // on the index worker so searches stay responsive (folder scopes can still be served
    // from a temporary index) instead of queueing behind a full walk.
    *event_watcher = EventWatcher::noop();
    update_app_state(app_handle, AppLifecycleState::Initializing);
    reset_status_bar(app_handle);
    *history_ready = false;
    *processed_events = 0;
    scoped.reset();
    *cache = SearchCache::noop(
        PathBuf::from(&next_watch_root),
        next_ignore_paths.clone(),
        next_include_paths.clone(),
        &APP_QUIT,
    );
    *watch_root = next_watch_root.clone();
    *indexing = Some(IndexingState {
        cancellation_token: scan_cancellation_token,
    });

    spawn_index_load(
        app_handle.clone(),
        next_watch_root,
        next_ignore_paths,
        next_include_paths,
        db_path.to_path_buf(),
        scan_cancellation_token,
        false,
        index_build_tx.clone(),
    );
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

fn handle_flush_tick(
    app_handle: &AppHandle,
    cache: &mut SearchCache,
    db_path: &Path,
    hide_flush_remaining_ticks: &mut u8,
) {
    if load_app_state() != AppLifecycleState::Ready {
        return;
    }
    let mut flush_search_cache = FlushSearchCache { cache, db_path };
    let flushed = start_flush_checks(
        || is_main_window_foreground(app_handle),
        search_activity::search_idles,
        &mut flush_search_cache,
        hide_flush_remaining_ticks,
    );
    if flushed {
        search_activity::note_search_activity();
    }
}

fn handle_event_watcher_events(
    app_handle: &AppHandle,
    cache: &mut SearchCache,
    events: Vec<FsEvent>,
    history_ready: &mut bool,
    processed_events: &mut usize,
    recent_snapshot: &mut Option<Vec<SlabIndex>>,
) {
    *processed_events += events.len();

    emit_status_bar_update(
        app_handle,
        cache.get_total_files(),
        *processed_events,
        cache.rescan_count() as usize,
    );

    let mut snapshots = Vec::with_capacity(events.len());
    for event in events.iter() {
        // `contains`, not `==`: the replay's final event can carry additional flag bits, and
        // an exact comparison left the app on "Updating" forever.
        if event.flag.contains(EventFlag::HistoryDone) {
            *history_ready = true;
            update_app_state(app_handle, AppLifecycleState::Ready);
        } else if *history_ready {
            snapshots.push(EventSnapshot {
                path: event.path.clone(),
                event_id: event.id,
                flag: event.flag,
                timestamp: unix_timestamp_now(),
            });
        }
    }

    let event_count = events.len();
    let handle_result = cache.handle_fs_events(events);
    if event_count > 0 {
        // The index changed, so the cached launch ranking is stale.
        *recent_snapshot = None;
    }
    if let Err(HandleFSEError::Rescan) = handle_result {
        info!("!!!!!!!!!! Rescan triggered !!!!!!!!");
        emit_status_bar_update(
            app_handle,
            cache.get_total_files(),
            *processed_events,
            cache.rescan_count() as usize,
        );
    }

    if *history_ready && !snapshots.is_empty() {
        forward_new_events(app_handle, &snapshots);
    }
}

/// How many ranked entries the launch list keeps in memory. Each scroll-to-bottom serves the
/// next page from this snapshot, so it must be comfortably larger than a few pages.
const RECENT_SNAPSHOT_SIZE: usize = 5_000;

fn handle_icon_viewport_update(
    cache: &mut SearchCache,
    update: IconViewportUpdate,
    icon_update_tx: &Sender<IconPayload>,
) {
    let IconViewportUpdate {
        viewport,
        icon_size,
        ..
    } = update;

    let nodes = cache.expand_file_nodes(&viewport);
    let icon_jobs: Vec<_> = viewport
        .into_iter()
        .zip(nodes)
        .map(|(slab_index, SearchResultNode { path, .. })| (slab_index, path))
        .collect();

    if icon_jobs.is_empty() {
        return;
    }

    icon_jobs
        .into_iter()
        .map(|(slab_index, path)| (slab_index, path.to_string_lossy().into_owned()))
        .filter(|(_, path)| {
            // OneDrive
            // iCloud Drive
            // Google Drive
            // Dropbox
            !path.contains("OneDrive")
                && !path.contains("com~apple~CloudDocs")
                && !path.contains("Google Drive")
                && !path.contains("Dropbox")
        })
        .for_each(|(slab_index, path)| {
            let icon_update_tx = icon_update_tx.clone();
            spawn(move || {
                if let Some(icon) = fs_icon::icon_of_path_sized(&path, icon_size).map(|data| {
                    format!(
                        "data:image/png;base64,{}",
                        general_purpose::STANDARD.encode(&data)
                    )
                }) {
                    let _ = icon_update_tx.send(IconPayload { slab_index, icon });
                }
            });
        });
}

/// Turns a literal path from the folder-scope box into a `PathBuf` without touching the disk.
///
/// The scope box accepts the same search syntax as the main query, but arbitrary name queries
/// cannot be resolved here. Literal paths (`/Users/...`, `~/Downloads`, optionally quoted) are
/// the common case. Any filesystem access (`is_dir`, `stat`) happens on the walk worker:
/// a network mount can block for seconds and must never stall the event loop.
fn parse_scope_path(raw: &str) -> Option<PathBuf> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let unquoted = trimmed
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .unwrap_or(trimmed);
    if unquoted.is_empty() {
        return None;
    }

    let home = || -> Option<PathBuf> { std::env::var_os("HOME").map(PathBuf::from) };
    let expanded = if unquoted == "~" {
        home()?
    } else if let Some(rest) = unquoted.strip_prefix("~/") {
        home()?.join(rest)
    } else {
        let path = PathBuf::from(unquoted);
        if path.is_absolute() {
            path
        } else {
            // Bare folder names (`Documents`) are resolved against the home directory;
            // anything else is search syntax that cannot be resolved without an index.
            home()?.join(path)
        }
    };

    (!expanded.as_os_str().is_empty()).then_some(expanded)
}

/// Drops ignore entries that are ancestors of (or equal to) the scope.
///
/// The ignore list is written for the whole-disk walk (`/Volumes`, `~/Library/Caches`, ...).
/// When the user explicitly asks to index one of those folders, keeping the ancestor rule
/// would filter out every child and produce an empty index.
fn ignore_paths_for_scope(scope: &Path, ignore_paths: &[PathBuf]) -> Vec<PathBuf> {
    ignore_paths
        .iter()
        .filter(|ignore| !scope.starts_with(ignore.as_path()))
        .cloned()
        .collect()
}

/// How long a persisted folder-scope index is reused before it is rebuilt.
///
/// Scope indexes are not watched by FSEvents (the watcher only covers the main root), so a
/// snapshot can go stale. Serving a few minutes old listing is much better than making the
/// user wait for a cold network walk again, but unbounded staleness would be wrong.
const SCOPE_CACHE_TTL: Duration = Duration::from_secs(30 * 60);

/// Folder-scope indexes larger than this are not persisted (the main index may cover small
/// scopes anyway, and a huge snapshot costs disk for little benefit).
const SCOPE_CACHE_MAX_NODES: usize = 500_000;

/// Persisted scope snapshots older than this are deleted while writing a new one.
const SCOPE_CACHE_RETENTION: Duration = Duration::from_secs(7 * 24 * 60 * 60);

/// FNV-1a. `DefaultHasher` is not guaranteed to be stable across Rust releases, and a changing
/// file name would silently invalidate every persisted scope snapshot on upgrade.
fn stable_path_hash(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// `.../com.cardinal.one/scopes/scope-<hash>.db`
fn scope_cache_path(db_path: &Path, scope: &Path) -> PathBuf {
    let hash = stable_path_hash(scope.as_os_str().as_encoded_bytes());
    let dir = db_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("scopes");
    dir.join(format!("scope-{hash:016x}.db"))
}

/// True when a persisted snapshot exists and is younger than [`SCOPE_CACHE_TTL`].
fn scope_cache_is_fresh(path: &Path) -> bool {
    std::fs::metadata(path)
        .and_then(|meta| meta.modified())
        .ok()
        .and_then(|modified| modified.elapsed().ok())
        .is_some_and(|age| age <= SCOPE_CACHE_TTL)
}

/// Deletes snapshots that have not been touched for [`SCOPE_CACHE_RETENTION`].
fn prune_scope_caches(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_none_or(|ext| ext != "db") {
            continue;
        }
        let too_old = std::fs::metadata(&path)
            .and_then(|meta| meta.modified())
            .ok()
            .and_then(|modified| modified.elapsed().ok())
            .is_some_and(|age| age > SCOPE_CACHE_RETENTION);
        if too_old {
            let _ = std::fs::remove_file(&path);
        }
    }
}

/// Walks `scope` on a worker thread so the event loop stays responsive.
///
/// `cancel` is leaked on purpose: `SearchCache` needs a `'static` flag, and it lets the loop
/// cancel this walk when the user changes the scope or a full rebuild starts.
fn spawn_scoped_index(
    app_handle: AppHandle,
    scope: PathBuf,
    ignore_paths: Vec<PathBuf>,
    include_paths: Vec<PathBuf>,
    db_path: PathBuf,
    result_tx: Sender<ScopedIndexResult>,
) -> &'static AtomicBool {
    let cancel: &'static AtomicBool = Box::leak(Box::new(AtomicBool::new(false)));
    let walk_cancel = cancel;
    // The scope's own ancestors must not filter out its children.
    let ignore_paths = ignore_paths_for_scope(&scope, &ignore_paths);

    let spawned = std::thread::Builder::new()
        .name("cardinal-scoped-index".to_string())
        .spawn(move || {
            if !scope.is_dir() {
                info!("Folder scope {:?} is not a directory; skipping", scope);
                let _ = result_tx.send(ScopedIndexResult::Unusable { scope });
                return;
            }

            let cache_path = scope_cache_path(&db_path, &scope);

            // A network folder can take minutes to walk, so reuse the snapshot from a recent
            // visit when one exists. Validation inside `try_read_persistent_cache` also
            // refuses snapshots written with a different ignore list.
            if scope_cache_is_fresh(&cache_path) {
                match SearchCache::try_read_persistent_cache(
                    &scope,
                    &cache_path,
                    &ignore_paths,
                    &include_paths,
                    &APP_QUIT,
                ) {
                    Ok(cached) => {
                        info!(
                            "Reusing persisted index for folder scope {:?} ({} nodes)",
                            scope,
                            cached.get_total_files()
                        );
                        let _ = result_tx.send(ScopedIndexResult::Built {
                            scope,
                            cache: Box::new(cached),
                        });
                        return;
                    }
                    Err(e) => info!(
                        "Ignoring persisted index for folder scope {:?}: {e:?}",
                        scope
                    ),
                }
            }

            info!("Building temporary index for folder scope {:?}", scope);
            let cache = build_search_cache_with(
                &scope,
                &ignore_paths,
                &include_paths,
                || APP_QUIT.load(Ordering::Relaxed) || walk_cancel.load(Ordering::Relaxed),
                |total| emit_status_bar_update(&app_handle, total, 0, 0),
            );

            let result = match cache {
                Some(mut cache) => {
                    let nodes = cache.get_total_files();
                    info!("Temporary index for {:?} ready ({nodes} nodes)", scope);
                    if nodes <= SCOPE_CACHE_MAX_NODES {
                        match cache.flush_snapshot_to_file(&cache_path) {
                            Ok(()) => {
                                info!("Persisted folder-scope index to {cache_path:?}");
                                if let Some(dir) = cache_path.parent() {
                                    prune_scope_caches(dir);
                                }
                            }
                            Err(e) => warn!(
                                "Failed to persist folder-scope index to {cache_path:?}: {e:?}"
                            ),
                        }
                    }
                    ScopedIndexResult::Built {
                        scope,
                        cache: Box::new(cache),
                    }
                }
                None => {
                    info!("Temporary index build for {:?} was cancelled", scope);
                    ScopedIndexResult::Unusable { scope }
                }
            };
            let _ = result_tx.send(result);
        });

    if let Err(e) = spawned {
        error!("Failed to spawn scoped index thread: {e:?}");
        cancel.store(true, Ordering::Relaxed);
    }
    cancel
}

/// Loads the persistent cache (fast path) or walks the filesystem on a dedicated thread.
///
/// Keeping this off the event loop means search requests are answered immediately while the
/// initial index is being prepared instead of queueing behind a multi-minute walk.
#[allow(clippy::too_many_arguments)]
pub(crate) fn spawn_index_load(
    app_handle: AppHandle,
    watch_root: String,
    ignore_paths: Vec<PathBuf>,
    include_paths: Vec<PathBuf>,
    db_path: PathBuf,
    cancellation_token: CancellationToken,
    // Manual rescans must not reuse the persisted snapshot.
    force_walk: bool,
    result_tx: Sender<IndexBuildResult>,
) {
    let spawned = std::thread::Builder::new()
        .name("cardinal-initial-index".to_string())
        .spawn(move || {
            let walk = |reason: &str| {
                info!("Walking filesystem ({reason})");
                build_search_cache(
                    &app_handle,
                    &watch_root,
                    &ignore_paths,
                    &include_paths,
                    cancellation_token,
                )
            };
            let (mut cache, should_persist) = if force_walk {
                (walk("manual rescan"), true)
            } else {
                match SearchCache::try_read_persistent_cache(
                    Path::new(&watch_root),
                    &db_path,
                    &ignore_paths,
                    &include_paths,
                    &APP_QUIT,
                ) {
                    Ok(cached) => {
                        info!("Loaded existing cache");
                        (Some(cached), false)
                    }
                    Err(e) => (walk(&format!("cache unavailable: {e:?}")), true),
                }
            };

            // Persist a freshly walked index right away. Without this, quitting before the
            // app reaches `Ready` (which is exactly the slow first run) would force a full
            // re-walk on every launch. A superseded build leaves the write to its successor.
            if should_persist
                && cancellation_token.is_cancelled().is_some()
                && let Some(cache) = cache.as_mut()
            {
                match cache.flush_snapshot_to_file(&db_path) {
                    Ok(()) => info!("Fresh index persisted to {:?}", db_path),
                    Err(e) => error!("Failed to persist fresh index to {:?}: {e:?}", db_path),
                }
            }

            if result_tx
                .send(IndexBuildResult {
                    cancellation_token,
                    cache,
                })
                .is_err()
            {
                info!("Initial index finished after the background loop exited");
            }
        });

    if let Err(e) = spawned {
        error!("Failed to spawn initial index thread: {e:?}");
    }
}

#[allow(clippy::too_many_arguments)]
pub fn run_background_event_loop(
    app_handle: &AppHandle,
    mut cache: SearchCache,
    mut event_watcher: EventWatcher,
    channels: BackgroundLoopChannels,
    mut watch_root: String,
    fse_latency_secs: f64,
    db_path: PathBuf,
    mut indexing: Option<IndexingState>,
) {
    let BackgroundLoopChannels {
        finish_rx,
        update_window_state_rx,
        search_rx,
        node_info_rx,
        icon_viewport_rx,
        recent_nodes_rx,
        rescan_rx,
        watch_config_rx,
        index_build_rx,
        index_build_tx,
        scoped_index_rx,
        scoped_index_tx,
        icon_update_tx,
    } = channels;
    let mut processed_events = 0usize;
    // Ranked launch list, built lazily on the first page request and dropped whenever the
    // index changes (FS events / rescan) so pages never show stale entries.
    let mut recent_snapshot: Option<Vec<SlabIndex>> = None;
    let mut history_ready = load_app_state() == AppLifecycleState::Ready;
    // Temporary per-folder index used when the main index does not cover the scope.
    let mut scoped = ScopedIndexState::default();

    let mut window_is_foreground = true;
    let mut hide_flush_remaining_ticks: u8 = 0;
    // Hide flush is polled on a 10s ticker; idle flush shares the same tick.
    let flush_ticker = crossbeam_channel::tick(Duration::from_secs(10));

    loop {
        crossbeam_channel::select! {
            recv(finish_rx) -> tx => {
                let tx = tx.expect("Finish channel closed");
                // Only save the complete full index. A noop cache (cancelled walk) or a
                // temporary per-scope index must never overwrite the persisted snapshot; a
                // parked full index is the one worth writing.
                let snapshot = if indexing.is_some() {
                    None
                } else if let Some(full) = scoped.parked_full.take() {
                    (!full.is_noop()).then_some(full)
                } else {
                    (!cache.is_noop() && scoped.ready.is_none()).then_some(cache)
                };
                let _ = tx.send(snapshot);
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
                // Never snapshot a temporary scope index (or the empty cache installed while
                // a rebuild runs) over the persisted full index.
                if indexing.is_none() && scoped.ready.is_none() {
                    handle_flush_tick(
                        app_handle,
                        &mut cache,
                        &db_path,
                        &mut hide_flush_remaining_ticks,
                    );
                }
            }
            recv(search_rx) -> job => {
                let SearchJob {
                    query,
                    options,
                    cancellation_token,
                    result_tx
                } = job.expect("Search channel closed");
                let opts = SearchOptions::from(options);

                // Literal folder scope that the active cache may not cover. A scope equal to
                // (or containing) the watch root is not smaller than the main walk.
                let scope = query
                    .directory_query
                    .as_deref()
                    .and_then(parse_scope_path)
                    .filter(|scope| !Path::new(&watch_root).starts_with(scope.as_path()));

                // Only trust `cache` as the full index when no scope index is installed.
                let covered_by_full = scope.as_deref().is_some_and(|s| {
                    indexing.is_none()
                        && scoped.ready.is_none()
                        && cache.node_index_for_path(s).is_some()
                });
                let action = decide_search_action(scope.as_deref(), indexing.is_some(), &scoped, covered_by_full);

                // A panicking query must not take the whole background thread down: convert it
                // into an error result so the caller is answered and the loop keeps running.
                let run_query = |cache: &mut SearchCache| -> SearchJobResult {
                    match catch_panic("search", || {
                        cache.search_query_with_options(query, opts, cancellation_token)
                    }) {
                        Ok(outcome) => SearchJobResult::Completed(outcome),
                        Err(reason) => SearchJobResult::Completed(Err(anyhow::anyhow!(
                            "Search failed: {reason}"
                        ))),
                    }
                };

                let result = match action {
                    SearchAction::Answer => Some(run_query(&mut cache)),
                    SearchAction::ReuseScope => {
                        let parked = scope
                            .as_deref()
                            .is_some_and(|scope| reuse_parked_scope(&mut cache, &mut scoped, scope));
                        if parked {
                            // FSEvents must not be applied to the scope index.
                            event_watcher = EventWatcher::noop();
                        }
                        recent_snapshot = None;
                        Some(run_query(&mut cache))
                    }
                    SearchAction::RestoreFullAndAnswer => {
                        // Non-scoped queries (and scopes the full index covers) must run
                        // against the full index, not a parked scope index.
                        if restore_full_cache(&mut cache, &mut scoped) && !cache.is_noop() {
                            event_watcher = EventWatcher::spawn(
                                watch_root.to_string(),
                                cache.last_event_id(),
                                fse_latency_secs,
                                cache.ignore_paths(),
                                cache.include_paths(),
                            )
                            .1;
                        }
                        recent_snapshot = None;
                        Some(run_query(&mut cache))
                    }
                    SearchAction::BuildScope(scope) => {
                        // Cancel any walk for a previous scope before starting a new one.
                        if let Some((_, cancel)) = scoped.building.take() {
                            cancel.store(true, Ordering::Relaxed);
                        }
                        scoped.unusable = None;
                        let cancel = spawn_scoped_index(
                            app_handle.clone(),
                            scope.clone(),
                            cache.ignore_paths().to_vec(),
                            cache.include_paths().to_vec(),
                            db_path.clone(),
                            scoped_index_tx.clone(),
                        );
                        scoped.building = Some((scope.clone(), cancel));
                        // Walk started in the background; report the scope so the UI can
                        // name it and retry shortly.
                        Some(SearchJobResult::ScopedIndexing {
                            scope: scope.to_string_lossy().into_owned(),
                        })
                    }
                    SearchAction::ScopeUnavailable => Some(SearchJobResult::ScopeUnavailable {
                        scope: scope
                            .as_deref()
                            .map(|scope| scope.to_string_lossy().into_owned())
                            .unwrap_or_default(),
                    }),
                    SearchAction::Wait => None,
                };

                let result = result.unwrap_or_else(|| match scope {
                    Some(scope) => SearchJobResult::ScopedIndexing {
                        scope: scope.to_string_lossy().into_owned(),
                    },
                    None => SearchJobResult::Indexing,
                });
                // The caller may have gone away (window closed/reloaded); never panic.
                let _ = result_tx.send(result);
            }
            recv(scoped_index_rx) -> built => {
                let built = built.expect("Scoped index channel closed");
                match built {
                    ScopedIndexResult::Built { scope, cache: scoped_cache } => {
                        if scoped.is_building(&scope) {
                            let parked = activate_scoped_cache(
                                &mut cache,
                                &mut scoped,
                                *scoped_cache,
                                scope.clone(),
                                indexing.is_none(),
                            );
                            if parked {
                                // FSEvents must not be applied to the scope index.
                                event_watcher = EventWatcher::noop();
                            }
                            recent_snapshot = None;
                            info!("Installed temporary index for {:?}", scope);
                        }
                    }
                    ScopedIndexResult::Unusable { scope } => {
                        if scoped.is_building(&scope) {
                            scoped.building = None;
                            scoped.unusable = Some(scope);
                        }
                    }
                }
            }
            recv(index_build_rx) -> built => {
                let built = built.expect("Index build channel closed");
                let is_active_build = indexing.is_some_and(|active_build| {
                    built.cancellation_token.version()
                        == active_build.cancellation_token.version()
                });

                if is_active_build {
                    match built.cache {
                        Some(next_cache) => {
                            info!(
                                "Initial index ready ({} nodes)",
                                next_cache.get_total_files()
                            );
                            emit_status_bar_update(app_handle, next_cache.get_total_files(), 0, 0);
                            recent_snapshot = None;
                            indexing = None;
                            processed_events = 0;
                            history_ready = false;
                            update_app_state(app_handle, AppLifecycleState::Updating);

                            if scoped.ready.is_some() {
                                // The user is looking at results from a scope index. Replacing
                                // it now would invalidate every row and force a re-walk of
                                // that folder; park the full index instead and swap it in as
                                // soon as a query does not need the scope.
                                scoped.parked_full = Some(next_cache);
                                info!(
                                    "Keeping temporary scope index; full index parked until \
                                     the next non-scoped search"
                                );
                            } else {
                                cache = next_cache;
                                event_watcher = EventWatcher::spawn(
                                    watch_root.to_string(),
                                    cache.last_event_id(),
                                    fse_latency_secs,
                                    cache.ignore_paths(),
                                    cache.include_paths(),
                                )
                                .1;
                            }
                        }
                        None => {
                            // The walk was cancelled (watch config/rescan superseded it, or the
                            // app is quitting). The newer owner installs its own cache; keep the
                            // temporary scoped index usable until then.
                            if !APP_QUIT.load(Ordering::Relaxed) {
                                info!("Initial index build was cancelled before completion");
                            }
                        }
                    }
                }
            }
            recv(recent_nodes_rx) -> request => {
                let request = request.expect("Recent nodes channel closed");
                let RecentNodesRequest {
                    offset,
                    limit,
                    response_tx,
                } = request;
                // Rank ONCE and serve pages from that snapshot. Re-ranking per page is not
                // stable: the scan has a wall-clock stat budget, so two calls can stop at
                // different points and produce overlapping (or missing) pages.
                if recent_snapshot.is_none() {
                    recent_snapshot = cache.most_recent_nodes(
                        RECENT_SNAPSHOT_SIZE,
                        CancellationToken::noop(),
                    );
                }
                let page = recent_snapshot
                    .as_ref()
                    .map(|ranked| {
                        ranked
                            .iter()
                            .skip(offset)
                            .take(limit)
                            .copied()
                            .collect()
                    })
                    .unwrap_or_default();
                let _ = response_tx.send(page);
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
                let update = update.expect("Icon viewport channel closed");
                handle_icon_viewport_update(&mut cache, update, &icon_update_tx);
            }
            recv(rescan_rx) -> request => {
                let scan_cancellation_token = request.expect("Rescan channel closed");
                info!("Manual rescan requested");
                if indexing.is_some() {
                    // A rescan issued while the initial index builds supersedes that build.
                    // Restart it in the background instead of rebuilding whatever partial
                    // cache happens to be installed (possibly a folder-scope index).
                    let ignore_paths = cache.ignore_paths().to_vec();
                    let include_paths = cache.include_paths().to_vec();
                    cache = SearchCache::noop(
                        PathBuf::from(&watch_root),
                        ignore_paths.clone(),
                        include_paths.clone(),
                        &APP_QUIT,
                    );
                    scoped.reset();
                    recent_snapshot = None;
                    indexing = Some(IndexingState {
                        cancellation_token: scan_cancellation_token,
                    });
                    event_watcher = EventWatcher::noop();
                    history_ready = false;
                    processed_events = 0;
                    update_app_state(app_handle, AppLifecycleState::Initializing);
                    reset_status_bar(app_handle);
                    spawn_index_load(
                        app_handle.clone(),
                        watch_root.clone(),
                        ignore_paths,
                        include_paths,
                        db_path.clone(),
                        scan_cancellation_token,
                        true,
                        index_build_tx.clone(),
                    );
                } else {
                    perform_rescan(
                        app_handle,
                        &mut cache,
                        &mut event_watcher,
                        &watch_root,
                        fse_latency_secs,
                        &mut history_ready,
                        &mut processed_events,
                        scan_cancellation_token,
                        &mut indexing,
                        &mut scoped,
                    );
                }
            }
            recv(watch_config_rx) -> update => {
                let next_update = update.expect("Watch config channel closed");
                handle_watch_config_update(
                    app_handle,
                    next_update,
                    &mut cache,
                    &mut event_watcher,
                    &mut watch_root,
                    &mut history_ready,
                    &mut processed_events,
                    &mut indexing,
                    &mut scoped,
                    &db_path,
                    &index_build_tx,
                );
            }
            recv(event_watcher) -> events => {
                let events = events.expect("Event stream closed");
                // A malformed event batch must not take the loop down with it.
                let handled = catch_panic("handle_fs_events", || {
                    handle_event_watcher_events(
                        app_handle,
                        &mut cache,
                        events,
                        &mut history_ready,
                        &mut processed_events,
                        &mut recent_snapshot,
                    )
                });
                if handled.is_err() {
                    recent_snapshot = None;
                }
            }
        }
    }
}

pub(crate) fn build_search_cache(
    app_handle: &AppHandle,
    watch_root: &str,
    ignore_paths: &[PathBuf],
    include_paths: &[PathBuf],
    scan_cancellation_token: CancellationToken,
) -> Option<SearchCache> {
    build_search_cache_with(
        Path::new(watch_root),
        ignore_paths,
        include_paths,
        || APP_QUIT.load(Ordering::Relaxed) || scan_cancellation_token.is_cancelled().is_none(),
        |total| emit_status_bar_update(app_handle, total, 0, 0),
    )
}

/// Walks `root` and reports progress through `on_progress` every 100 ms.
///
/// Split from [`build_search_cache`] so tests (and scoped walks) do not need an `AppHandle`.
pub(crate) fn build_search_cache_with<F, P>(
    root: &Path,
    ignore_paths: &[PathBuf],
    include_paths: &[PathBuf],
    cancel: F,
    on_progress: P,
) -> Option<SearchCache>
where
    F: Fn() -> bool + Send + Sync,
    P: Fn(usize) + Send + Sync,
{
    let walk_data = WalkData::new(root, ignore_paths, include_paths, false, cancel);
    let walking_done = AtomicBool::new(false);

    std::thread::scope(|s| {
        s.spawn(|| {
            while !walking_done.load(Ordering::Relaxed) {
                let dirs = walk_data.num_dirs.load(Ordering::Relaxed);
                let files = walk_data.num_files.load(Ordering::Relaxed);
                on_progress(dirs + files);
                std::thread::sleep(Duration::from_millis(100));
            }
        });
        let cache = SearchCache::walk_fs_with_walk_data(&walk_data, &APP_QUIT);
        walking_done.store(true, Ordering::Relaxed);
        cache
    })
}

#[allow(clippy::too_many_arguments)]
fn perform_rescan(
    app_handle: &AppHandle,
    cache: &mut SearchCache,
    event_watcher: &mut EventWatcher,
    watch_root: &str,
    fse_latency_secs: f64,
    history_ready: &mut bool,
    processed_events: &mut usize,
    scan_cancellation_token: CancellationToken,
    indexing: &mut Option<IndexingState>,
    scoped: &mut ScopedIndexState,
) {
    if scan_cancellation_token.is_cancelled().is_none() {
        info!("Skipping stale rescan request");
        return;
    }

    // A rescan replaces the whole cache; any in-flight initial build or temporary
    // folder-scope index is obsolete.
    *indexing = None;
    scoped.reset();
    *event_watcher = EventWatcher::noop();
    update_app_state(app_handle, AppLifecycleState::Initializing);
    *history_ready = false;
    *processed_events = 0;
    reset_status_bar(app_handle);

    let mut phantom1 = PathBuf::new();
    let mut phantom2 = Vec::new();
    let mut phantom3 = Vec::new();
    let walk_data = cache.walk_data(
        &mut phantom1,
        &mut phantom2,
        &mut phantom3,
        scan_cancellation_token,
    );
    let walking_done = AtomicBool::new(false);
    let stopped = std::thread::scope(|s| {
        s.spawn(|| {
            while !walking_done.load(Ordering::Relaxed) {
                let dirs = walk_data.num_dirs.load(Ordering::Relaxed);
                let files = walk_data.num_files.load(Ordering::Relaxed);
                let total = dirs + files;
                emit_status_bar_update(app_handle, total, 0, 0);
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
        update_app_state(app_handle, AppLifecycleState::Updating);
        EventWatcher::spawn(
            watch_root.to_string(),
            cache.last_event_id(),
            fse_latency_secs,
            cache.ignore_paths(),
            cache.include_paths(),
        )
        .1
    };
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

    fn describe_search_action(
        scope: Option<&Path>,
        indexing: bool,
        scoped: &ScopedIndexState,
        covered_by_full: bool,
    ) -> &'static str {
        match decide_search_action(scope, indexing, scoped, covered_by_full) {
            SearchAction::Answer => "answer",
            SearchAction::RestoreFullAndAnswer => "restore-full",
            SearchAction::ReuseScope => "reuse-scope",
            SearchAction::ScopeUnavailable => "scope-unavailable",
            SearchAction::BuildScope(_) => "build-scope",
            SearchAction::Wait => "wait",
        }
    }

    fn temp_dir_for(label: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default();
        std::env::temp_dir().join(format!("cardinal-{label}-{}-{unique}", std::process::id()))
    }

    #[test]
    fn scope_walk_drops_ignores_of_its_own_ancestors() {
        let ignore = vec![
            PathBuf::from("/Volumes"),
            PathBuf::from("/private/var"),
            PathBuf::from("/Users/smith/Library/Caches"),
        ];

        // `/Volumes` alone would filter out every child of the requested scope.
        assert_eq!(
            ignore_paths_for_scope(Path::new("/Volumes/nfs_D"), &ignore),
            vec![
                PathBuf::from("/private/var"),
                PathBuf::from("/Users/smith/Library/Caches")
            ]
        );

        // Ignores *inside* the scope still apply.
        assert_eq!(
            ignore_paths_for_scope(Path::new("/Users/smith"), &ignore),
            vec![
                PathBuf::from("/Volumes"),
                PathBuf::from("/private/var"),
                PathBuf::from("/Users/smith/Library/Caches")
            ]
        );

        // A path that merely shares a prefix is not an ancestor.
        assert_eq!(
            ignore_paths_for_scope(Path::new("/VolumesExtra/x"), &ignore),
            ignore
        );
    }

    #[test]
    fn scoped_walk_indexes_files_under_an_ignored_ancestor() {
        let root = temp_dir_for("scope-ignored");
        let scope = root.join("nfs_D");
        std::fs::create_dir_all(&scope).expect("create scope dir");
        std::fs::write(scope.join("report.txt"), b"x").expect("write file");

        let search_count = |cache: &mut SearchCache| {
            cache
                .search_query_with_options(
                    search_cache::SearchQuery {
                        directory_query: Some(scope.to_string_lossy().into_owned()),
                        query: Some("report".to_string()),
                    },
                    SearchOptions::default(),
                    CancellationToken::noop(),
                )
                .expect("scoped search")
                .nodes
                .expect("completed")
                .len()
        };

        // Simulate the real config: the parent directory is ignored for the full walk.
        let ignore = vec![root.clone()];
        let filtered = ignore_paths_for_scope(&scope, &ignore);
        let mut cache = build_search_cache_with(&scope, &filtered, &[], || false, |_| {})
            .expect("scoped walk should complete");
        assert_eq!(
            search_count(&mut cache),
            1,
            "a scope inside an ignored ancestor must still be indexed"
        );

        // Without filtering, the ancestor ignore removes every child of the scope.
        let mut unfiltered = build_search_cache_with(&scope, &ignore, &[], || false, |_| {})
            .expect("walk completes");
        assert_eq!(
            search_count(&mut unfiltered),
            0,
            "an ancestor ignore filters out every child, which is why filtering matters"
        );

        std::fs::remove_dir_all(&root).expect("cleanup temp dir");
    }

    #[test]
    fn search_action_prefers_installed_scope_then_full_index() {
        let scope = PathBuf::from("/Volumes/nfs_D");
        let mut state = ScopedIndexState::default();

        // No index at all: a literal folder scope is walked right away instead of waiting
        // for the full walk to reach that folder.
        assert_eq!(
            describe_search_action(Some(&scope), true, &state, false),
            "build-scope"
        );
        // Without a scope there is nothing to do but wait for the full index.
        assert_eq!(describe_search_action(None, true, &state, false), "wait");

        // Full index does not contain the folder (e.g. it lives under an ignored `/Volumes`).
        assert_eq!(
            describe_search_action(Some(&scope), false, &state, false),
            "build-scope"
        );
        // The full index covers it: answer from the full index.
        assert_eq!(
            describe_search_action(Some(&scope), false, &state, true),
            "restore-full"
        );

        // While the walk runs, do not start a second one.
        let cancel: &'static AtomicBool = Box::leak(Box::new(AtomicBool::new(false)));
        state.building = Some((scope.clone(), cancel));
        assert_eq!(
            describe_search_action(Some(&scope), true, &state, false),
            "wait"
        );

        // Once installed, the scope index must win over everything else, otherwise the
        // query would run against the full index and return nothing.
        state.building = None;
        state.ready = Some(scope.clone());
        assert_eq!(
            describe_search_action(Some(&scope), false, &state, false),
            "answer"
        );

        // A non-scoped query restores the full index that was parked.
        assert_eq!(
            describe_search_action(None, false, &state, false),
            "restore-full"
        );

        // Not a directory: report it instead of retrying forever.
        let state = ScopedIndexState {
            unusable: Some(scope.clone()),
            ..Default::default()
        };
        assert_eq!(
            describe_search_action(Some(&scope), true, &state, false),
            "scope-unavailable"
        );
    }

    #[test]
    fn scope_cache_path_is_stable_and_per_scope() {
        let db = PathBuf::from("/tmp/app/cardinal.db");
        let a = scope_cache_path(&db, Path::new("/Volumes/nfs_D"));
        let b = scope_cache_path(&db, Path::new("/Volumes/nfs_D"));
        let c = scope_cache_path(&db, Path::new("/Volumes/nfs_E"));

        assert_eq!(a, b, "the same scope must map to the same file");
        assert_ne!(a, c, "different scopes must not share a snapshot");
        assert_eq!(a.parent(), Some(Path::new("/tmp/app/scopes")));
        assert_eq!(a.extension().unwrap(), "db");
        // FNV-1a of an empty input is the offset basis; guards against a swapped-in hasher.
        assert_eq!(stable_path_hash(b""), 0xcbf2_9ce4_8422_2325);
    }

    #[test]
    fn history_done_is_detected_when_combined_with_other_flags() {
        // Regression: the loop compared the flag with `==`, so a HistoryDone event carrying any
        // other bit left the app on "Updating" forever (the cache was fine, the watcher ran,
        // and searches answered — but the lifecycle never advanced).
        let combined = EventFlag::HistoryDone | EventFlag::ItemIsDir | EventFlag::MustScanSubDirs;
        assert!(
            combined.contains(EventFlag::HistoryDone),
            "HistoryDone must be detected through `contains`"
        );
        assert_ne!(
            combined,
            EventFlag::HistoryDone,
            "the combined value is what broke the exact comparison"
        );
    }

    #[test]
    fn catch_panic_converts_a_panic_into_an_error() {
        // A panicking query used to unwind the whole background thread, leaving every later
        // search with `RecvError`.
        let ok = catch_panic("test", || 7);
        assert_eq!(ok, Ok(7));

        let failed = catch_panic("test", || -> i32 { panic!("boom") });
        assert!(failed.is_err());
        assert!(failed.unwrap_err().contains("boom"));
    }

    #[test]
    fn scope_cache_freshness_respects_ttl() {
        let dir = temp_dir_for("scope-ttl");
        std::fs::create_dir_all(&dir).expect("create dir");
        let file = dir.join("scope.db");

        assert!(
            !scope_cache_is_fresh(&file),
            "missing snapshot is not fresh"
        );

        std::fs::write(&file, b"x").expect("write snapshot");
        assert!(
            scope_cache_is_fresh(&file),
            "just-written snapshot is fresh"
        );

        std::fs::remove_dir_all(&dir).expect("cleanup temp dir");
    }

    #[test]
    fn persisted_scope_snapshot_round_trips() {
        let root = temp_dir_for("scope-persist");
        let scope = root.join("scope");
        let skipped = scope.join("skipme");
        std::fs::create_dir_all(scope.join("nested")).expect("create scope dir");
        std::fs::create_dir_all(&skipped).expect("create skipped dir");
        std::fs::write(scope.join("alpha.txt"), b"a").expect("write file");
        std::fs::write(scope.join("nested").join("beta.txt"), b"b").expect("write file");
        std::fs::write(skipped.join("gamma.txt"), b"c").expect("write file");

        // The ignore list used for writing and reading must be identical, otherwise
        // `try_read_persistent_cache` rejects the snapshot as inconsistent. The ancestor
        // (`root`) is dropped, the entry inside the scope survives.
        let db_path = root.join("cardinal.db");
        let cache_path = scope_cache_path(&db_path, &scope);
        let ignore = ignore_paths_for_scope(&scope, &[root.clone(), skipped.clone()]);
        assert_eq!(ignore, vec![skipped.clone()], "ancestor ignore is dropped");

        let mut cache = build_search_cache_with(&scope, &ignore, &[], || false, |_| {})
            .expect("walk completes");
        let node_count = cache.get_total_files();
        cache
            .flush_snapshot_to_file(&cache_path)
            .expect("persist scope snapshot");
        assert!(scope_cache_is_fresh(&cache_path));

        let mut reloaded = SearchCache::try_read_persistent_cache(
            &scope,
            &cache_path,
            &ignore,
            &Vec::new(),
            &APP_QUIT,
        )
        .expect("persisted snapshot is readable");
        assert_eq!(reloaded.get_total_files(), node_count);
        let hits = reloaded
            .search_query_with_options(
                search_cache::SearchQuery {
                    directory_query: Some(scope.to_string_lossy().into_owned()),
                    query: Some("beta".to_string()),
                },
                SearchOptions::default(),
                CancellationToken::noop(),
            )
            .expect("search")
            .nodes
            .expect("completed");
        assert_eq!(hits.len(), 1, "nested file is searchable after reload");
        let skipped_hits = reloaded
            .search_query_with_options(
                search_cache::SearchQuery {
                    directory_query: Some(scope.to_string_lossy().into_owned()),
                    query: Some("gamma".to_string()),
                },
                SearchOptions::default(),
                CancellationToken::noop(),
            )
            .expect("search")
            .nodes
            .expect("completed");
        assert_eq!(
            skipped_hits.len(),
            0,
            "ignored subdirectories stay excluded after reload"
        );

        // A different ignore list must not silently reuse the snapshot.
        assert!(
            SearchCache::try_read_persistent_cache(
                &scope,
                &cache_path,
                &Vec::new(),
                &Vec::new(),
                &APP_QUIT
            )
            .is_err(),
            "snapshots written with another ignore list must be rejected"
        );

        std::fs::remove_dir_all(&root).expect("cleanup temp dir");
    }

    #[test]
    fn scoped_index_answers_scope_queries_without_blocking() {
        let root = temp_dir_for("scope-idx");
        let scope = root.join("scope");
        std::fs::create_dir_all(scope.join("nested")).expect("create scope dir");
        std::fs::write(scope.join("alpha.txt"), b"a").expect("write file");
        std::fs::write(scope.join("nested").join("beta.txt"), b"b").expect("write file");

        // A worker-thread walk must produce a cache that answers searches scoped to the
        // folder, including files in nested subdirectories.
        let cache = build_search_cache_with(&scope, &[], &[], || false, |_| {});
        let mut cache = cache.expect("scoped walk should complete");
        assert!(cache.get_total_files() > 0, "temporary index is populated");

        let scope_query = scope.to_string_lossy().into_owned();
        for (query, expected) in [("alpha", 1), ("beta", 1)] {
            let outcome = cache
                .search_query_with_options(
                    search_cache::SearchQuery {
                        directory_query: Some(scope_query.clone()),
                        query: Some(query.to_string()),
                    },
                    SearchOptions::default(),
                    CancellationToken::noop(),
                )
                .expect("scoped search");
            assert_eq!(
                outcome.nodes.expect("completed search").len(),
                expected,
                "query {query} inside the folder scope"
            );
        }

        std::fs::remove_dir_all(&root).expect("cleanup temp dir");
    }

    #[test]
    fn scoped_cache_parking_never_treats_a_scope_index_as_the_full_one() {
        let root = temp_dir_for("scope-parking");
        let full_dir = root.join("full");
        let scope_a = root.join("a");
        let scope_b = root.join("b");
        for dir in [&full_dir, &scope_a, &scope_b] {
            std::fs::create_dir_all(dir).expect("create dir");
            std::fs::write(dir.join("file.txt"), b"x").expect("write file");
        }
        let walk = |dir: &Path| {
            build_search_cache_with(dir, &[], &[], || false, |_| {}).expect("walk completes")
        };
        let full_count = walk(&full_dir).get_total_files();

        let mut cache = walk(&full_dir);
        let mut scoped = ScopedIndexState::default();

        // Full index active -> installing a scope index parks it.
        assert!(
            activate_scoped_cache(
                &mut cache,
                &mut scoped,
                walk(&scope_a),
                scope_a.clone(),
                true
            ),
            "the full index must be parked so it can be restored"
        );
        assert_eq!(scoped.ready.as_deref(), Some(scope_a.as_path()));
        assert!(scoped.parked_full.is_some());

        // Switching to another scope must not park the first scope index as if it were full,
        // but it must keep it around for reuse.
        assert!(
            !activate_scoped_cache(
                &mut cache,
                &mut scoped,
                walk(&scope_b),
                scope_b.clone(),
                true
            ),
            "a scope index must never be parked as the full index"
        );
        assert_eq!(scoped.ready.as_deref(), Some(scope_b.as_path()));
        assert!(scoped.has_parked_scope(&scope_a));

        // Restoring brings the original full index back and parks the scope for reuse.
        assert!(restore_full_cache(&mut cache, &mut scoped));
        assert!(scoped.ready.is_none());
        assert_eq!(cache.get_total_files(), full_count);
        assert!(scoped.has_parked_scope(&scope_b));

        // Reusing it must not walk the folder again and must swap the caches back.
        assert!(reuse_parked_scope(&mut cache, &mut scoped, &scope_b));
        assert_eq!(scoped.ready.as_deref(), Some(scope_b.as_path()));
        assert!(!scoped.has_parked_scope(&scope_b));
        assert_eq!(
            describe_search_action(Some(&scope_b), false, &scoped, false),
            "answer"
        );

        std::fs::remove_dir_all(&root).expect("cleanup temp dir");
    }

    #[test]
    fn scoped_state_tracks_ready_building_and_unusable_scopes() {
        let a = PathBuf::from("/tmp/a");
        let b = PathBuf::from("/tmp/b");
        let mut state = ScopedIndexState::default();

        assert!(!state.is_ready_for(&a));
        state.ready = Some(a.clone());
        assert!(state.is_ready_for(&a));
        assert!(!state.is_ready_for(&b));

        let cancel: &'static AtomicBool = Box::leak(Box::new(AtomicBool::new(false)));
        state.building = Some((b.clone(), cancel));
        assert!(state.is_building(&b));
        assert!(!state.is_building(&a));

        state.unusable = Some(PathBuf::from("/tmp/c"));
        assert!(state.is_unusable(Path::new("/tmp/c")));

        // Resetting cancels the in-flight walk and forgets the installed scope.
        state.reset();
        assert!(!state.is_ready_for(&a));
        assert!(!state.is_building(&b));
        assert!(!state.is_unusable(Path::new("/tmp/c")));
        assert!(
            cancel.load(Ordering::Relaxed),
            "reset must cancel the running walk"
        );
    }

    #[test]
    fn parse_scope_path_expands_home_and_quotes_without_touching_disk() {
        let home = std::env::var_os("HOME").expect("HOME is set in tests");
        let home = PathBuf::from(home);

        assert_eq!(parse_scope_path("~"), Some(home.clone()));
        assert_eq!(
            parse_scope_path("  ~/Documents  "),
            Some(home.join("Documents"))
        );
        assert_eq!(
            parse_scope_path("\"/Users/smith/Desktop\""),
            Some(PathBuf::from("/Users/smith/Desktop")),
            "quoted paths are accepted"
        );
        assert_eq!(
            parse_scope_path("Documents"),
            Some(home.join("Documents")),
            "bare names resolve against the home directory"
        );
        // Resolution must be pure: a non-existent path is still parsed, and the walker
        // decides whether it is a directory (it may live on a slow network mount).
        assert_eq!(
            parse_scope_path("/definitely/not/a/real/dir-xyz"),
            Some(PathBuf::from("/definitely/not/a/real/dir-xyz"))
        );
        assert_eq!(parse_scope_path(""), None);
        assert_eq!(parse_scope_path("   "), None);
        assert_eq!(parse_scope_path("\"\""), None);
    }
}
