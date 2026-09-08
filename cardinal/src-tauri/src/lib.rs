mod background;
mod commands;
mod lifecycle;
mod quicklook;
mod search_activity;
mod sort;
mod window_controls;

use anyhow::Result;
use background::{
    BackgroundLoopChannels, IconPayload, build_search_cache, emit_status_bar_update,
    run_background_event_loop,
};
use cardinal_sdk::EventWatcher;
use commands::{
    LatestRequestSlot, MetadataSortRequest, NodeInfoRequest, SearchJob, SearchState,
    WatchConfigUpdate, activate_main_window, cancel_search, cancel_sort, close_quicklook,
    copy_files_to_clipboard, get_app_status, get_nodes_info, get_sorted_view, hide_main_window,
    move_to_trash, normalize_watch_config, open_in_finder, open_path, search, set_indexing_paused,
    set_tray_activation_policy, set_watch_config, start_logic, toggle_main_window,
    toggle_quicklook, trigger_rescan, update_icon_viewport, update_quicklook,
};
use crossbeam_channel::{Receiver, RecvTimeoutError, Sender, bounded, unbounded};
use lifecycle::{APP_QUIT, AppLifecycleState, EXIT_REQUESTED, emit_app_state, update_app_state};
use once_cell::sync::OnceCell;
use search_cache::{SearchCache, SlabIndex};
use search_cancel::CancellationToken;
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::Duration,
};
use tauri::{Emitter, Manager, RunEvent, WindowEvent};
use tracing::{info, level_filters::LevelFilter, warn};
use tracing_subscriber::EnvFilter;
use window_controls::{activate_window, hide_window};

static DB_PATH: OnceCell<PathBuf> = OnceCell::new();
pub(crate) static LOGIC_START: OnceCell<Sender<LogicStartConfig>> = OnceCell::new();
pub(crate) const DEFAULT_SYSTEM_IGNORE_PATH: &str = "/System/Volumes/Data";
const FSE_LATENCY_SECS: f64 = 0.1;

#[derive(Debug, Clone)]
pub(crate) struct LogicStartConfig {
    pub watch_root: String,
    pub ignore_paths: Vec<String>,
    pub include_paths: Vec<String>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() -> Result<()> {
    let builder = tracing_subscriber::fmt();
    if let Ok(filter) = EnvFilter::try_from_default_env() {
        builder.with_env_filter(filter).init();
    } else {
        builder.with_max_level(LevelFilter::INFO).init();
    }

    let (finish_tx, finish_rx) = bounded::<Sender<Option<SearchCache>>>(1);
    let (search_tx, search_rx) = unbounded::<SearchJob>();
    let (node_info_tx, node_info_rx) = unbounded::<NodeInfoRequest>();
    // Bound full-index sort requests so repeated UI clicks cannot retain unlimited
    // result vectors while the background cache thread is busy.
    let (metadata_sort_tx, metadata_sort_rx) = bounded::<()>(1);
    let metadata_sort_pending = Arc::new(LatestRequestSlot::<MetadataSortRequest>::default());
    let metadata_sort_generation = Arc::new(AtomicU64::new(0));
    let (icon_viewport_tx, icon_viewport_rx) = unbounded::<(u64, Vec<SlabIndex>)>();
    let (rescan_tx, rescan_rx) = unbounded::<CancellationToken>();
    let (watch_config_tx, watch_config_rx) = unbounded::<WatchConfigUpdate>();
    let (indexing_control_tx, indexing_control_rx) = unbounded::<bool>();
    let indexing_paused = Arc::new(AtomicBool::new(false));
    let rebuild_required = Arc::new(AtomicBool::new(false));
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
            metadata_sort_tx,
            metadata_sort_pending.clone(),
            icon_viewport_tx.clone(),
            rescan_tx.clone(),
            watch_config_tx.clone(),
            indexing_control_tx,
            indexing_paused.clone(),
            metadata_sort_generation,
            update_window_state_tx.clone(),
        ))
        .invoke_handler(tauri::generate_handler![
            search,
            get_nodes_info,
            get_sorted_view,
            update_icon_viewport,
            get_app_status,
            trigger_rescan,
            set_indexing_paused,
            cancel_search,
            cancel_sort,
            set_watch_config,
            open_in_finder,
            open_path,
            move_to_trash,
            toggle_quicklook,
            close_quicklook,
            update_quicklook,
            start_logic,
            hide_main_window,
            activate_main_window,
            toggle_main_window,
            set_tray_activation_policy,
            copy_files_to_clipboard,
        ])
        .build(tauri::generate_context!())
        .expect("error while running tauri application");

    let db_path = DB_PATH
        .get_or_try_init(|| app.path().app_config_dir().map(|p| p.join("cardinal.db")))
        .expect("Failed to initialize database path");

    let app_handle = app.handle().to_owned();
    let channels = BackgroundLoopChannels {
        finish_rx,
        search_rx,
        node_info_rx,
        metadata_sort_rx,
        metadata_sort_pending,
        icon_viewport_rx,
        rescan_rx,
        watch_config_rx,
        indexing_control_rx,
        indexing_paused,
        rebuild_required,
        icon_update_tx,
        update_window_state_rx,
    };
    emit_app_state(&app_handle);
    let icon_app_handle = app_handle.clone();
    spawn_detached(move || {
        while let Ok(icon) = icon_update_rx.recv() {
            let mut icons = vec![icon];
            std::thread::sleep(Duration::from_millis(100));
            icons.extend(icon_update_rx.try_iter());
            info!("emitting {} icons", icons.len());
            icon_app_handle.emit("icon_update", icons).unwrap();
        }
        info!("icon update thread exited");
    });

    let logic_app_handle = app_handle.clone();
    let logic_db_path = db_path.clone();
    spawn_detached(move || {
        let Some(config) = wait_for_logic_start(logic_start_rx) else {
            info!("Background thread quitting without Full Disk Access");
            return;
        };

        run_logic_thread(&logic_app_handle, &logic_db_path, channels, config);
    });

    app.run(move |app_handle, event| match event {
        RunEvent::Exit => {
            APP_QUIT.store(true, Ordering::Relaxed);
            if !EXIT_REQUESTED.swap(true, Ordering::Relaxed) {
                request_background_shutdown(&finish_tx);
            }
        }
        RunEvent::ExitRequested { code, .. } => {
            let already_requested = EXIT_REQUESTED.swap(true, Ordering::Relaxed);
            APP_QUIT.store(true, Ordering::Relaxed);
            if !already_requested {
                info!(
                    "Exit requested (code: {:?}); stopping background work",
                    code
                );
                request_background_shutdown(&finish_tx);
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

    Ok(())
}

fn run_logic_thread(
    app_handle: &tauri::AppHandle,
    db_path: &Path,
    channels: BackgroundLoopChannels,
    config: LogicStartConfig,
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
    let path = PathBuf::from(&watch_root);
    let ignore_paths: Vec<_> = ignore_paths.into_iter().map(PathBuf::from).collect();
    let include_paths: Vec<_> = include_paths.into_iter().map(PathBuf::from).collect();

    let mut cache = match SearchCache::try_read_persistent_cache(
        &path,
        db_path,
        &ignore_paths,
        &include_paths,
        &APP_QUIT,
    ) {
        Ok(cached) => {
            info!("Loaded existing cache");
            emit_status_bar_update(app_handle, cached.get_total_files(), 0, 0);
            cached
        }
        Err(e) => {
            info!("Walking filesystem: {:?}", e);
            if let Some(cache) = build_search_cache(
                app_handle,
                &watch_root,
                &ignore_paths,
                &include_paths,
                CancellationToken::new_scan(),
            ) {
                emit_status_bar_update(app_handle, cache.get_total_files(), 0, 0);
                cache
            } else if APP_QUIT.load(Ordering::Relaxed) {
                info!("Walk filesystem cancelled, app quitting");
                channels
                    .finish_rx
                    .recv()
                    .expect("Failed to receive finish signal")
                    .send(None)
                    .expect("Failed to send None cache");
                return;
            } else {
                info!("Initial scan cancelled by newer request, use noop cache");
                channels.rebuild_required.store(true, Ordering::SeqCst);
                SearchCache::noop(
                    path.clone(),
                    ignore_paths.clone(),
                    include_paths.clone(),
                    &APP_QUIT,
                )
            }
        }
    };

    let event_watcher = if cache.is_noop() || channels.indexing_paused.load(Ordering::SeqCst) {
        info!("Using noop event watcher due to cancelled initial scan");
        EventWatcher::noop()
    } else {
        update_app_state(app_handle, AppLifecycleState::Updating);
        EventWatcher::spawn(
            watch_root.to_string(),
            cache.last_event_id(),
            FSE_LATENCY_SECS,
            cache.ignore_paths(),
            cache.include_paths(),
        )
        .1
    };

    info!("Started background processing thread");
    // TODO(ldm0): remove this watch_root, use cache's path instead
    run_background_event_loop(
        app_handle,
        cache,
        event_watcher,
        channels,
        watch_root.to_string(),
        FSE_LATENCY_SECS,
        db_path.to_path_buf(),
    );

    info!("Background thread exited");
}

fn request_background_shutdown(finish_tx: &Sender<Sender<Option<SearchCache>>>) {
    let (cache_tx, cache_rx) = bounded::<Option<SearchCache>>(1);
    drop(cache_rx);
    if let Err(error) = finish_tx.try_send(cache_tx) {
        info!("Background shutdown signal was already queued or unavailable: {error:?}");
    }
}

fn spawn_detached<F>(task: F)
where
    F: FnOnce() + Send + 'static,
{
    std::thread::spawn(task);
}

fn wait_for_logic_start(rx: Receiver<LogicStartConfig>) -> Option<LogicStartConfig> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shutdown_request_does_not_wait_for_cache_flush() {
        let (finish_tx, finish_rx) = bounded(1);

        request_background_shutdown(&finish_tx);

        let cache_tx = finish_rx
            .try_recv()
            .expect("shutdown signal should be queued");
        assert!(
            cache_tx.send(None).is_err(),
            "quit path must not retain a receiver and wait for a cache snapshot"
        );
    }

    #[test]
    fn detached_background_worker_does_not_block_the_caller() {
        let (started_tx, started_rx) = bounded(1);
        let (_release_tx, release_rx) = bounded::<()>(1);

        spawn_detached(move || {
            started_tx.send(()).expect("worker should start");
            let _ = release_rx.recv();
        });

        started_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("detached worker should run independently");
    }
}
