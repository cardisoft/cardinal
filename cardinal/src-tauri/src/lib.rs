mod background;
mod bookmarks_window;
mod commands;
mod lifecycle;
mod quicklook;
mod search_activity;
mod sort;
mod window_controls;

use anyhow::{Context, Result};
use background::{
    BackgroundLoopChannels, IconPayload, IconViewportUpdate, IndexBuildResult, IndexingState,
    ScopedIndexResult, run_background_event_loop, spawn_index_load,
};
use bookmarks_window::{hide_bookmarks_window, show_bookmarks_window, toggle_bookmarks_window};
use cardinal_sdk::EventWatcher;
use commands::{
    NodeInfoRequest, RecentNodesRequest, SearchJob, SearchState, WatchConfigUpdate,
    activate_main_window, choose_app_for_path, close_quicklook, copy_files_to_clipboard,
    get_app_status, get_nodes_info, get_recent_nodes, get_sorted_view, hide_main_window,
    normalize_watch_config, open_in_finder, open_path, open_with_app, open_with_options,
    read_text_file, search, set_tray_activation_policy, set_watch_config, start_logic,
    toggle_main_window, toggle_quicklook, trigger_rescan, update_icon_viewport, update_quicklook,
    write_text_file,
};
use crossbeam_channel::{Receiver, RecvTimeoutError, Sender, bounded, unbounded};
use lifecycle::{APP_QUIT, AppLifecycleState, EXIT_REQUESTED, emit_app_state, load_app_state};
use once_cell::sync::OnceCell;
use search_cache::SearchCache;
use search_cancel::CancellationToken;
use std::{
    path::{Path, PathBuf},
    sync::{Once, atomic::Ordering},
    time::Duration,
};
use tauri::{Emitter, Manager, RunEvent, WindowEvent};
use tracing::{info, level_filters::LevelFilter, warn};
use tracing_subscriber::EnvFilter;
use window_controls::{activate_window, hide_window};

static DB_PATH: OnceCell<PathBuf> = OnceCell::new();
pub(crate) static LOGIC_START: OnceCell<Sender<LogicStartConfig>> = OnceCell::new();
/// Holds a start request that arrived before `LOGIC_START` was initialised.
///
/// Tauri commands can be invoked as soon as the webview loads, which can happen while the
/// `Builder` is still running. Dropping that request used to deadlock startup: the frontend
/// latched "logic started" and never retried, while the background thread waited forever.
static PENDING_LOGIC_START: parking_lot::Mutex<Option<LogicStartConfig>> =
    parking_lot::Mutex::new(None);
pub(crate) const DEFAULT_SYSTEM_IGNORE_PATH: &str = "/System/Volumes/Data";
const FSE_LATENCY_SECS: f64 = 0.1;

#[derive(Debug, Clone)]
pub(crate) struct LogicStartConfig {
    pub watch_root: String,
    pub ignore_paths: Vec<String>,
    pub include_paths: Vec<String>,
}

/// Writes panic reports next to the app's logs.
///
/// A bundle launched from Finder has no stderr, so a panic in the background thread used to
/// leave nothing behind: the app just sat on "Updating" and every search failed with
/// `RecvError`. Persisting the message + location (and a backtrace when `RUST_BACKTRACE` is
/// set) makes the next occurrence diagnosable.
fn install_panic_logger() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let thread = std::thread::current();
        let name = thread.name().unwrap_or("<unnamed>");
        let location = info
            .location()
            .map(|loc| format!("{}:{}:{}", loc.file(), loc.line(), loc.column()))
            .unwrap_or_else(|| "<unknown>".to_string());
        let payload = info.payload();
        let message = payload
            .downcast_ref::<&str>()
            .map(|text| (*text).to_string())
            .or_else(|| payload.downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "<non-string panic payload>".to_string());

        let report = format!(
            "[{}] thread '{}' panicked at {location}: {message}\n",
            chrono_like_timestamp(),
            name
        );
        if let Some(path) = panic_log_path() {
            if let Some(dir) = path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
            {
                use std::io::Write as _;
                let _ = file.write_all(report.as_bytes());
                let _ = file.flush();
            }
        }

        previous(info);
    }));
}

fn chrono_like_timestamp() -> String {
    // `SystemTime` -> "YYYY-MM-DD HH:MM:SS" without pulling in a date crate.
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let (days, secs) = (now / 86_400, now % 86_400);
    let (mut year, mut remaining) = (1970u32, days as u32);
    loop {
        let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
        let length = if leap { 366 } else { 365 };
        if remaining < length {
            break;
        }
        remaining -= length;
        year += 1;
    }
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let month_days = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut month = 1;
    for (index, length) in month_days.iter().enumerate() {
        if remaining < *length {
            month = index as u32 + 1;
            break;
        }
        remaining -= *length;
    }
    format!(
        "{year:04}-{month:02}-{:02} {:02}:{:02}:{:02}",
        remaining + 1,
        secs / 3600,
        (secs % 3600) / 60,
        secs % 60
    )
}

fn panic_log_path() -> Option<std::path::PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(
        std::path::PathBuf::from(home)
            .join("Library/Logs/Cardinal")
            .join("panic.log"),
    )
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() -> Result<()> {
    let builder = tracing_subscriber::fmt();
    if let Ok(filter) = EnvFilter::try_from_default_env() {
        builder.with_env_filter(filter).init();
    } else {
        builder.with_max_level(LevelFilter::INFO).init();
    }

    install_panic_logger();

    let (finish_tx, finish_rx) = bounded::<Sender<Option<SearchCache>>>(1);
    let (search_tx, search_rx) = unbounded::<SearchJob>();
    let (node_info_tx, node_info_rx) = unbounded::<NodeInfoRequest>();
    let (icon_viewport_tx, icon_viewport_rx) = unbounded::<IconViewportUpdate>();
    let (recent_nodes_tx, recent_nodes_rx) = unbounded::<RecentNodesRequest>();
    let (rescan_tx, rescan_rx) = unbounded::<CancellationToken>();
    let (watch_config_tx, watch_config_rx) = unbounded::<WatchConfigUpdate>();
    let (index_build_tx, index_build_rx) = unbounded::<IndexBuildResult>();
    let (scoped_index_tx, scoped_index_rx) = unbounded::<ScopedIndexResult>();
    let (icon_update_tx, icon_update_rx) = unbounded::<IconPayload>();
    let (update_window_state_tx, update_window_state_rx) = bounded::<()>(1);
    let (logic_start_tx, logic_start_rx) = bounded(1);
    LOGIC_START
        .set(logic_start_tx)
        .expect("LOGIC_START channel already initialized");

    let mut builder = tauri::Builder::default();
    #[cfg(not(feature = "dev"))]
    {
        builder = builder.plugin(tauri_plugin_prevent_default::init());
    }
    let update_window_state_tx_for_window = update_window_state_tx.clone();
    builder = builder
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_drag::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_macos_permissions::init())
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .on_window_event(move |window, event| {
            if window.label() != "main" {
                return;
            }

            match event {
                WindowEvent::Focused(_) => {
                    let _ = update_window_state_tx_for_window.try_send(());
                }
                WindowEvent::CloseRequested { api, .. } => {
                    if EXIT_REQUESTED.load(Ordering::Relaxed) {
                        return;
                    }

                    api.prevent_close();

                    let Some(window) = window.get_webview_window("main") else {
                        warn!("Close requested but main window is unavailable");
                        return;
                    };

                    if hide_window(&window) {
                        // The bookmarks panel is its own window; never leave it floating
                        // over other apps after the main window is gone.
                        if let Some(bookmarks) = window
                            .app_handle()
                            .get_webview_window(bookmarks_window::BOOKMARKS_WINDOW_LABEL)
                        {
                            let _ = bookmarks.hide();
                        }
                        let _ = update_window_state_tx_for_window.try_send(());
                        info!("Main window hidden; Cardinal keeps running in the background");
                    }
                }
                _ => {}
            }
        });

    let app = builder
        .manage(SearchState::new(
            search_tx,
            node_info_tx,
            icon_viewport_tx.clone(),
            recent_nodes_tx.clone(),
            rescan_tx.clone(),
            watch_config_tx.clone(),
            update_window_state_tx.clone(),
        ))
        // Set from the start, not only before each show: that way any path that orders the
        // window front from a hidden state lands the user on the Desktop they are looking at.
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                window_controls::prefer_active_space(&window);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            search,
            get_nodes_info,
            get_recent_nodes,
            get_sorted_view,
            update_icon_viewport,
            get_app_status,
            trigger_rescan,
            set_watch_config,
            open_in_finder,
            open_path,
            open_with_options,
            open_with_app,
            choose_app_for_path,
            write_text_file,
            read_text_file,
            toggle_quicklook,
            close_quicklook,
            update_quicklook,
            start_logic,
            hide_main_window,
            activate_main_window,
            toggle_main_window,
            set_tray_activation_policy,
            copy_files_to_clipboard,
            show_bookmarks_window,
            hide_bookmarks_window,
            toggle_bookmarks_window,
        ])
        .build(tauri::generate_context!())
        .expect("error while running tauri application");

    let db_path = DB_PATH
        .get_or_try_init(|| app.path().app_config_dir().map(|p| p.join("cardinal.db")))
        .expect("Failed to initialize database path");

    let app_handle = &app.handle().to_owned();
    let channels = BackgroundLoopChannels {
        finish_rx,
        search_rx,
        node_info_rx,
        icon_viewport_rx,
        recent_nodes_rx,
        rescan_rx,
        watch_config_rx,
        index_build_rx,
        index_build_tx: index_build_tx.clone(),
        scoped_index_rx,
        scoped_index_tx,
        icon_update_tx,
        update_window_state_rx,
    };
    emit_app_state(app_handle);
    let icon_update_rx = &icon_update_rx;
    std::thread::scope(move |s| {
        s.spawn(|| {
            while let Ok(icon) = icon_update_rx.recv() {
                let mut icons = vec![icon];
                std::thread::sleep(Duration::from_millis(100));
                icons.extend(icon_update_rx.try_iter());
                info!("emitting {} icons", icons.len());
                app_handle.emit("icon_update", icons).unwrap();
            }
            info!("icon update thread exited");
        });

        let logic_start_rx = logic_start_rx;
        s.spawn(move || {
            let Some(config) = wait_for_logic_start(logic_start_rx) else {
                info!("Background thread quitting without Full Disk Access");
                return;
            };

            run_logic_thread(app_handle, db_path, channels, config, index_build_tx);
        });

        app.run(move |app_handle, event| match event {
            RunEvent::Exit => {
                APP_QUIT.store(true, Ordering::Relaxed);
                flush_cache_to_file_once(&finish_tx, db_path);
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

                flush_cache_to_file_once(&finish_tx, db_path);

                if code.is_none() {
                    api.prevent_exit();
                    app_handle.exit(0);
                }
            }
            RunEvent::Reopen { .. } => {
                // On macOS, clicking the Dock icon should bring the main window back even if the
                // app still "has windows" but they are hidden.
                if let Some(window) = app_handle.get_webview_window("main") {
                    activate_window(&window);
                } else {
                    warn!("Reopen requested but main window is unavailable");
                }
            }
            _ => {}
        });
    });

    Ok(())
}

fn run_logic_thread(
    app_handle: &tauri::AppHandle,
    db_path: &Path,
    channels: BackgroundLoopChannels,
    config: LogicStartConfig,
    index_build_tx: Sender<IndexBuildResult>,
) {
    let Some((watch_root, ignore_paths, include_paths)) = normalize_watch_config(
        &config.watch_root,
        config.ignore_paths,
        config.include_paths,
        Some("/"),
    ) else {
        warn!("Invalid watch root in start config; skipping background startup");
        return;
    };
    let ignore_paths: Vec<_> = ignore_paths.into_iter().map(PathBuf::from).collect();
    let include_paths: Vec<_> = include_paths.into_iter().map(PathBuf::from).collect();

    // Cache loading and filesystem walking happen on a worker so search requests can be
    // serviced immediately. A literal folder scope can even be searched during the initial
    // walk via a temporary index of just that folder.
    let scan_cancellation_token = CancellationToken::new_scan();
    spawn_index_load(
        app_handle.clone(),
        watch_root.clone(),
        ignore_paths.clone(),
        include_paths.clone(),
        db_path.to_path_buf(),
        scan_cancellation_token,
        false,
        index_build_tx,
    );

    let cache = SearchCache::noop(
        PathBuf::from(&watch_root),
        ignore_paths,
        include_paths,
        &APP_QUIT,
    );
    let event_watcher = EventWatcher::noop();

    info!("Started background processing thread; initial index building asynchronously");
    // TODO(ldm0): remove this watch_root, use cache's path instead
    run_background_event_loop(
        app_handle,
        cache,
        event_watcher,
        channels,
        watch_root.to_string(),
        FSE_LATENCY_SECS,
        db_path.to_path_buf(),
        Some(IndexingState {
            cancellation_token: scan_cancellation_token,
        }),
    );

    info!("Background thread exited");
}

fn flush_cache_to_file_once(finish_tx: &Sender<Sender<Option<SearchCache>>>, db_path: &PathBuf) {
    static FLUSH_ONCE: Once = Once::new();
    // `Updating` means a complete cache exists and the watcher is replaying history; that is
    // good enough to persist. Only `Initializing` (walk/rebuild in flight) is skipped.
    if load_app_state() == AppLifecycleState::Initializing {
        info!("App is still initializing, skipping cache flush");
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
                .flush_to_file(db_path)
                .context("Failed to write cache to file")
                .unwrap();

            info!("Cache flushed successfully to {:?}", db_path);
        } else {
            info!("Cancelled before data constructed, no cache to flush");
        }
    });
}

fn wait_for_logic_start(rx: Receiver<LogicStartConfig>) -> Option<LogicStartConfig> {
    // A request may have landed before the sender existed; honour it first.
    if let Some(pending) = PENDING_LOGIC_START.lock().take() {
        info!("Using Full Disk Access grant that arrived early");
        return Some(pending);
    }

    info!("Waiting for Full Disk Access signal from the frontend");
    loop {
        if APP_QUIT.load(Ordering::Relaxed) {
            return None;
        }

        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(config) => {
                info!(
                    "Received Full Disk Access grant, starting background processing (watch_root={}, ignore_paths={:?}, include_paths={:?})",
                    config.watch_root, config.ignore_paths, config.include_paths
                );
                return Some(config);
            }
            Err(RecvTimeoutError::Timeout) => continue,
            Err(RecvTimeoutError::Disconnected) => {
                warn!("Full Disk Access channel disconnected before grant");
                return None;
            }
        }
    }
}
