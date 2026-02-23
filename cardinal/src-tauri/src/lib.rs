mod background;
mod commands;
mod lifecycle;
mod quicklook;
mod search_activity;
mod sort;
mod window_controls;

use anyhow::{Context, Result};
use background::{
    BackgroundLoopChannels, IconPayload, build_search_cache, emit_status_bar_update,
    run_background_event_loop,
};
use cardinal_sdk::EventWatcher;
use commands::{
    NodeInfoRequest, SearchJob, SearchState, WatchConfigUpdate, activate_main_window,
    close_quicklook, copy_files_to_clipboard, get_app_status, get_nodes_info, get_sorted_view,
    hide_main_window, normalize_watch_config, open_in_finder, open_path, search,
    set_tray_activation_policy, set_watch_config, start_logic, toggle_main_window,
    toggle_quicklook, trigger_rescan, update_icon_viewport, update_quicklook,
};
use crossbeam_channel::{Receiver, RecvTimeoutError, Sender, bounded, unbounded};
use lifecycle::{
    APP_QUIT, AppLifecycleState, EXIT_REQUESTED, emit_app_state, load_app_state, update_app_state,
};
use once_cell::sync::OnceCell;
use search_cache::{SearchCache, SearchOutcome, SlabIndex};
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
pub(crate) const DEFAULT_SYSTEM_IGNORE_PATH: &str = "/System/Volumes/Data";
const FSE_LATENCY_SECS: f64 = 0.1;

#[derive(Debug, Clone)]
pub(crate) struct LogicStartConfig {
    pub watch_root: String,
    pub ignore_paths: Vec<String>,
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
    let (result_tx, result_rx) = unbounded::<Result<SearchOutcome>>();
    let (node_info_tx, node_info_rx) = unbounded::<NodeInfoRequest>();
    let (icon_viewport_tx, icon_viewport_rx) = unbounded::<(u64, Vec<SlabIndex>)>();
    let (rescan_tx, rescan_rx) = unbounded::<CancellationToken>();
    let (watch_config_tx, watch_config_rx) = unbounded::<WatchConfigUpdate>();
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
            result_rx,
            node_info_tx,
            icon_viewport_tx.clone(),
            rescan_tx.clone(),
            watch_config_tx.clone(),
            update_window_state_tx.clone(),
        ))
        .invoke_handler(tauri::generate_handler![
            search,
            get_nodes_info,
            get_sorted_view,
            update_icon_viewport,
            get_app_status,
            trigger_rescan,
            set_watch_config,
            open_in_finder,
            open_path,
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

    let app_handle = &app.handle().to_owned();
    let channels = BackgroundLoopChannels {
        finish_rx,
        search_rx,
        result_tx,
        node_info_rx,
        icon_viewport_rx,
        rescan_rx,
        watch_config_rx,
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

            run_logic_thread(app_handle, db_path, channels, config);
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
) {
    let Some((watch_root, ignore_paths)) =
        normalize_watch_config(&config.watch_root, config.ignore_paths, Some("/"))
    else {
        warn!("Invalid watch root in start config; skipping background startup");
        return;
    };
    let path = PathBuf::from(&watch_root);
    let ignore_paths: Vec<_> = ignore_paths.into_iter().map(PathBuf::from).collect();

    let mut cache = match SearchCache::try_read_persistent_cache(
        &path,
        db_path,
        &ignore_paths,
        Some(&APP_QUIT),
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
                SearchCache::noop(path.clone(), ignore_paths.clone(), &APP_QUIT)
            }
        }
    };

    let event_watcher = if cache.is_noop() {
        info!("Using noop event watcher due to cancelled initial scan");
        EventWatcher::noop()
    } else {
        EventWatcher::spawn(
            watch_root.to_string(),
            cache.last_event_id(),
            FSE_LATENCY_SECS,
        )
        .1
    };

    if load_app_state() != AppLifecycleState::Ready {
        update_app_state(app_handle, AppLifecycleState::Updating);
    }
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

fn flush_cache_to_file_once(finish_tx: &Sender<Sender<Option<SearchCache>>>, db_path: &PathBuf) {
    static FLUSH_ONCE: Once = Once::new();
    if load_app_state() != AppLifecycleState::Ready {
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
    info!("Waiting for Full Disk Access signal from the frontend");
    loop {
        if APP_QUIT.load(Ordering::Relaxed) {
            return None;
        }

        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(config) => {
                info!(
                    "Received Full Disk Access grant, starting background processing (watch_root={}, ignore_paths={})",
                    config.watch_root,
                    config.ignore_paths.len()
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
