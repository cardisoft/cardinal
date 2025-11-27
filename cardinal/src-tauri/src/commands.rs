use crate::{
    LOGIC_START,
    lifecycle::{EXIT_REQUESTED, load_app_state},
    quicklook::{close_preview_panel, open_preview_panel, update_preview_panel},
    window_controls::{WindowToggle, activate_window, hide_window, toggle_window},
};
use anyhow::Result;
use base64::{Engine as _, engine::general_purpose};
use crossbeam_channel::{Receiver, Sender};
use search_cache::{SearchOptions, SearchOutcome, SearchResultNode, SlabIndex, SlabNodeMetadata};
use search_cancel::CancellationToken;
use serde::{Deserialize, Serialize};
use std::{process::Command, sync::atomic::Ordering};
use tauri::{AppHandle, Manager, State};
use tracing::{info, warn};

#[derive(Debug, Clone, Copy, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SearchOptionsPayload {
    #[serde(default)]
    pub case_insensitive: bool,
}

impl From<SearchOptionsPayload> for SearchOptions {
    fn from(SearchOptionsPayload { case_insensitive }: SearchOptionsPayload) -> Self {
        SearchOptions { case_insensitive }
    }
}

#[derive(Debug, Clone)]
pub struct SearchJob {
    pub query: String,
    pub options: SearchOptionsPayload,
    pub cancellation_token: CancellationToken,
}

pub struct SearchState {
    search_tx: Sender<SearchJob>,
    result_rx: Receiver<Result<SearchOutcome>>,

    node_info_tx: Sender<Vec<SlabIndex>>,
    node_info_results_rx: Receiver<Vec<SearchResultNode>>,

    icon_viewport_tx: Sender<(u64, Vec<SlabIndex>)>,
    rescan_tx: Sender<()>,
}

impl SearchState {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        search_tx: Sender<SearchJob>,
        result_rx: Receiver<Result<SearchOutcome>>,
        node_info_tx: Sender<Vec<SlabIndex>>,
        node_info_results_rx: Receiver<Vec<SearchResultNode>>,
        icon_viewport_tx: Sender<(u64, Vec<SlabIndex>)>,
        rescan_tx: Sender<()>,
    ) -> Self {
        Self {
            search_tx,
            result_rx,
            node_info_tx,
            node_info_results_rx,
            icon_viewport_tx,
            rescan_tx,
        }
    }
}

#[derive(Serialize)]
pub struct NodeInfo {
    pub path: String,
    pub metadata: Option<NodeInfoMetadata>,
    pub icon: Option<String>,
}

#[derive(Serialize)]
pub struct SearchResponse {
    pub results: Vec<SlabIndex>,
    pub highlights: Vec<String>,
}

#[derive(Serialize)]
pub struct NodeInfoMetadata {
    pub r#type: u8,
    pub size: u64,
    pub ctime: u32,
    pub mtime: u32,
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

#[tauri::command]
pub fn close_quicklook(app_handle: AppHandle) -> Result<(), String> {
    let app_handle_cloned = app_handle.clone();
    app_handle
        .run_on_main_thread(move || {
            close_preview_panel(app_handle_cloned);
        })
        .map_err(|e| format!("Failed to dispatch quicklook action: {e:?}"))?;
    Ok(())
}

#[tauri::command]
pub fn update_quicklook(app_handle: AppHandle, paths: Vec<String>) -> Result<(), String> {
    app_handle
        .run_on_main_thread(move || {
            update_preview_panel(paths);
        })
        .map_err(|e| format!("Failed to dispatch quicklook action: {e:?}"))?;
    Ok(())
}

#[tauri::command]
pub fn open_quicklook(app_handle: AppHandle, paths: Vec<String>) -> Result<(), String> {
    app_handle
        .run_on_main_thread(move || {
            open_preview_panel(paths);
        })
        .map_err(|e| format!("Failed to dispatch quicklook action: {e:?}"))?;
    Ok(())
}

#[tauri::command]
pub async fn search(
    query: String,
    options: Option<SearchOptionsPayload>,
    version: u64,
    state: State<'_, SearchState>,
) -> Result<SearchResponse, String> {
    let options = options.unwrap_or_default();
    let cancellation_token = CancellationToken::new(version);
    state
        .search_tx
        .send(SearchJob {
            query,
            options,
            cancellation_token,
        })
        .map_err(|e| format!("Failed to send search request: {e:?}"))?;

    let search_result = state
        .result_rx
        .recv()
        .map_err(|e| format!("Failed to receive search result: {e:?}"))?
        .map(|res| {
            let SearchOutcome { nodes, highlights } = res;
            let results = match nodes {
                Some(list) => list,
                None => {
                    info!("Search {version} was cancelled");
                    Vec::new()
                }
            };
            SearchResponse {
                results,
                highlights,
            }
        });

    search_result.map_err(|e| format!("Failed to process search result: {e:?}"))
}

#[tauri::command]
pub async fn get_nodes_info(
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
pub async fn update_icon_viewport(
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
pub async fn get_app_status() -> Result<String, String> {
    Ok(load_app_state().as_str().to_string())
}

#[tauri::command]
pub async fn trigger_rescan(state: State<'_, SearchState>) -> Result<(), String> {
    state
        .rescan_tx
        .send(())
        .map_err(|e| format!("Failed to request rescan: {e:?}"))?;
    Ok(())
}

#[tauri::command]
pub fn open_in_finder(path: String) -> Result<(), String> {
    Command::new("open")
        .arg("-R")
        .arg(&path)
        .spawn()
        .map_err(|e| format!("Failed to reveal path in Finder: {e}"))?;
    Ok(())
}

#[tauri::command]
pub fn open_path(path: String) -> Result<(), String> {
    Command::new("open")
        .arg(&path)
        .spawn()
        .map_err(|e| format!("Failed to open path: {e}"))?;
    Ok(())
}

#[tauri::command]
pub fn request_app_exit(app_handle: AppHandle) -> Result<(), String> {
    EXIT_REQUESTED.store(true, Ordering::Relaxed);
    app_handle.exit(0);
    Ok(())
}

#[tauri::command]
pub fn start_logic() {
    if let Some(sender) = LOGIC_START.get() {
        let _ = sender.send(());
    }
}

#[tauri::command]
pub fn hide_main_window(app: AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        if hide_window(&window) {
            info!("Main window hidden via command");
        }
    }
}

#[tauri::command]
pub fn activate_main_window(app: AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        activate_window(&window);
        info!("Main window activated via command");
    } else {
        warn!("Activate requested but main window is unavailable");
    }
}

#[tauri::command]
pub fn toggle_main_window(app: AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        if matches!(toggle_window(&window), WindowToggle::Hidden) {
            info!("Main window hidden via command");
        } else {
            info!("Main window shown via command");
        }
    } else {
        warn!("Toggle requested but main window is unavailable");
    }
}
