use crate::{
    DEFAULT_SYSTEM_IGNORE_PATH, LOGIC_START, LogicStartConfig,
    lifecycle::load_app_state,
    quicklook::{
        QuickLookItemInput, close_preview_panel, toggle_preview_panel, update_preview_panel,
    },
    search_activity,
    sort::{SortEntry, SortStatePayload, sort_entries},
    window_controls::{activate_main_window_impl, hide_main_window_impl, toggle_main_window_impl},
};
use anyhow::{Result, anyhow};
use base64::{Engine as _, engine::general_purpose};
use camino::{Utf8Path as Path, Utf8PathBuf as PathBuf};
use crossbeam_channel::{Sender, TrySendError, bounded};
use objc2::{
    rc::{Retained, autoreleasepool},
    runtime::ProtocolObject,
};
use objc2_app_kit::{NSPasteboard, NSPasteboardItem, NSPasteboardTypeString, NSPasteboardWriting};
use objc2_foundation::{NSArray, NSString, NSURL};
use parking_lot::Mutex;
use search_cache::{
    SearchOptions, SearchOutcome, SearchQuery, SearchResultNode, SlabIndex, SlabNodeMetadata,
};
use search_cancel::CancellationToken;
use serde::{Deserialize, Serialize};
use std::{
    cell::LazyCell,
    process::Command,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
};
use tauri::{ActivationPolicy, AppHandle, State};
use tracing::{error, info, warn};

#[derive(Debug, Clone)]
pub struct WatchConfigUpdate {
    pub watch_root: String,
    pub ignore_paths: Vec<String>,
    pub include_paths: Vec<String>,
    pub scan_cancellation_token: CancellationToken,
}

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
    pub query: SearchQuery,
    pub options: SearchOptionsPayload,
    pub cancellation_token: CancellationToken,
    pub result_tx: Sender<Result<SearchOutcome>>,
}

#[derive(Debug, Clone)]
pub struct NodeInfoRequest {
    pub slab_indices: Vec<SlabIndex>,
    pub response_tx: Sender<Vec<SearchResultNode>>,
}

#[derive(Debug)]
pub struct MetadataSortRequest {
    pub results: Vec<SlabIndex>,
    pub sort: SortStatePayload,
    pub generation: u64,
    pub active_generation: Arc<AtomicU64>,
    pub response_tx: Sender<Vec<SlabIndex>>,
}

pub(crate) struct LatestRequestSlot<T> {
    pending: Mutex<Option<T>>,
}

impl<T> Default for LatestRequestSlot<T> {
    fn default() -> Self {
        Self {
            pending: Mutex::new(None),
        }
    }
}

impl<T> LatestRequestSlot<T> {
    pub(crate) fn replace(&self, request: T) -> Option<T> {
        self.pending.lock().replace(request)
    }

    pub(crate) fn take(&self) -> Option<T> {
        self.pending.lock().take()
    }
}

#[derive(Default)]
struct SortedViewCache {
    slab_indices: Vec<SlabIndex>,
    nodes: Vec<SearchResultNode>,
}

pub struct SearchState {
    search_tx: Sender<SearchJob>,
    node_info_tx: Sender<NodeInfoRequest>,
    metadata_sort_tx: Sender<()>,
    metadata_sort_pending: Arc<LatestRequestSlot<MetadataSortRequest>>,
    icon_viewport_tx: Sender<(u64, Vec<SlabIndex>)>,
    rescan_tx: Sender<CancellationToken>,
    watch_config_tx: Sender<WatchConfigUpdate>,
    indexing_control_tx: Sender<bool>,
    indexing_paused: Arc<AtomicBool>,
    metadata_sort_generation: Arc<AtomicU64>,
    sorted_view_cache: Mutex<Option<SortedViewCache>>,
    pub(crate) update_window_state_tx: Sender<()>,
}

impl SearchState {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        search_tx: Sender<SearchJob>,
        node_info_tx: Sender<NodeInfoRequest>,
        metadata_sort_tx: Sender<()>,
        metadata_sort_pending: Arc<LatestRequestSlot<MetadataSortRequest>>,
        icon_viewport_tx: Sender<(u64, Vec<SlabIndex>)>,
        rescan_tx: Sender<CancellationToken>,
        watch_config_tx: Sender<WatchConfigUpdate>,
        indexing_control_tx: Sender<bool>,
        indexing_paused: Arc<AtomicBool>,
        metadata_sort_generation: Arc<AtomicU64>,
        update_window_state_tx: Sender<()>,
    ) -> Self {
        Self {
            search_tx,
            node_info_tx,
            metadata_sort_tx,
            metadata_sort_pending,
            icon_viewport_tx,
            rescan_tx,
            watch_config_tx,
            indexing_control_tx,
            indexing_paused,
            metadata_sort_generation,
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

    fn request_metadata_sort(
        &self,
        results: Vec<SlabIndex>,
        sort: SortStatePayload,
    ) -> Vec<SlabIndex> {
        let fallback = results.clone();
        let (response_tx, response_rx) = bounded::<Vec<SlabIndex>>(1);
        let generation = self.metadata_sort_generation.fetch_add(1, Ordering::SeqCst) + 1;
        let request = MetadataSortRequest {
            results,
            sort,
            generation,
            active_generation: self.metadata_sort_generation.clone(),
            response_tx,
        };
        if let Some(superseded) = self.metadata_sort_pending.replace(request) {
            let _ = superseded.response_tx.send(Vec::new());
        }
        match self.metadata_sort_tx.try_send(()) {
            Ok(()) | Err(TrySendError::Full(())) => {}
            Err(TrySendError::Disconnected(())) => {
                error!("Failed to notify background metadata sorter");
                return fallback;
            }
        }

        response_rx.recv().unwrap_or_else(|error| {
            error!("Failed to receive metadata sort result: {error:?}");
            fallback
        })
    }

    fn cancel_metadata_sort(&self) {
        self.metadata_sort_generation.fetch_add(1, Ordering::SeqCst);
        if let Some(pending) = self.metadata_sort_pending.take() {
            let _ = pending.response_tx.send(Vec::new());
        }
    }
}

/// Normalizes user-provided path input into an absolute path string.
///
/// Expands a leading `~` component using the current `HOME` directory and rejects
/// non-absolute paths (including relative paths and unsupported `~user` forms).
/// Returns `Some` absolute path string when valid, otherwise `None`.
fn normalize_path_input(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    let path = Path::new(trimmed);
    let mut expanded = PathBuf::new();
    let home = LazyCell::new(|| {
        std::env::var_os("HOME").and_then(|h| h.to_string_lossy().into_owned().into())
    });

    for (index, component) in path.into_iter().enumerate() {
        if index == 0 && component == "~" {
            expanded.push(home.as_deref()?);
        } else {
            expanded.push(component);
        }
    }

    let resolved = expanded.into_string();
    if resolved.starts_with('/') {
        Some(resolved)
    } else {
        None
    }
}

pub(crate) fn normalize_watch_config(
    watch_root: &str,
    ignore_paths: Vec<String>,
    include_paths: Vec<String>,
    fallback_watch_root: Option<&str>,
) -> Option<(String, Vec<String>, Vec<String>)> {
    let watch_root = normalize_path_input(watch_root)
        .or_else(|| fallback_watch_root.and_then(normalize_path_input))?;
    let mut ignore_paths = ignore_paths
        .into_iter()
        .filter_map(|path| {
            let normalized = normalize_path_input(&path);
            if normalized.is_none() {
                warn!("Ignoring invalid ignore path: {path:?}");
            }
            normalized
        })
        .collect::<Vec<_>>();
    if !ignore_paths
        .iter()
        .any(|path| path == DEFAULT_SYSTEM_IGNORE_PATH)
    {
        ignore_paths.push(DEFAULT_SYSTEM_IGNORE_PATH.to_string());
    }
    let include_paths = include_paths
        .into_iter()
        .filter_map(|path| {
            let normalized = normalize_path_input(&path);
            if normalized.is_none() {
                warn!("Ignoring invalid include path: {path:?}");
            }
            normalized
        })
        .collect::<Vec<_>>();
    Some((watch_root, ignore_paths, include_paths))
}

#[derive(Serialize)]
pub struct NodeInfo {
    pub path: String,
    pub metadata: Option<NodeInfoMetadata>,
    pub icon: Option<String>,
}

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SearchResponse {
    pub results: Vec<SlabIndex>,
    pub highlights: Vec<String>,
    pub status_code: u8,
}

impl SearchResponse {
    pub const OK: u8 = 0;
    pub const CANCELLED: u8 = 1;
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
    directory_query: Option<String>,
    query: Option<String>,
    options: Option<SearchOptionsPayload>,
    state: State<'_, SearchState>,
) -> Result<SearchResponse, String> {
    search_activity::note_search_activity();

    let options = options.unwrap_or_default();
    let cancellation_token = CancellationToken::new_search();
    let (result_tx, result_rx) = bounded(1);
    if let Err(e) = state.search_tx.send(SearchJob {
        query: SearchQuery {
            directory_query,
            query,
        },
        options,
        cancellation_token,
        result_tx,
    }) {
        error!("Failed to send search request: {e:?}");
        return Err(format!("Failed to send search request: {e:?}"));
    }

    match result_rx.recv() {
        Ok(res) => res,
        Err(e) => {
            error!("Failed to receive search result: {e:?}");
            return Err(format!("Failed to receive search result: {e:?}"));
        }
    }
    .map(|SearchOutcome { nodes, highlights }| {
        let (status_code, results) = match nodes {
            Some(list) => (SearchResponse::OK, list),
            None => {
                let version = cancellation_token.version();
                info!("Search {version} was cancelled");
                (SearchResponse::CANCELLED, vec![])
            }
        };
        SearchResponse {
            results,
            highlights,
            status_code,
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
    if sort_state.key.uses_metadata() {
        return state.request_metadata_sort(results, sort_state);
    }
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
    if let Err(e) = state.rescan_tx.send(CancellationToken::new_scan()) {
        error!("Failed to request rescan: {e:?}");
    }
}

#[tauri::command]
pub fn set_indexing_paused(paused: bool, state: State<'_, SearchState>) -> Result<(), String> {
    state.indexing_paused.store(paused, Ordering::SeqCst);
    if paused {
        let _ = CancellationToken::new_scan();
    }
    state
        .indexing_control_tx
        .send(paused)
        .map_err(|error| format!("Failed to update indexing state: {error}"))
}

#[tauri::command]
pub fn cancel_sort(state: State<'_, SearchState>) {
    state.cancel_metadata_sort();
}

#[tauri::command]
pub fn cancel_search() {
    let _ = CancellationToken::new_search();
}

#[tauri::command(async)]
pub fn set_watch_config(
    watch_root: String,
    ignore_paths: Vec<String>,
    include_paths: Option<Vec<String>>,
    state: State<'_, SearchState>,
) {
    let Some((watch_root, ignore_paths, include_paths)) = normalize_watch_config(
        &watch_root,
        ignore_paths,
        include_paths.unwrap_or_default(),
        None,
    ) else {
        warn!("Ignoring invalid watch_root: {watch_root:?}");
        return;
    };

    if let Err(e) = state.watch_config_tx.send(WatchConfigUpdate {
        watch_root,
        ignore_paths,
        include_paths,
        scan_cancellation_token: CancellationToken::new_scan(),
    }) {
        error!("Failed to request watch config change: {e:?}");
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

fn validate_trash_paths(paths: &[String]) -> Result<Vec<PathBuf>, String> {
    if paths.is_empty() || paths.len() > 1_000 {
        return Err("Trash request must contain between 1 and 1000 paths".to_string());
    }

    let mut validated = Vec::with_capacity(paths.len());
    for input in paths {
        if input.is_empty() || input != input.trim() {
            return Err(
                "Trash paths cannot be empty or contain surrounding whitespace".to_string(),
            );
        }
        let path = PathBuf::from(input);
        let has_alias_component = input
            .split(['/', '\\'])
            .any(|component| component == "." || component == "..");
        if !path.is_absolute() || path.parent().is_none() || has_alias_component {
            return Err("Only absolute, non-root paths can be moved to Trash".to_string());
        }
        let home = std::env::var("HOME").map_err(|_| "Home directory is unavailable")?;
        let metadata = std::fs::symlink_metadata(path.as_std_path()).ok();
        if !metadata.as_ref().is_some_and(std::fs::Metadata::is_symlink) {
            let canonical_home =
                std::fs::canonicalize(&home).map_err(|_| "Home directory could not be verified")?;
            if same_file::is_same_file(path.as_std_path(), &home).unwrap_or(false) {
                return Err("The home directory cannot be moved to Trash".to_string());
            }
            if let Ok(canonical_target) = std::fs::canonicalize(path.as_std_path())
                && (canonical_target == canonical_home || canonical_target.parent().is_none())
            {
                return Err("Home and filesystem roots cannot be moved to Trash".to_string());
            }
        }
        if !validated.contains(&path) {
            validated.push(path);
        }
    }

    validated.sort_unstable_by(|left, right| {
        left.components()
            .count()
            .cmp(&right.components().count())
            .then_with(|| left.cmp(right))
    });
    let mut top_level = Vec::with_capacity(validated.len());
    for path in validated {
        if top_level
            .iter()
            .any(|parent: &PathBuf| path.starts_with(parent))
        {
            continue;
        }
        top_level.push(path);
    }
    Ok(top_level)
}

#[tauri::command]
pub async fn move_to_trash(paths: Vec<String>) -> Result<(), String> {
    let validated = validate_trash_paths(&paths)?;
    tauri::async_runtime::spawn_blocking(move || trash::delete_all(validated))
        .await
        .map_err(|error| format!("Trash operation failed: {error}"))?
        .map_err(|error| format!("Could not move item to Trash: {error}"))
}

#[tauri::command]
pub async fn start_logic(
    watch_root: String,
    ignore_paths: Vec<String>,
    include_paths: Option<Vec<String>>,
) {
    if let Some(sender) = LOGIC_START.get() {
        let _ = sender.try_send(LogicStartConfig {
            watch_root,
            ignore_paths,
            include_paths: include_paths.unwrap_or_default(),
        });
    }
}

#[tauri::command]
pub async fn hide_main_window(app: AppHandle) {
    hide_main_window_impl(&app);
}

#[tauri::command]
pub async fn activate_main_window(app: AppHandle) {
    activate_main_window_impl(&app);
}

#[tauri::command]
pub async fn toggle_main_window(app: AppHandle) {
    toggle_main_window_impl(&app);
}

#[tauri::command]
pub async fn set_tray_activation_policy(app: AppHandle, enabled: bool) {
    let app_handle = app.clone();
    if let Err(e) = app.run_on_main_thread(move || {
        let policy = if enabled {
            ActivationPolicy::Accessory
        } else {
            ActivationPolicy::Regular
        };
        if let Err(e) = app_handle.set_activation_policy(policy) {
            error!("Failed to set activation policy: {e:?}");
        }
        activate_main_window_impl(&app_handle);
    }) {
        error!("Failed to dispatch activation policy update: {e:?}");
    }
}

#[tauri::command]
pub async fn copy_files_to_clipboard(paths: Vec<String>) {
    if paths.is_empty() {
        return;
    }

    if let Err(err) = copy_files_to_clipboard_impl(paths) {
        error!("Failed to copy files to clipboard: {err:?}");
    }
}

fn copy_files_to_clipboard_impl(paths: Vec<String>) -> Result<()> {
    autoreleasepool(|_| unsafe {
        let pasteboard = NSPasteboard::generalPasteboard();
        pasteboard.clearContents();

        let urls: Vec<Retained<NSURL>> = paths
            .iter()
            .map(|path| NSURL::fileURLWithPath(&NSString::from_str(path)))
            .collect();

        let path_strings = NSPasteboardItem::new();
        {
            let ns_path = NSString::from_str(&paths.join(" "));
            path_strings.setString_forType(&ns_path, NSPasteboardTypeString);
        }

        let objects: Vec<&ProtocolObject<dyn NSPasteboardWriting>> = urls
            .iter()
            .map(|url| ProtocolObject::from_ref(&**url))
            .chain(Some(ProtocolObject::from_ref(&*path_strings)))
            .collect();
        let array = NSArray::from_slice(&objects);
        if pasteboard.writeObjects(&array) {
            Ok(())
        } else {
            Err(anyhow!("NSPasteboard.writeObjects failed"))
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_sort_pending_slot_retains_only_the_latest_request() {
        let slot = LatestRequestSlot::default();

        assert_eq!(slot.replace(1), None);
        assert_eq!(slot.replace(2), Some(1));
        assert_eq!(slot.take(), Some(2));
        assert_eq!(slot.take(), None);
    }

    #[test]
    fn normalize_rejects_empty_input() {
        assert_eq!(normalize_path_input(""), None);
        assert_eq!(normalize_path_input("   "), None);
    }

    #[test]
    fn normalize_accepts_absolute_paths() {
        assert_eq!(normalize_path_input("/"), Some("/".to_string()));
        assert_eq!(
            normalize_path_input(" /var/log "),
            Some("/var/log".to_string())
        );
    }

    #[test]
    fn normalize_expands_tilde_when_home_available() {
        let Ok(home) = std::env::var("HOME") else {
            return;
        };
        assert_eq!(normalize_path_input("~"), Some(home.clone()));
        assert_eq!(
            normalize_path_input("~/Documents"),
            Some(format!("{home}/Documents"))
        );
    }

    #[test]
    fn normalize_rejects_relative_paths_and_tilde_users() {
        assert_eq!(normalize_path_input("relative/path"), None);
        assert_eq!(normalize_path_input("./relative"), None);
        assert_eq!(normalize_path_input("~someone"), None);
        assert_eq!(normalize_path_input("~someone/Documents"), None);
    }

    #[test]
    fn trash_paths_require_safe_absolute_non_root_targets() {
        assert!(validate_trash_paths(&[]).is_err());
        assert!(validate_trash_paths(&["relative".to_string()]).is_err());
        assert!(validate_trash_paths(&["/".to_string()]).is_err());
        assert!(validate_trash_paths(&["/Users/example/..".to_string()]).is_err());
        assert!(validate_trash_paths(&["/tmp/./report.pdf".to_string()]).is_err());
        assert!(validate_trash_paths(&[" /tmp/report.pdf ".to_string()]).is_err());
        if let Ok(home) = std::env::var("HOME") {
            let case_alias = home.replacen("/Users/", "/users/", 1);
            if case_alias != home && std::path::Path::new(&case_alias).exists() {
                assert!(validate_trash_paths(&[case_alias]).is_err());
            }
            let firmlink_alias = format!("/System/Volumes/Data{home}");
            if std::path::Path::new(&firmlink_alias).exists()
                && same_file::is_same_file(&firmlink_alias, &home).unwrap_or(false)
            {
                assert!(validate_trash_paths(&[firmlink_alias]).is_err());
            }
        }
        assert_eq!(
            validate_trash_paths(&["/tmp/report.pdf".to_string()]).unwrap(),
            vec![PathBuf::from("/tmp/report.pdf")]
        );
        assert_eq!(
            validate_trash_paths(&[
                "/tmp/project".to_string(),
                "/tmp/project/report.pdf".to_string(),
                "/tmp/other.txt".to_string(),
            ])
            .unwrap(),
            vec![
                PathBuf::from("/tmp/other.txt"),
                PathBuf::from("/tmp/project")
            ]
        );
    }
}
