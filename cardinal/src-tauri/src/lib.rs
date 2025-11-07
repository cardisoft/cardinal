// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use anyhow::{Context, Result};
use base64::{Engine as _, engine::general_purpose};
use cardinal_sdk::{EventFlag, EventWatcher};
use crossbeam_channel::{Receiver, Sender, bounded, unbounded};
use rayon::spawn;
use search_cache::{
    HandleFSEError, SearchCache, SearchOptions, SearchResultNode, SlabIndex, SlabNodeMetadata,
    WalkData,
};
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    sync::{
        LazyLock, Once,
        atomic::{AtomicBool, AtomicU8, Ordering},
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Emitter, Manager, RunEvent, Runtime, State, WebviewWindow, WindowEvent};
use tauri_plugin_global_shortcut::ShortcutState;
use tracing::{error, info, level_filters::LevelFilter, warn};
use tracing_subscriber::EnvFilter;

static CACHE_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
    directories::ProjectDirs::from("", "", "Cardinal")
        .expect(
            "Failed to get ProjectDirs: no valid home directory \
                path could be retrieved from the operating system",
        )
        .config_dir()
        .join("cardinal.db")
});
static APP_QUIT: AtomicBool = AtomicBool::new(false);
static EXIT_REQUESTED: AtomicBool = AtomicBool::new(false);
const QUICK_LAUNCH_SHORTCUT: &str = "CmdOrCtrl+Shift+Space";

#[cfg(desktop)]
const TRAY_MENU_QUIT_ID: &str = "tray.quit_cardinal";

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AppLifecycleState {
    Initializing = 0,
    Ready = 1,
}

impl AppLifecycleState {
    fn from_u8(value: u8) -> Self {
        match value {
            1 => Self::Ready,
            _ => Self::Initializing,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Initializing => "Initializing",
            Self::Ready => "Ready",
        }
    }
}

static APP_LIFECYCLE_STATE: AtomicU8 = AtomicU8::new(AppLifecycleState::Initializing as u8);

fn load_app_state() -> AppLifecycleState {
    AppLifecycleState::from_u8(APP_LIFECYCLE_STATE.load(Ordering::Acquire))
}

fn store_app_state(state: AppLifecycleState) {
    APP_LIFECYCLE_STATE.store(state as u8, Ordering::Release);
}

fn emit_app_state(app_handle: &tauri::AppHandle) {
    if let Err(err) = app_handle.emit("app_lifecycle_state", load_app_state().as_str()) {
        error!("Failed to emit app_lifecycle_state event: {:?}", err);
    }
}

fn update_app_state(app_handle: &tauri::AppHandle, state: AppLifecycleState) {
    if load_app_state() == state {
        return;
    }
    store_app_state(state);
    emit_app_state(app_handle);
}

fn ensure_main_window_visible<R: Runtime>(app_handle: &AppHandle<R>) -> Option<WebviewWindow<R>> {
    let window = app_handle.get_webview_window("main")?;

    if let Ok(true) = window.is_minimized() {
        if let Err(err) = window.unminimize() {
            error!(?err, "Failed to unminimize window");
        }
    }

    if let Ok(false) = window.is_visible() {
        if let Err(err) = window.show() {
            error!(?err, "Failed to show window");
        }
    }

    if let Err(err) = window.set_focus() {
        error!(?err, "Failed to focus window");
    }

    Some(window)
}

fn hide_main_window<R: Runtime>(app_handle: &AppHandle<R>) -> bool {
    if let Some(window) = app_handle.get_webview_window("main") {
        if let Err(err) = window.hide() {
            error!(?err, "Failed to hide main window");
            return false;
        }
        return true;
    }

    warn!("Hide requested but main window is unavailable");
    false
}

fn toggle_main_window<R: Runtime>(app_handle: &AppHandle<R>) {
    let Some(window) = app_handle.get_webview_window("main") else {
        warn!("Toggle requested but main window is unavailable");
        return;
    };

    let is_visible = window.is_visible().unwrap_or(true);
    let is_minimized = window.is_minimized().unwrap_or(false);

    if is_visible && !is_minimized {
        if hide_main_window(app_handle) {
            info!("Global shortcut hid the Cardinal window");
        }
    } else {
        trigger_quick_launch(app_handle);
    }
}

fn trigger_quick_launch<R: Runtime>(app_handle: &AppHandle<R>) {
    let Some(window) = ensure_main_window_visible(app_handle) else {
        error!("Quick launch shortcut triggered but main window is unavailable");
        return;
    };

    if let Err(err) = window.emit("quick_launch", ()) {
        error!(?err, "Failed to emit quick launch event");
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct SearchOptionsPayload {
    #[serde(default)]
    use_regex: bool,
    #[serde(default)]
    case_insensitive: bool,
}

impl From<SearchOptionsPayload> for SearchOptions {
    fn from(
        SearchOptionsPayload {
            use_regex,
            case_insensitive,
        }: SearchOptionsPayload,
    ) -> Self {
        SearchOptions {
            use_regex,
            case_insensitive,
        }
    }
}

#[derive(Debug, Clone)]
struct SearchJob {
    query: String,
    options: SearchOptionsPayload,
}

struct SearchState {
    search_tx: Sender<SearchJob>,
    result_rx: Receiver<Result<Vec<SlabIndex>>>,

    node_info_tx: Sender<Vec<SlabIndex>>,
    node_info_results_rx: Receiver<Vec<SearchResultNode>>,

    icon_viewport_tx: Sender<(u64, Vec<SlabIndex>)>,
    rescan_tx: Sender<()>,
}

#[tauri::command]
async fn search(
    query: String,
    options: Option<SearchOptionsPayload>,
    state: State<'_, SearchState>,
) -> Result<Vec<SlabIndex>, String> {
    let options = options.unwrap_or_default();
    // Send the search request to the background worker.
    state
        .search_tx
        .send(SearchJob { query, options })
        .map_err(|e| format!("Failed to send search request: {e:?}"))?;

    // Wait for the search result.
    let search_result = state
        .result_rx
        .recv()
        .map_err(|e| format!("Failed to receive search result: {e:?}"))?;

    // Propagate any search errors back to the caller.
    search_result.map_err(|e| format!("Failed to process search result: {e:?}"))
}

#[derive(Serialize)]
struct NodeInfo {
    path: String,
    metadata: Option<NodeInfoMetadata>,
    icon: Option<String>,
}

#[derive(Serialize)]
struct NodeInfoMetadata {
    r#type: u8,
    size: u64,
    ctime: u32,
    mtime: u32,
}

impl NodeInfoMetadata {
    pub fn from_metadata(metadata: SlabNodeMetadata<'_>) -> Self {
        Self {
            r#type: metadata.r#type() as u8,
            size: metadata.size(),
            ctime: metadata.ctime().map(|x| x.get()).unwrap_or_default(),
            mtime: metadata.mtime().map(|x| x.get()).unwrap_or_default(),
        }
    }
}

fn unix_timestamp_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

struct EventSnapshot {
    path: PathBuf,
    event_id: u64,
    flag: EventFlag,
    timestamp: i64,
}

// Forward new events to frontend without storing them in Rust
fn forward_new_events(app_handle: &tauri::AppHandle, snapshots: &[EventSnapshot]) {
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

    // Emit new events to frontend - frontend will maintain the list
    let _ = app_handle.emit("fs_events_batch", new_events);
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct RecentEvent {
    path: String,
    flag_bits: u32,
    event_id: u64,
    timestamp: i64,
}

#[derive(Serialize, Clone)]
struct StatusBarUpdate {
    scanned_files: usize,
    processed_events: usize,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct IconPayload {
    slab_index: SlabIndex,
    icon: String,
}

// TODO(ldm0): refactor to reduce argument count
#[allow(clippy::too_many_arguments)]
fn run_background_event_loop(
    app_handle: &tauri::AppHandle,
    mut cache: SearchCache,
    mut event_watcher: EventWatcher,
    finish_rx: Receiver<Sender<Option<SearchCache>>>,
    search_rx: Receiver<SearchJob>,
    result_tx: Sender<Result<Vec<SlabIndex>>>,
    node_info_rx: Receiver<Vec<SlabIndex>>,
    node_info_results_tx: Sender<Vec<SearchResultNode>>,
    icon_viewport_rx: Receiver<(u64, Vec<SlabIndex>)>,
    rescan_rx: Receiver<()>,
    icon_update_tx: Sender<IconPayload>,
    watch_root: &str,
    fse_latency_secs: f64,
) {
    let mut processed_events = 0usize;
    let mut history_ready = matches!(load_app_state(), AppLifecycleState::Ready);
    loop {
        crossbeam_channel::select! {
            recv(finish_rx) -> tx => {
                let tx = tx.expect("Finish channel closed");
                tx.send(Some(cache)).expect("Failed to send cache");
                return;
            }
            recv(search_rx) -> job => {
                let SearchJob { query, options } = job.expect("Search channel closed");
                let opts: SearchOptions = options.into();
                let result = if query.is_empty() {
                    Ok(cache.search_empty())
                } else {
                    cache.search_with_options(&query, opts)
                };
                result_tx.send(result).expect("Failed to send result");
            }
            recv(node_info_rx) -> results => {
                let results = results.expect("Node info channel closed");
                let node_info_results = cache.expand_file_nodes(&results);
                node_info_results_tx.send(node_info_results).expect("Failed to send node info results");
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
                    // Filter out OneDrive&iCloud paths to avoid triggering file download
                    .filter(|(_, path)| !path.contains("OneDrive") && !path.contains("com~apple~CloudDocs"))
                    .for_each(|(slab_index, path)| {
                        let icon_update_tx = icon_update_tx.clone();
                        spawn(move || {
                            if let Some(icon) = fs_icon::icon_of_path_ql(&path).map(|data| format!(
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
                update_app_state(app_handle, AppLifecycleState::Initializing);
                app_handle
                    .emit(
                        "status_bar_update",
                        StatusBarUpdate {
                            scanned_files: 0,
                            processed_events: 0,
                        },
                    )
                    .unwrap();

                #[allow(unused_assignments)]
                {
                    event_watcher = EventWatcher::noop();
                }

                let walk_data = cache.walk_data();
                let walking_done = AtomicBool::new(false);
                std::thread::scope(|s| {
                    s.spawn(|| {
                        while !walking_done.load(Ordering::Relaxed) {
                            let dirs = walk_data.num_dirs.load(Ordering::Relaxed);
                            let files = walk_data.num_files.load(Ordering::Relaxed);
                            let total = dirs + files;
                            app_handle
                                .emit(
                                    "status_bar_update",
                                    StatusBarUpdate {
                                        scanned_files: total,
                                        processed_events: 0,
                                    },
                                )
                                .unwrap();
                            std::thread::sleep(Duration::from_millis(100));
                        }
                    });
                    cache.rescan_with_walk_data(&walk_data);
                    walking_done.store(true, Ordering::Relaxed);
                });

                let (_, watcher) = EventWatcher::spawn(
                    watch_root.to_string(),
                    cache.last_event_id(),
                    fse_latency_secs,
                );
                event_watcher = watcher;
                history_ready = false;
            }
            recv(event_watcher) -> events => {
                let events = events.expect("Event stream closed");
                processed_events += events.len();

                // Emit a status-bar update with the current scan and event counts.
                app_handle.emit("status_bar_update", StatusBarUpdate {
                    scanned_files: cache.get_total_files(),
                    processed_events
                }).unwrap();

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
                    #[allow(unused_assignments)]
                    {
                        event_watcher = EventWatcher::noop();
                    }
                    cache.rescan();
                    event_watcher = EventWatcher::spawn(watch_root.to_string(), cache.last_event_id(), fse_latency_secs).1;
                    update_app_state(app_handle, AppLifecycleState::Initializing);
                    history_ready = false;
                }

                if history_ready && !snapshots.is_empty() {
                    forward_new_events(app_handle, &snapshots);
                }
            }
        }
    }
}

#[tauri::command]
async fn get_nodes_info(
    results: Vec<SlabIndex>,
    state: State<'_, SearchState>,
) -> Result<Vec<NodeInfo>, String> {
    if results.is_empty() {
        return Ok(Vec::new());
    }

    state
        .node_info_tx
        .send(results.clone())
        .map_err(|e| format!("Failed to send node info request: {e:?}"))?;

    let nodes = state
        .node_info_results_rx
        .recv()
        .map_err(|e| format!("Failed to receive node info results: {e:?}"))?;

    let node_infos = nodes
        .into_iter()
        .map(|SearchResultNode { path, metadata }| {
            let path = path.to_string_lossy().into_owned();
            // icon_of_path_ns is fast enough, we can synchronously get basic
            // icon here, and then get QuickLook icon later(with push
            // icon_update).
            let icon = fs_icon::icon_of_path_ns(&path).map(|data| {
                format!(
                    "data:image/png;base64,{}",
                    general_purpose::STANDARD.encode(data)
                )
            });
            NodeInfo {
                path,
                icon,
                metadata: metadata.as_ref().map(NodeInfoMetadata::from_metadata),
            }
        })
        .collect();

    Ok(node_infos)
}

#[tauri::command]
async fn update_icon_viewport(
    id: u64,
    viewport: Vec<SlabIndex>,
    state: State<'_, SearchState>,
) -> Result<(), String> {
    state
        .icon_viewport_tx
        .send((id, viewport))
        .map_err(|e| format!("Failed to send icon viewport update: {e:?}"))
}

#[tauri::command]
async fn get_app_status() -> Result<String, String> {
    Ok(load_app_state().as_str().to_string())
}

#[tauri::command]
async fn trigger_rescan(state: State<'_, SearchState>) -> Result<(), String> {
    state
        .rescan_tx
        .send(())
        .map_err(|e| format!("Failed to request rescan: {e:?}"))?;
    Ok(())
}

#[tauri::command]
fn open_in_finder(path: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let p = Path::new(&path);
        // On macOS, `open -R` reveals the file or directory in Finder.
        std::process::Command::new("open")
            .arg("-R")
            .arg(p)
            .spawn()
            .map_err(|e| format!("Failed to reveal path in Finder: {e}"))?;
    }
    #[cfg(target_os = "windows")]
    {
        let p = Path::new(&path);
        // On Windows, use explorer.exe /select to reveal the file.
        std::process::Command::new("explorer.exe")
            .arg("/select,")
            .arg(p)
            .spawn()
            .map_err(|e| format!("Failed to reveal path in Explorer: {}", e))?;
    }
    #[cfg(target_os = "linux")]
    {
        let p = Path::new(&path);
        // On Linux, use xdg-open to open the parent directory.
        if let Some(parent) = p.parent() {
            std::process::Command::new("xdg-open")
                .arg(parent)
                .spawn()
                .map_err(|e| format!("Failed to open parent directory: {}", e))?;
        } else {
            std::process::Command::new("xdg-open")
                .arg(p)
                .spawn()
                .map_err(|e| format!("Failed to open path: {}", e))?;
        }
    }
    Ok(())
}

#[cfg(desktop)]
fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    use tauri::{
        menu::{MenuBuilder, MenuItemBuilder},
        tray::{MouseButton, TrayIconBuilder, TrayIconEvent},
    };

    let handle = app.handle();

    let quit_item = MenuItemBuilder::with_id(TRAY_MENU_QUIT_ID, "Quit Cardinal")
        .accelerator("CmdOrCtrl+Q")
        .build(app)?;

    let menu = MenuBuilder::new(app).item(&quit_item).build()?;

    let mut tray_builder = TrayIconBuilder::new()
        .menu(&menu)
        .tooltip("Cardinal")
        .on_menu_event(|app, event| {
            if event.id.as_ref() == TRAY_MENU_QUIT_ID {
                EXIT_REQUESTED.store(true, Ordering::Relaxed);
                app.exit(0);
            }
        })
        .on_tray_icon_event(|tray, tray_event| {
            if let TrayIconEvent::Click { button, .. } = tray_event {
                if matches!(button, MouseButton::Left) {
                    ensure_main_window_visible(tray.app_handle());
                }
            }
        });

    let tray_icon_owned = handle
        .default_window_icon()
        .cloned()
        .map(|icon| icon.to_owned());
    if let Some(icon) = tray_icon_owned {
        tray_builder = tray_builder.icon(icon);
    }

    let tray_icon = tray_builder.build(app)?;
    let _ = Box::leak(Box::new(tray_icon));
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() -> Result<()> {
    // Initialize the tracing subscriber to print logs to the command line
    let builder = tracing_subscriber::fmt();
    if let Ok(filter) = EnvFilter::try_from_default_env() {
        builder.with_env_filter(filter).init();
    } else {
        builder.with_max_level(LevelFilter::INFO).init();
    }

    // Create communication channels
    let (finish_tx, finish_rx) = bounded::<Sender<Option<SearchCache>>>(1);
    let (search_tx, search_rx) = unbounded::<SearchJob>();
    let (result_tx, result_rx) = unbounded();
    let (node_info_tx, node_info_rx) = unbounded();
    let (node_info_results_tx, node_info_results_rx) = unbounded::<Vec<SearchResultNode>>();
    let (icon_viewport_tx, icon_viewport_rx) = unbounded::<(u64, Vec<SlabIndex>)>();
    let (rescan_tx, rescan_rx) = unbounded::<()>();
    let (icon_update_tx, icon_update_rx) = unbounded::<IconPayload>();
    let quick_launch_shortcut_plugin = tauri_plugin_global_shortcut::Builder::new()
        .with_shortcut(QUICK_LAUNCH_SHORTCUT)
        .expect("invalid quick launch shortcut definition")
        .with_handler(|app, _shortcut, event| {
            if event.state == ShortcutState::Pressed {
                toggle_main_window(app);
            }
        })
        .build();
    let mut builder = tauri::Builder::default();
    #[cfg(not(feature = "dev"))]
    {
        builder = builder.plugin(tauri_plugin_prevent_default::init());
    }
    builder = builder
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_macos_permissions::init())
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .plugin(quick_launch_shortcut_plugin)
        .on_window_event(|window, event| {
            if window.label() != "main" {
                return;
            }

            if let WindowEvent::CloseRequested { api, .. } = event {
                if EXIT_REQUESTED.load(Ordering::Relaxed) {
                    return;
                }

                api.prevent_close();
                let app_handle = window.app_handle();
                if hide_main_window(app_handle) {
                    info!("Main window hidden; Cardinal keeps running in the background");
                }
            }
        });
    // Run the Tauri application.
    let app = builder
        .manage(SearchState {
            search_tx,
            result_rx,
            node_info_tx,
            node_info_results_rx,
            icon_viewport_tx: icon_viewport_tx.clone(),
            rescan_tx: rescan_tx.clone(),
        })
        .invoke_handler(tauri::generate_handler![
            search,
            get_nodes_info,
            update_icon_viewport,
            get_app_status,
            trigger_rescan,
            open_in_finder
        ])
        .build(tauri::generate_context!())
        .expect("error while running tauri application");

    #[cfg(desktop)]
    setup_tray(&app).expect("failed to initialize system tray");

    let app_handle = &app.handle().to_owned();
    emit_app_state(app_handle);
    let icon_update_rx = &icon_update_rx;
    std::thread::scope(move |s| {
        // Init icon update thread
        s.spawn(|| {
            while let Ok(icon) = icon_update_rx.recv() {
                let mut icons = vec![icon];
                // Batch icon updates within 100ms
                std::thread::sleep(Duration::from_millis(100));
                icons.extend(icon_update_rx.try_iter());
                info!("emitting {} icons", icons.len());
                app_handle.emit("icon_update", icons).unwrap();
            }
            info!("icon update thread exited");
        });
        // Init background event processing thread
        s.spawn(move || {
            if !has_full_disk_access(app_handle) {
                info!("App does not have Full Disk Access, sleeping indefinitely");
                while !APP_QUIT.load(Ordering::Relaxed) {
                    std::thread::sleep(Duration::from_millis(100));
                }
                info!("Background thread quitting without Full Disk Access");
                return;
            }
            const WATCH_ROOT: &str = "/";
            const FSE_LATENCY_SECS: f64 = 0.1;
            let path = PathBuf::from(WATCH_ROOT);
            // Initialize the search cache.
            let mut cache = match SearchCache::try_read_persistent_cache(
                &path,
                &CACHE_PATH,
                Some(Path::new("/System/Volumes/Data")),
                Some(&APP_QUIT),
            ) {
                Ok(cached) => {
                    info!("Loaded existing cache");
                    // If using cache, defer the emit init process to HistoryDone event processing

                    // Emit the initial status-bar payload.
                    app_handle
                        .emit(
                            "status_bar_update",
                            StatusBarUpdate {
                                scanned_files: cached.get_total_files(),
                                processed_events: 0,
                            },
                        )
                        .unwrap();

                    cached
                }
                Err(e) => {
                    info!("Walking filesystem: {:?}", e);
                    let walk_data = WalkData::new(
                        Some(Path::new("/System/Volumes/Data")),
                        false,
                        Some(&APP_QUIT),
                    );
                    let walking_done = AtomicBool::new(false);
                    let cache = std::thread::scope(|s| {
                        s.spawn(|| {
                            while !walking_done.load(Ordering::Relaxed) {
                                let dirs = walk_data.num_dirs.load(Ordering::Relaxed);
                                let files = walk_data.num_files.load(Ordering::Relaxed);
                                let total = dirs + files;
                                app_handle
                                    .emit(
                                        "status_bar_update",
                                        StatusBarUpdate {
                                            scanned_files: total,
                                            processed_events: 0,
                                        },
                                    )
                                    .unwrap();
                                std::thread::sleep(Duration::from_millis(100));
                            }
                        });
                        let cache = SearchCache::walk_fs_with_walk_data(
                            path.clone(),
                            &walk_data,
                            Some(Path::new("/System/Volumes/Data")),
                            Some(&APP_QUIT),
                        );

                        walking_done.store(true, Ordering::Relaxed);

                        cache
                    });

                    let Some(cache) = cache else {
                        info!("Walk filesystem cancelled, app quitting");
                        finish_rx
                            .recv()
                            .expect("Failed to receive finish signal")
                            .send(None)
                            .expect("Failed to send None cache");
                        return;
                    };

                    // Emit the initial status-bar payload.
                    app_handle
                        .emit(
                            "status_bar_update",
                            StatusBarUpdate {
                                scanned_files: cache.get_total_files(),
                                processed_events: 0,
                            },
                        )
                        .unwrap();

                    // If full file system scan, emit initialized instantly.
                    update_app_state(app_handle, AppLifecycleState::Ready);
                    cache
                }
            };

            // Start the filesystem event watcher.
            let event_watcher = EventWatcher::spawn(
                WATCH_ROOT.to_string(),
                cache.last_event_id(),
                FSE_LATENCY_SECS,
            )
            .1;
            if !matches!(load_app_state(), AppLifecycleState::Ready) {
                update_app_state(app_handle, AppLifecycleState::Initializing);
            }
            info!("Started background processing thread");
            run_background_event_loop(
                app_handle,
                cache,
                event_watcher,
                finish_rx,
                search_rx,
                result_tx,
                node_info_rx,
                node_info_results_tx,
                icon_viewport_rx,
                rescan_rx,
                icon_update_tx,
                WATCH_ROOT,
                FSE_LATENCY_SECS,
            );

            info!("Background thread exited");
        });

        app.run(move |app_handle, event| {
            match &event {
                RunEvent::Exit => {
                    APP_QUIT.store(true, Ordering::Relaxed);
                    // Triggered when the tray context menu requests an exit.
                    // TODO(ldm0): Periodically save the cache so a close event within ~10s can skip another flush.

                    // Write cache to file before app exit
                    flush_cache_to_file_once(&finish_tx);
                }
                RunEvent::ExitRequested { api, code, .. } => {
                    let already_requested = EXIT_REQUESTED.swap(true, Ordering::Relaxed);
                    APP_QUIT.store(true, Ordering::Relaxed);
                    if !already_requested {
                        info!(
                            "Exit requested (code: {:?}); flushing cache before shutdown",
                            code
                        );
                    }

                    flush_cache_to_file_once(&finish_tx);

                    if code.is_none() {
                        api.prevent_exit();
                        app_handle.exit(0);
                    }
                }
                RunEvent::Reopen {
                    has_visible_windows,
                    ..
                } => {
                    if !has_visible_windows {
                        ensure_main_window_visible(app_handle);
                    }
                }
                _ => (),
            }
        });
    });
    Ok(())
}

/// Write cache to file before app exit
fn flush_cache_to_file_once(finish_tx: &Sender<Sender<Option<SearchCache>>>) {
    static FLUSH_ONCE: Once = Once::new();
    if load_app_state() == AppLifecycleState::Initializing {
        info!("App not fully initialized, skipping cache flush");
        return;
    }
    FLUSH_ONCE.call_once(move || {
        let (cache_tx, cache_rx) = bounded::<Option<SearchCache>>(1);
        finish_tx
            .send(cache_tx)
            .context("cache_tx is closed")
            .unwrap();
        if let Some(cache) = cache_rx.recv().context("cache_tx is closed").unwrap() {
            cache
                .flush_to_file(&CACHE_PATH)
                .context("Failed to write cache to file")
                .unwrap();

            info!("Cache flushed successfully to {:?}", &*CACHE_PATH);
        } else {
            info!("Canncelled during data construction, no cache to flush");
        }
    });
}

fn has_full_disk_access<R: Runtime>(app_handle: &AppHandle<R>) -> bool {
    // Reference: https://github.com/inket/FullDiskAccess/blob/846e04ea2b84fce843f47d7e7f3421189221829c/Sources/FullDiskAccess/FullDiskAccess.swift#L46
    let check_dirs = ["Library/Containers/com.apple.stocks", "Library/Safari"];

    if let Ok(home_dir) = app_handle.path().home_dir() {
        for check_dir in check_dirs.iter() {
            if std::fs::read_dir(home_dir.join(check_dir)).is_ok() {
                return true;
            }
        }
    }

    false
}

/*
fn request_full_disk_access() -> Result<(), String> {
    std::process::Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles")
        .output()
        .map_err(|error| error.to_string())?;
    Ok(())
}
 */
