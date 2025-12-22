use crate::{
    LOGIC_START,
    lifecycle::load_app_state,
    quicklook::{
        QuickLookItemInput, close_preview_panel, toggle_preview_panel, update_preview_panel,
    },
    search_activity,
    sort::{SortEntry, SortStatePayload, sort_entries},
    window_controls::{WindowToggle, activate_window, hide_window, toggle_window},
};
use anyhow::Result;
use base64::{Engine as _, engine::general_purpose};
use crossbeam_channel::{Receiver, Sender, bounded};
use parking_lot::Mutex;
use search_cache::{SearchOptions, SearchOutcome, SearchResultNode, SlabIndex, SlabNodeMetadata};
use search_cancel::CancellationToken;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf, process::Command};
use tauri::{AppHandle, Manager, State};
use tracing::{error, info, warn};

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

#[derive(Debug, Clone)]
pub struct NodeInfoRequest {
    pub slab_indices: Vec<SlabIndex>,
    pub response_tx: Sender<Vec<SearchResultNode>>,
}

#[derive(Default)]
struct SortedViewCache {
    slab_indices: Vec<SlabIndex>,
    nodes: Vec<SearchResultNode>,
}

pub struct SearchState {
    search_tx: Sender<SearchJob>,
    result_rx: Receiver<Result<SearchOutcome>>,

    node_info_tx: Sender<NodeInfoRequest>,

    icon_viewport_tx: Sender<(u64, Vec<SlabIndex>)>,
    rescan_tx: Sender<()>,
    sorted_view_cache: Mutex<Option<SortedViewCache>>,
    update_window_state_tx: Sender<()>,
}

impl SearchState {
    pub fn new(
        search_tx: Sender<SearchJob>,
        result_rx: Receiver<Result<SearchOutcome>>,
        node_info_tx: Sender<NodeInfoRequest>,
        icon_viewport_tx: Sender<(u64, Vec<SlabIndex>)>,
        rescan_tx: Sender<()>,
        update_window_state_tx: Sender<()>,
    ) -> Self {
        Self {
            search_tx,
            result_rx,
            node_info_tx,
            icon_viewport_tx,
            rescan_tx,
            sorted_view_cache: Mutex::new(None),
            update_window_state_tx,
        }
    }

    fn request_nodes(&self, slab_indices: Vec<SlabIndex>) -> Vec<SearchResultNode> {
        if slab_indices.is_empty() {
            return Vec::new();
        }

        let (response_tx, response_rx) = bounded::<Vec<SearchResultNode>>(1);
        if let Err(e) = self.node_info_tx.send(NodeInfoRequest {
            slab_indices,
            response_tx,
        }) {
            error!("Failed to send node info request: {e:?}");
            return Vec::new();
        }

        response_rx.recv().unwrap_or_else(|e| {
            error!("Failed to receive node info results: {e:?}");
            Vec::new()
        })
    }

    fn fetch_sorted_nodes(&self, slab_indices: &[SlabIndex]) -> Vec<SearchResultNode> {
        if slab_indices.is_empty() {
            return Vec::new();
        }

        let mut cache_guard = self.sorted_view_cache.lock();
        if let Some(cached) = cache_guard
            .as_ref()
            .filter(|cache| cache.slab_indices == slab_indices)
            .map(|cache| cache.nodes.clone())
        {
            return cached;
        }

        let nodes = self.request_nodes(slab_indices.to_vec());
        *cache_guard = Some(SortedViewCache {
            slab_indices: slab_indices.to_vec(),
            nodes: nodes.clone(),
        });
        nodes
    }
}

#[derive(Serialize)]
pub struct NodeInfo {
    pub path: String,
    pub metadata: Option<NodeInfoMetadata>,
    pub icon: Option<String>,
}

#[derive(Serialize, Default)]
pub struct SearchResponse {
    pub results: Vec<SlabIndex>,
    pub highlights: Vec<String>,
}

#[derive(Serialize)]
pub struct NodeInfoMetadata {
    pub r#type: u8,
    pub size: i64,
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
pub async fn close_quicklook(app_handle: AppHandle) {
    let app_handle_cloned = app_handle.clone();
    if let Err(e) = app_handle.run_on_main_thread(move || {
        close_preview_panel(app_handle_cloned);
    }) {
        error!("Failed to dispatch quicklook action: {e:?}");
    }
}

#[tauri::command]
pub async fn update_quicklook(app_handle: AppHandle, items: Vec<QuickLookItemInput>) {
    let app_handle_cloned = app_handle.clone();
    if let Err(e) = app_handle.run_on_main_thread(move || {
        update_preview_panel(app_handle_cloned, items);
    }) {
        error!("Failed to dispatch quicklook action: {e:?}");
    }
}

#[tauri::command]
pub async fn toggle_quicklook(app_handle: AppHandle, items: Vec<QuickLookItemInput>) {
    let app_handle_cloned = app_handle.clone();
    if let Err(e) = app_handle.run_on_main_thread(move || {
        toggle_preview_panel(app_handle_cloned, items);
    }) {
        error!("Failed to dispatch quicklook action: {e:?}");
    }
}

#[tauri::command]
pub async fn search(
    query: String,
    options: Option<SearchOptionsPayload>,
    version: u64,
    state: State<'_, SearchState>,
) -> Result<SearchResponse, String> {
    search_activity::note_search_activity();

    let options = options.unwrap_or_default();
    let cancellation_token = CancellationToken::new(version);
    if let Err(e) = state.search_tx.send(SearchJob {
        query,
        options,
        cancellation_token,
    }) {
        error!("Failed to send search request: {e:?}");
        return Ok(SearchResponse::default());
    }

    match state.result_rx.recv() {
        Ok(res) => res,
        Err(e) => {
            error!("Failed to receive search result: {e:?}");
            return Ok(SearchResponse::default());
        }
    }
    .map(|SearchOutcome { nodes, highlights }| {
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
    })
    .map_err(|e| format!("Failed to process search result: {e:?}"))
}

#[tauri::command(async)]
pub fn get_nodes_info(
    results: Vec<SlabIndex>,
    include_icons: Option<bool>,
    state: State<'_, SearchState>,
) -> Vec<NodeInfo> {
    if results.is_empty() {
        return Vec::new();
    }

    let include_icons = include_icons.unwrap_or(true);
    let nodes = state.request_nodes(results);

    nodes
        .into_iter()
        .map(|SearchResultNode { path, metadata }| {
            let path = path.to_string_lossy().into_owned();
            let icon = if include_icons {
                fs_icon::icon_of_path_ns(&path, 512.0).map(|data| {
                    format!(
                        "data:image/png;base64,{}",
                        general_purpose::STANDARD.encode(data)
                    )
                })
            } else {
                None
            };
            NodeInfo {
                path,
                icon,
                metadata: metadata.as_ref().map(NodeInfoMetadata::from_metadata),
            }
        })
        .collect()
}

#[tauri::command(async)]
pub fn get_sorted_view(
    results: Vec<SlabIndex>,
    sort: Option<SortStatePayload>,
    state: State<'_, SearchState>,
) -> Vec<SlabIndex> {
    if results.is_empty() || sort.is_none() {
        return results;
    }

    let sort_state = sort.expect("checked above");
    let nodes = state.fetch_sorted_nodes(&results);
    let mut entries: Vec<SortEntry> = results
        .into_iter()
        .zip(nodes)
        .map(|(slab_index, node)| SortEntry::new(slab_index, node))
        .collect();

    sort_entries(&mut entries, &sort_state);

    entries.into_iter().map(|entry| entry.slab_index).collect()
}

#[tauri::command(async)]
pub fn update_icon_viewport(id: u64, viewport: Vec<SlabIndex>, state: State<'_, SearchState>) {
    if let Err(e) = state.icon_viewport_tx.send((id, viewport)) {
        error!("Failed to send icon viewport update: {e:?}");
    }
}

#[tauri::command]
pub async fn get_app_status() -> String {
    load_app_state().as_str().to_string()
}

#[tauri::command(async)]
pub fn trigger_rescan(state: State<'_, SearchState>) {
    if let Err(e) = state.rescan_tx.send(()) {
        error!("Failed to request rescan: {e:?}");
    }
}

#[tauri::command]
pub async fn open_in_finder(path: String) {
    if let Err(e) = Command::new("open").arg("-R").arg(&path).spawn() {
        error!("Failed to reveal path in Finder: {e}");
    }
}

#[tauri::command]
pub async fn open_path(path: String) {
    if let Err(e) = Command::new("open").arg(&path).spawn() {
        error!("Failed to open path: {e}");
    }
}

#[tauri::command]
pub async fn start_logic() {
    if let Some(sender) = LOGIC_START.get() {
        let _ = sender.send(());
    }
}

#[tauri::command]
pub async fn hide_main_window(app: AppHandle) {
    if let Some(window) = app.get_webview_window("main")
        && hide_window(&window)
    {
        info!("Main window hidden via command");
        if let Some(state) = app.try_state::<SearchState>() {
            let _ = state.update_window_state_tx.try_send(());
        }
    }
}

#[tauri::command]
pub async fn activate_main_window(app: AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        activate_window(&window);
        info!("Main window activated via command");
        if let Some(state) = app.try_state::<SearchState>() {
            let _ = state.update_window_state_tx.try_send(());
        }
    } else {
        warn!("Activate requested but main window is unavailable");
    }
}

#[tauri::command]
pub async fn toggle_main_window(app: AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        if matches!(toggle_window(&window), WindowToggle::Hidden) {
            info!("Main window hidden via command");
        } else {
            info!("Main window shown via command");
        }
        if let Some(state) = app.try_state::<SearchState>() {
            let _ = state.update_window_state_tx.try_send(());
        }
    } else {
        warn!("Toggle requested but main window is unavailable");
    }
}
#[tauri::command]
pub async fn trash_paths(paths: Vec<String>) -> Result<(), String> {
    let path_bufs: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    trash::delete_all(path_bufs).map_err(|e| {
        error!("Failed to trash paths: {e}");
        e.to_string()
    })
}

#[tauri::command]
pub async fn delete_paths(paths: Vec<String>) -> Result<(), String> {
    for path in paths {
        let p = PathBuf::from(path);
        if p.is_dir() {
            if let Err(e) = fs::remove_dir_all(&p) {
                error!("Failed to delete directory {p:?}: {e}");
                return Err(e.to_string());
            }
        } else if let Err(e) = fs::remove_file(&p) {
            error!("Failed to delete file {p:?}: {e}");
            return Err(e.to_string());
        }
    }
    Ok(())
}
