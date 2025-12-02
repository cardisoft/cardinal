use crate::{
    LOGIC_START,
    lifecycle::load_app_state,
    quicklook::{
        QuickLookItemInput, close_preview_panel, toggle_preview_panel, update_preview_panel,
    },
    window_controls::{WindowToggle, activate_window, hide_window, toggle_window},
};
use anyhow::Result;
use base64::{Engine as _, engine::general_purpose};
use crossbeam_channel::{Receiver, Sender, bounded};
use parking_lot::Mutex;
use search_cache::{
    SearchOptions, SearchOutcome, SearchResultNode, SlabIndex, SlabNodeMetadata,
    SlabNodeMetadataCompact,
};
use search_cancel::CancellationToken;
use serde::{Deserialize, Serialize};
use std::{cmp::Ordering as StdOrdering, process::Command};
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

fn normalize_path(path: &std::path::Path) -> String {
    path.to_string_lossy().into_owned()
}

fn metadata_numeric(meta: &SlabNodeMetadataCompact, key: SortKeyPayload) -> i64 {
    let Some(meta_ref) = meta.as_ref() else {
        return i64::MIN;
    };
    match key {
        SortKeyPayload::Size => meta_ref.size() as i64,
        SortKeyPayload::Mtime => meta_ref
            .mtime()
            .map(|value| value.get() as i64)
            .unwrap_or(i64::MIN),
        SortKeyPayload::Ctime => meta_ref
            .ctime()
            .map(|value| value.get() as i64)
            .unwrap_or(i64::MIN),
        SortKeyPayload::FullPath | SortKeyPayload::Filename => 0,
    }
}

pub struct SearchState {
    search_tx: Sender<SearchJob>,
    result_rx: Receiver<Result<SearchOutcome>>,

    node_info_tx: Sender<NodeInfoRequest>,

    icon_viewport_tx: Sender<(u64, Vec<SlabIndex>)>,
    rescan_tx: Sender<()>,
    sorted_view_cache: Mutex<Option<SortedViewCache>>,
}

impl SearchState {
    pub fn new(
        search_tx: Sender<SearchJob>,
        result_rx: Receiver<Result<SearchOutcome>>,
        node_info_tx: Sender<NodeInfoRequest>,
        icon_viewport_tx: Sender<(u64, Vec<SlabIndex>)>,
        rescan_tx: Sender<()>,
    ) -> Self {
        Self {
            search_tx,
            result_rx,
            node_info_tx,
            icon_viewport_tx,
            rescan_tx,
            sorted_view_cache: Mutex::new(None),
        }
    }

    fn request_nodes(&self, slab_indices: Vec<SlabIndex>) -> Result<Vec<SearchResultNode>, String> {
        if slab_indices.is_empty() {
            return Ok(Vec::new());
        }

        let (response_tx, response_rx) = bounded::<Vec<SearchResultNode>>(1);
        self.node_info_tx
            .send(NodeInfoRequest {
                slab_indices,
                response_tx,
            })
            .map_err(|e| format!("Failed to send node info request: {e:?}"))?;

        response_rx
            .recv()
            .map_err(|e| format!("Failed to receive node info results: {e:?}"))
    }

    fn fetch_sorted_nodes(
        &self,
        slab_indices: &[SlabIndex],
    ) -> Result<Vec<SearchResultNode>, String> {
        if slab_indices.is_empty() {
            return Ok(Vec::new());
        }

        let mut cache_guard = self.sorted_view_cache.lock();
        if let Some(cached) = cache_guard
            .as_ref()
            .filter(|cache| cache.slab_indices == slab_indices)
            .map(|cache| cache.nodes.clone())
        {
            return Ok(cached);
        }

        let nodes = self.request_nodes(slab_indices.to_vec())?;
        *cache_guard = Some(SortedViewCache {
            slab_indices: slab_indices.to_vec(),
            nodes: nodes.clone(),
        });
        Ok(nodes)
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
pub fn update_quicklook(
    app_handle: AppHandle,
    items: Vec<QuickLookItemInput>,
) -> Result<(), String> {
    let app_handle_cloned = app_handle.clone();
    app_handle
        .run_on_main_thread(move || {
            update_preview_panel(app_handle_cloned, items);
        })
        .map_err(|e| format!("Failed to dispatch quicklook action: {e:?}"))?;
    Ok(())
}

#[tauri::command]
pub fn toggle_quicklook(
    app_handle: AppHandle,
    items: Vec<QuickLookItemInput>,
) -> Result<(), String> {
    let app_handle_cloned = app_handle.clone();
    app_handle
        .run_on_main_thread(move || {
            toggle_preview_panel(app_handle_cloned, items);
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
    include_icons: Option<bool>,
    state: State<'_, SearchState>,
) -> Result<Vec<NodeInfo>, String> {
    if results.is_empty() {
        return Ok(Vec::new());
    }

    let include_icons = include_icons.unwrap_or(true);
    let nodes = state.request_nodes(results)?;

    let node_infos = nodes
        .into_iter()
        .map(|SearchResultNode { path, metadata }| {
            let path = path.to_string_lossy().into_owned();
            let icon = if include_icons {
                fs_icon::icon_of_path_ns(&path).map(|data| {
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
        .collect();

    Ok(node_infos)
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SortKeyPayload {
    Filename,
    FullPath,
    Size,
    Mtime,
    Ctime,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortDirectionPayload {
    Asc,
    Desc,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SortStatePayload {
    pub key: SortKeyPayload,
    pub direction: SortDirectionPayload,
}

#[derive(Debug)]
struct SortEntry {
    original_index: usize,
    slab_index: SlabIndex,
    node: SearchResultNode,
    path_key: String,
    name_key: String,
}

fn extract_filename(node: &SearchResultNode) -> String {
    node.path
        .file_name()
        .and_then(|name| name.to_str())
        .map(|x| x.to_string())
        .unwrap_or_else(|| node.path.to_string_lossy().into_owned())
}

fn compare_entries(a: &SortEntry, b: &SortEntry, sort: &SortStatePayload) -> StdOrdering {
    let ordering = match sort.key {
        SortKeyPayload::FullPath => a.path_key.cmp(&b.path_key),
        SortKeyPayload::Filename => a.name_key.cmp(&b.name_key),
        SortKeyPayload::Size | SortKeyPayload::Mtime | SortKeyPayload::Ctime => {
            metadata_numeric(&a.node.metadata, sort.key)
                .cmp(&metadata_numeric(&b.node.metadata, sort.key))
        }
    };

    let ordering = ordering.then_with(|| a.original_index.cmp(&b.original_index));

    match sort.direction {
        SortDirectionPayload::Asc => ordering,
        SortDirectionPayload::Desc => ordering.reverse(),
    }
}

#[tauri::command]
pub async fn get_sorted_view(
    results: Vec<SlabIndex>,
    sort: Option<SortStatePayload>,
    state: State<'_, SearchState>,
) -> Result<Vec<SlabIndex>, String> {
    if results.is_empty() || sort.is_none() {
        return Ok(results);
    }

    let sort_state = sort.expect("checked above");
    let nodes = state.fetch_sorted_nodes(&results)?;
    let mut entries: Vec<SortEntry> = results
        .into_iter()
        .zip(nodes.into_iter())
        .enumerate()
        .map(|(idx, (slab_index, node))| SortEntry {
            original_index: idx,
            path_key: normalize_path(&node.path),
            name_key: extract_filename(&node),
            slab_index,
            node,
        })
        .collect();

    entries.sort_by(|a, b| compare_entries(a, b, &sort_state));

    Ok(entries.into_iter().map(|entry| entry.slab_index).collect())
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
