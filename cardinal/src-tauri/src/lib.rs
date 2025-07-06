// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use anyhow::{Context, Result};
use cardinal_sdk::{EventFlag, EventWatcher};
use crossbeam_channel::{bounded, unbounded, Receiver, Sender};
use search_cache::{HandleFSEError, SearchCache, SearchNode};
use std::{cell::LazyCell, path::PathBuf};
use tauri::{Emitter, RunEvent, State};
use tracing::{info, level_filters::LevelFilter};
use tracing_subscriber::EnvFilter;

struct SearchState {
    search_tx: Sender<String>,
    result_rx: Receiver<Result<Vec<SearchNode>>>,
}

#[tauri::command]
async fn search(query: &str, state: State<'_, SearchState>) -> Result<Vec<String>, String> {
    // 发送搜索请求到后台线程
    state
        .search_tx
        .send(query.to_string())
        .map_err(|e| format!("Failed to send search request: {e}"))?;

    // 等待搜索结果
    let search_result = state
        .result_rx
        .recv()
        .map_err(|e| format!("Failed to receive search result: {e}"))?;

    // 处理搜索结果
    search_result
        .map(|nodes| {
            nodes
                .into_iter()
                .map(|n| n.path.to_string_lossy().into_owned())
                .collect()
        })
        .map_err(|e| e.to_string())
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
    let (finish_tx, finish_rx) = bounded::<Sender<SearchCache>>(1);
    let (search_tx, search_rx) = unbounded::<String>();
    let (result_tx, result_rx) = unbounded::<Result<Vec<SearchNode>>>();
    // 运行Tauri应用
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(SearchState {
            search_tx,
            result_rx,
        })
        .invoke_handler(tauri::generate_handler![search])
        .build(tauri::generate_context!())
        .expect("error while running tauri application");

    let app_handle = app.handle().to_owned();
    // 启动后台处理线程
    std::thread::spawn(move || {
        // 初始化搜索缓存
        let path = PathBuf::from("/");
        let mut processed_events = 0;
        let emit_init = {
            let app_handle_clone = app_handle.clone();
            LazyCell::new(move || app_handle_clone.emit("init_completed", ()).unwrap())
        };
        let mut cache = if let Ok(cached) = SearchCache::try_read_persistent_cache(&path) {
            info!("Loaded existing cache");
            // If using cache, delay the emit init process to HistoryDone event processing
            cached
        } else {
            info!("Walking filesystem...");
            let cache = SearchCache::walk_fs(path.clone());
            // If full file system scan, emit initialized instantly.
            *emit_init;
            cache
        };

        // 启动事件监听器
        let mut event_watcher = EventWatcher::spawn("/".to_string(), cache.last_event_id(), 0.1);
        info!("Started background processing thread");

        loop {
            crossbeam_channel::select! {
                recv(finish_rx) -> tx => {
                    let tx = tx.expect("Finish channel closed");
                    tx.send(cache).expect("Failed to send cache");
                    break;
                }
                recv(search_rx) -> query => {
                    let query = query.expect("Search channel closed");
                    let result = cache.query_files(query);
                    result_tx.send(result).expect("Failed to send result");
                }
                recv(event_watcher.receiver) -> events => {
                    let events = events.expect("Event stream closed");
                    processed_events += events.len();
                    app_handle.emit("status_update", format!("Processing {} events...", processed_events)).unwrap();
                    // Emit HistoryDone inform frontend that cache is ready.
                    if events.iter().any(|x| x.flag == EventFlag::HistoryDone) {
                        *emit_init;
                    }
                    if let Err(HandleFSEError::Rescan) = cache.handle_fs_events(events) {
                        info!("!!!!!!!!!! Rescan triggered !!!!!!!!");
                        // Here we clear event_watcher first as rescan may take a lot of time
                        event_watcher.clear();
                        cache.rescan();
                        event_watcher = EventWatcher::spawn("/".to_string(), cache.last_event_id(), 0.1);
                    }
                }
            }
        }
        info!("Background thread exited");
    });

    app.run(move |app_handle, event| {
        match &event {
            RunEvent::ExitRequested { api, code, .. } => {
                // Keep the event loop running even if all windows are closed
                // This allow us to catch tray icon events when there is no window
                // if we manually requested an exit (code is Some(_)) we will let it go through
                if code.is_none() {
                    info!("Tauri application exited, flushing cache...");

                    // TODO(ldm0): is this necessary?
                    api.prevent_exit();

                    // TODO(ldm0): change the tray icon to "saving"

                    let (cache_tx, cache_rx) = bounded::<SearchCache>(1);
                    finish_tx
                        .send(cache_tx)
                        .context("cache_tx is closed")
                        .unwrap();
                    let cache = cache_rx.recv().context("cache_tx is closed").unwrap();
                    cache
                        .flush_to_file()
                        .context("Failed to write cache to file")
                        .unwrap();

                    info!("Cache flushed successfully");

                    app_handle.exit(0);
                }
            }
            _ => (),
        }
    });
    Ok(())
}
