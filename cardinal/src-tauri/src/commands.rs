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
use crossbeam_channel::{Receiver, Sender, bounded};
use objc2::{
    rc::{Retained, autoreleasepool},
    runtime::ProtocolObject,
};
use objc2_app_kit::{NSPasteboard, NSPasteboardItem, NSPasteboardTypeString, NSPasteboardWriting};
use objc2_foundation::{NSArray, NSString, NSURL};
use parking_lot::Mutex;
use search_cache::{SearchOptions, SearchOutcome, SearchResultNode, SlabIndex, SlabNodeMetadata};
use search_cancel::CancellationToken;
use serde::{Deserialize, Serialize};
use std::{
    cell::LazyCell,
    fs::OpenOptions,
    io::Write,
    process::Command,
};
use tauri::{ActivationPolicy, AppHandle, State};
use tracing::{error, info, warn};

#[derive(Debug, Clone)]
pub struct WatchConfigUpdate {
    pub watch_root: String,
    pub ignore_paths: Vec<String>,
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
    watch_config_tx: Sender<WatchConfigUpdate>,
    sorted_view_cache: Mutex<Option<SortedViewCache>>,
    pub(crate) update_window_state_tx: Sender<()>,
}

impl SearchState {
    pub fn new(
        search_tx: Sender<SearchJob>,
        result_rx: Receiver<Result<SearchOutcome>>,
        node_info_tx: Sender<NodeInfoRequest>,
        icon_viewport_tx: Sender<(u64, Vec<SlabIndex>)>,
        rescan_tx: Sender<()>,
        watch_config_tx: Sender<WatchConfigUpdate>,
        update_window_state_tx: Sender<()>,
    ) -> Self {
        Self {
            search_tx,
            result_rx,
            node_info_tx,
            icon_viewport_tx,
            rescan_tx,
            watch_config_tx,
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

fn expand_path_input(raw: &str) -> Option<String> {
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

    Some(expanded.into_string())
}

/// Normalizes user-provided path input into an absolute path string.
///
/// Expands a leading `~` component using the current `HOME` directory and rejects
/// non-absolute paths (including relative paths and unsupported `~user` forms).
/// Returns `Some` absolute path string when valid, otherwise `None`.
fn normalize_path_input(raw: &str) -> Option<String> {
    let resolved = expand_path_input(raw)?;
    resolved.starts_with('/').then_some(resolved)
}

/// Normalizes ignore entries.
///
/// Keeps raw entries so matching follows gitignore semantics exactly,
/// including empty lines and comments.
fn normalize_ignore_path_input(raw: &str) -> String {
    raw.to_string()
}

pub(crate) fn normalize_watch_config(
    watch_root: &str,
    ignore_paths: Vec<String>,
    fallback_watch_root: Option<&str>,
) -> Option<(String, Vec<String>)> {
    let watch_root = normalize_path_input(watch_root)
        .or_else(|| fallback_watch_root.and_then(normalize_path_input))?;
    let mut ignore_paths = ignore_paths
        .into_iter()
        .map(|path| normalize_ignore_path_input(&path))
        .collect::<Vec<_>>();
    if !ignore_paths
        .iter()
        .any(|path| path == DEFAULT_SYSTEM_IGNORE_PATH)
    {
        ignore_paths.push(DEFAULT_SYSTEM_IGNORE_PATH.to_string());
    }
    Some((watch_root, ignore_paths))
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

#[tauri::command(async)]
pub fn set_watch_config(
    watch_root: String,
    ignore_paths: Vec<String>,
    state: State<'_, SearchState>,
) {
    let Some((watch_root, ignore_paths)) = normalize_watch_config(&watch_root, ignore_paths, None)
    else {
        warn!("Ignoring invalid watch_root: {watch_root:?}");
        return;
    };

    if let Err(e) = state.watch_config_tx.send(WatchConfigUpdate {
        watch_root,
        ignore_paths,
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

#[tauri::command]
pub async fn prompt_save_listed_files_tsv(
    app: AppHandle,
    default_filename: String,
) -> Result<Option<String>, String> {
    let (response_tx, response_rx) = bounded::<Result<Option<String>, String>>(1);

    app.run_on_main_thread(move || {
        let result = prompt_save_listed_files_tsv_impl(default_filename);
        if response_tx.send(result).is_err() {
            error!("Failed to send prompt_save_listed_files_tsv response");
        }
    })
    .map_err(|e| format!("Failed to dispatch prompt save dialog on main thread: {e:?}"))?;

    response_rx
        .recv()
        .map_err(|e| format!("Failed to receive prompt_save_listed_files_tsv response: {e:?}"))?
}

fn normalize_export_default_filename(default_filename: String) -> String {
    let fallback_filename = "cardinal-word-list.tsv".to_string();
    let trimmed = default_filename.trim();
    if trimmed.is_empty() {
        fallback_filename
    } else {
        trimmed.to_string()
    }
}

fn prompt_save_listed_files_tsv_impl(default_filename: String) -> Result<Option<String>, String> {
    let default_filename = normalize_export_default_filename(default_filename);

    info!(
        "Opening listed-files TSV save dialog with default filename: {}",
        default_filename
    );

    let dialog = rfd::FileDialog::new()
        .add_filter("Tab-separated values", &["tsv"])
        .set_file_name(&default_filename);
    let Some(mut path) = dialog.save_file() else {
        info!("Listed-files TSV save dialog canceled by user");
        return Ok(None);
    };

    if path.extension().is_none() {
        path.set_extension("tsv");
    }

    let path = path.to_string_lossy().into_owned();
    Ok(Some(path))
}

#[tauri::command]
pub async fn write_listed_files_tsv(path: String, content: String) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&path)
        .map_err(|e| format!("Failed to open TSV file for writing {}: {e}", path))?;
    file.write_all(content.as_bytes())
        .map_err(|e| format!("Failed to write TSV file to {}: {e}", path))?;
    info!("Saved listed-files TSV to {}", path);
    Ok(())
}

#[tauri::command]
pub async fn append_listed_files_tsv_chunk(path: String, content: String) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| format!("Failed to open TSV file for appending {}: {e}", path))?;
    file.write_all(content.as_bytes())
        .map_err(|e| format!("Failed appending TSV chunk to {}: {e}", path))?;
    Ok(())
}

#[tauri::command]
pub async fn remove_listed_files_tsv(path: String) -> Result<(), String> {
    match std::fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(format!("Failed to remove TSV file {}: {err}", path)),
    }
}

#[tauri::command]
pub async fn start_logic(watch_root: String, ignore_paths: Vec<String>) {
    if let Some(sender) = LOGIC_START.get() {
        let _ = sender.try_send(LogicStartConfig {
            watch_root,
            ignore_paths,
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
    fn normalize_ignore_accepts_absolute_paths_and_globs() {
        assert_eq!(
            normalize_ignore_path_input("/tmp/cache"),
            "/tmp/cache".to_string()
        );
        assert_eq!(
            normalize_ignore_path_input("**/node_modules/**"),
            "**/node_modules/**".to_string()
        );
        assert_eq!(
            normalize_ignore_path_input("build/*.tmp"),
            "build/*.tmp".to_string()
        );
        assert_eq!(normalize_ignore_path_input(".cache"), ".cache".to_string());
        assert_eq!(
            normalize_ignore_path_input("Library/Biome/"),
            "Library/Biome/".to_string()
        );
    }

    #[test]
    fn normalize_ignore_keeps_gitignore_syntax_verbatim() {
        assert_eq!(
            normalize_ignore_path_input("!/important.pyc"),
            "!/important.pyc".to_string()
        );
        assert_eq!(
            normalize_ignore_path_input("# comment"),
            "# comment".to_string()
        );
        assert_eq!(
            normalize_ignore_path_input("relative/path"),
            "relative/path".to_string()
        );
        assert_eq!(
            normalize_ignore_path_input("~someone/**"),
            "~someone/**".to_string()
        );
        assert_eq!(
            normalize_ignore_path_input("../relative/path"),
            "../relative/path".to_string()
        );
    }

    #[test]
    fn normalize_ignore_keeps_empty_and_whitespace_entries() {
        assert_eq!(normalize_ignore_path_input(""), String::new());
        assert_eq!(normalize_ignore_path_input("   "), "   ".to_string());
    }

    #[test]
    fn normalize_watch_config_keeps_globs_and_adds_default_ignore_path() {
        let Some((watch_root, ignore_paths)) = normalize_watch_config(
            "/",
            vec![
                "**/node_modules/**".to_string(),
                String::new(),
                "relative/path".to_string(),
                "/tmp/cache".to_string(),
            ],
            None,
        ) else {
            panic!("watch config should be valid");
        };

        assert_eq!(watch_root, "/");
        assert!(ignore_paths.contains(&"**/node_modules/**".to_string()));
        assert!(ignore_paths.contains(&String::new()));
        assert!(ignore_paths.contains(&"relative/path".to_string()));
        assert!(ignore_paths.contains(&"/tmp/cache".to_string()));
        assert!(ignore_paths.contains(&DEFAULT_SYSTEM_IGNORE_PATH.to_string()));
    }
}
