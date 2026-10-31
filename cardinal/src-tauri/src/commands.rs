use crate::{
    DEFAULT_SYSTEM_IGNORE_PATH, LOGIC_START, LogicStartConfig, PENDING_LOGIC_START,
    background::IconViewportUpdate,
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
use crossbeam_channel::{Sender, bounded};
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
use std::{cell::LazyCell, process::Command};
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

/// Result of a search job handled by the background loop.
#[derive(Debug)]
pub enum SearchJobResult {
    Completed(Result<SearchOutcome>),
    /// The index is still being built and the query has no filesystem scope that can be
    /// answered right away. The frontend uses this to show progress instead of waiting.
    Indexing,
    /// A temporary index for this folder scope is being built; the frontend shows the scope
    /// and retries shortly.
    ScopedIndexing {
        scope: String,
    },
    /// The folder scope does not exist (or is not a directory). Retrying cannot help, so the
    /// frontend must stop polling and say so.
    ScopeUnavailable {
        scope: String,
    },
}

#[derive(Debug, Clone)]
pub struct SearchJob {
    pub query: SearchQuery,
    pub options: SearchOptionsPayload,
    pub cancellation_token: CancellationToken,
    pub result_tx: Sender<SearchJobResult>,
}

#[derive(Debug, Clone)]
pub struct NodeInfoRequest {
    pub slab_indices: Vec<SlabIndex>,
    pub response_tx: Sender<Vec<SearchResultNode>>,
}

/// Asks the background loop for the next page of the launch "recent files" list.
pub struct RecentNodesRequest {
    pub offset: usize,
    pub limit: usize,
    pub response_tx: Sender<Vec<SlabIndex>>,
}

#[derive(Default)]
struct SortedViewCache {
    slab_indices: Vec<SlabIndex>,
    nodes: Vec<SearchResultNode>,
}

pub struct SearchState {
    search_tx: Sender<SearchJob>,
    node_info_tx: Sender<NodeInfoRequest>,
    icon_viewport_tx: Sender<IconViewportUpdate>,
    recent_nodes_tx: Sender<RecentNodesRequest>,
    rescan_tx: Sender<CancellationToken>,
    watch_config_tx: Sender<WatchConfigUpdate>,
    sorted_view_cache: Mutex<Option<SortedViewCache>>,
    pub(crate) update_window_state_tx: Sender<()>,
}

impl SearchState {
    pub fn new(
        search_tx: Sender<SearchJob>,
        node_info_tx: Sender<NodeInfoRequest>,
        icon_viewport_tx: Sender<IconViewportUpdate>,
        recent_nodes_tx: Sender<RecentNodesRequest>,
        rescan_tx: Sender<CancellationToken>,
        watch_config_tx: Sender<WatchConfigUpdate>,
        update_window_state_tx: Sender<()>,
    ) -> Self {
        Self {
            search_tx,
            node_info_tx,
            icon_viewport_tx,
            recent_nodes_tx,
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

/// Shown when the background indexing thread is no longer running.
///
/// It used to surface as `Failed to receive search result: RecvError`, which tells the user
/// nothing. The thread only exits when it panics (see the panic log at
/// `~/Library/Logs/Cardinal/panic.log`) or during shutdown, and the only remedy is a restart.
const BACKGROUND_THREAD_GONE: &str = "后台索引线程已停止（可能因异常退出）。请重启 Cardinal；详细原因见 ~/Library/Logs/Cardinal/panic.log";

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
    /// Folder scope being indexed, when `status_code` is `SCOPED_INDEXING`.
    pub scope: Option<String>,
    pub status_code: u8,
}

impl SearchResponse {
    pub const OK: u8 = 0;
    pub const CANCELLED: u8 = 1;
    /// The index is being built; the request was not evaluated. The UI should show progress
    /// and retry automatically once the lifecycle reaches `Updating`/`Ready`.
    pub const INDEXING: u8 = 2;
    /// A temporary index for the requested folder scope is being built.
    pub const SCOPED_INDEXING: u8 = 3;
    /// The folder scope is not a directory; retrying will not help.
    pub const SCOPE_UNAVAILABLE: u8 = 4;
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
        // The receiver only disappears when the background loop is gone (it panicked or the
        // app is shutting down); retrying cannot help, so say what the user should do.
        return Err(BACKGROUND_THREAD_GONE.to_string());
    }

    let received = match result_rx.recv() {
        Ok(value) => Ok(value),
        Err(e) => {
            error!("Failed to receive search result: {e:?}");
            Err(BACKGROUND_THREAD_GONE.to_string())
        }
    };

    match received {
        Err(message) => Err(message),
        Ok(SearchJobResult::Indexing) => Ok(SearchResponse {
            results: vec![],
            highlights: vec![],
            scope: None,
            status_code: SearchResponse::INDEXING,
        }),
        Ok(SearchJobResult::ScopedIndexing { scope }) => Ok(SearchResponse {
            results: vec![],
            highlights: vec![],
            scope: Some(scope),
            status_code: SearchResponse::SCOPED_INDEXING,
        }),
        Ok(SearchJobResult::ScopeUnavailable { scope }) => Ok(SearchResponse {
            results: vec![],
            highlights: vec![],
            scope: Some(scope),
            status_code: SearchResponse::SCOPE_UNAVAILABLE,
        }),
        Ok(SearchJobResult::Completed(res)) => res
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
                    scope: None,
                    status_code,
                }
            })
            .map_err(|e| format!("Failed to process search result: {e:?}")),
    }
}

#[tauri::command(async)]
pub fn get_nodes_info(
    results: Vec<SlabIndex>,
    include_icons: Option<bool>,
    icon_size: Option<f64>,
    state: State<'_, SearchState>,
) -> Vec<NodeInfo> {
    if results.is_empty() {
        return Vec::new();
    }

    let include_icons = include_icons.unwrap_or(true);
    // The list view only needs a 16px bitmap; the tiled view asks for a bigger raster so
    // the same Finder artwork stays sharp at the largest icon preset.
    let icon_size = icon_size
        .filter(|size| size.is_finite() && *size > 0.0)
        .unwrap_or(fs_icon::DEFAULT_ICON_SIZE)
        .min(fs_icon::LARGE_ICON_SIZE);
    let nodes = state.request_nodes(results);

    nodes
        .into_iter()
        .map(|SearchResultNode { path, metadata }| {
            let path = path.to_string_lossy().into_owned();
            let icon = if include_icons {
                fs_icon::icon_of_path_ns_sized(&path, icon_size).map(|data| {
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

/// Returns one page of the launch "recent files" list. The frontend calls this again when the
/// user scrolls near the bottom, so the initial screen stays cheap while browsing still feels
/// endless. Only the empty-query view uses it; real searches are always complete.
#[tauri::command(async)]
pub fn get_recent_nodes(
    offset: usize,
    limit: usize,
    state: State<'_, SearchState>,
) -> Vec<SlabIndex> {
    let (response_tx, response_rx) = bounded::<Vec<SlabIndex>>(1);
    if let Err(e) = state.recent_nodes_tx.send(RecentNodesRequest {
        offset,
        limit,
        response_tx,
    }) {
        error!("Failed to request recent nodes: {e:?}");
        return Vec::new();
    }
    response_rx.recv().unwrap_or_default()
}

#[tauri::command(async)]
pub fn update_icon_viewport(
    id: u64,
    viewport: Vec<SlabIndex>,
    icon_size: Option<f64>,
    state: State<'_, SearchState>,
) {
    let icon_size = icon_size
        .filter(|size| size.is_finite() && *size > 0.0)
        .unwrap_or(fs_icon::DEFAULT_ICON_SIZE)
        .min(fs_icon::LARGE_ICON_SIZE);
    if let Err(e) = state.icon_viewport_tx.send(IconViewportUpdate {
        id,
        viewport,
        icon_size,
    }) {
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

/// An application that can open a given file, as listed by LaunchServices.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenWithApp {
    /// Bundle identifier, e.g. `com.example.player`. Used to launch the app.
    pub bundle_id: String,
    /// Localised display name, e.g. `Fig Player`.
    pub name: String,
    /// Absolute path of the `.app` bundle, used for the icon and as a launch fallback.
    pub path: String,
    /// True when this is the system default handler for the file.
    pub is_default: bool,
}

/// Where a selection maps to in the LaunchServices database.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenWithOptions {
    pub apps: Vec<OpenWithApp>,
    pub default_app: Option<OpenWithApp>,
}

#[cfg(target_os = "macos")]
fn open_with_options_impl(path: &str) -> Option<OpenWithOptions> {
    use objc2_app_kit::NSWorkspace;
    use objc2_foundation::{NSString, NSURL};

    let path_ns = NSString::from_str(path);
    let url = NSURL::fileURLWithPath(&path_ns);
    let workspace = NSWorkspace::sharedWorkspace();

    let default_url = workspace.URLForApplicationToOpenURL(&url);
    let app_urls = workspace.URLsForApplicationsToOpenURL(&url);

    let make_app = |app_url: &NSURL, is_default: bool| -> Option<OpenWithApp> {
        let app_path = app_url.path()?;
        let app_path = app_path.to_string();
        // `NSBundle` gives the localised name and the identifier; reading them from the URL
        // alone would only produce the folder name.
        let bundle = objc2_foundation::NSBundle::bundleWithURL(app_url)?;
        let name = bundle
            .objectForInfoDictionaryKey(&NSString::from_str("CFBundleDisplayName"))
            .and_then(|value| value.downcast::<NSString>().ok())
            .map(|value| value.to_string())
            .or_else(|| bundle.name().map(|name| name.to_string()))
            .unwrap_or_else(|| app_path.clone());
        let bundle_id = bundle.bundleIdentifier()?.to_string();
        Some(OpenWithApp {
            bundle_id,
            name,
            path: app_path,
            is_default,
        })
    };

    let default_app = default_url
        .as_ref()
        .and_then(|app_url| make_app(app_url, true));

    let mut apps: Vec<OpenWithApp> = app_urls
        .iter()
        .filter_map(|app_url| {
            let is_default = default_url
                .as_ref()
                .is_some_and(|default| default.path() == app_url.path());
            make_app(&app_url, is_default)
        })
        .collect();

    // LaunchServices returns the default handler again inside the list; keep one entry each.
    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    apps.dedup_by(|a, b| a.bundle_id == b.bundle_id);

    // Keep the default handler first, like Finder does.
    apps.sort_by_key(|app| !app.is_default);

    Some(OpenWithOptions { apps, default_app })
}

#[cfg(not(target_os = "macos"))]
fn open_with_options_impl(_path: &str) -> Option<OpenWithOptions> {
    None
}

/// Lists the applications that can open `path`, default handler first.
#[tauri::command(async)]
pub fn open_with_options(path: String) -> Option<OpenWithOptions> {
    open_with_options_impl(&path)
}

/// Opens `path` with a specific application.
///
/// `bundle_id` is preferred (it survives the app being moved between `/Applications`,
/// `~/Applications` and other volumes); `app_path` is the fallback when LaunchServices cannot
/// resolve the identifier, which happens for apps that were moved or are not registered.
#[tauri::command]
pub async fn open_with_app(
    path: String,
    bundle_id: String,
    app_path: String,
) -> Result<(), String> {
    // `open -b <bundle id>` asks LaunchServices to resolve the handler, which is the closest
    // equivalent to double-clicking the app in Finder.
    let by_bundle = Command::new("open")
        .arg("-b")
        .arg(&bundle_id)
        .arg(&path)
        .spawn();

    if by_bundle.is_ok() {
        return Ok(());
    }

    let message = by_bundle.err().map(|e| e.to_string()).unwrap_or_default();
    warn!("open -b {bundle_id} failed ({message}); falling back to the bundle path");

    Command::new("open")
        .arg("-a")
        .arg(&app_path)
        .arg(&path)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Failed to open {path} with {bundle_id}: {e}"))
}

/// Opens the macOS "open with" chooser for `path`.
///
/// Finder shows the chooser for a single selection; `open -a` cannot do that, so the app is
/// asked to present the panel directly.
#[tauri::command]
pub async fn choose_app_for_path(path: String) -> Result<(), String> {
    Command::new("open")
        .arg("-R")
        .arg(&path)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Failed to reveal {path}: {e}"))
}

/// Writes UTF-8 text (CSV exports) to an absolute path chosen by the user in a save dialog.
#[tauri::command]
pub async fn write_text_file(path: String, contents: String) -> Result<(), String> {
    std::fs::write(&path, contents).map_err(|e| format!("Failed to write {path}: {e}"))
}

/// Reads a UTF-8 text file (CSV imports) from an absolute path chosen in an open dialog.
#[tauri::command]
pub async fn read_text_file(path: String) -> Result<String, String> {
    std::fs::read_to_string(&path).map_err(|e| format!("Failed to read {path}: {e}"))
}

#[tauri::command]
pub async fn start_logic(
    watch_root: String,
    ignore_paths: Vec<String>,
    include_paths: Option<Vec<String>>,
) {
    let config = LogicStartConfig {
        watch_root,
        ignore_paths,
        include_paths: include_paths.unwrap_or_default(),
    };

    let Some(sender) = LOGIC_START.get() else {
        // The webview can call this before the builder has created the channel. Parking the
        // request (instead of dropping it) is what keeps startup from hanging forever.
        info!("start_logic arrived before the background channel existed; parking it");
        *PENDING_LOGIC_START.lock() = Some(config);
        return;
    };

    if let Err(error) = sender.try_send(config) {
        warn!("Failed to hand off the start request: {error:?}");
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
}
