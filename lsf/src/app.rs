use anyhow::{Context, Result};
use cardinal_sdk::EventWatcher;
use crossbeam_channel::{Receiver, Sender, bounded, unbounded};
use fswalk::WalkData;
use search_cache::{HandleFSEError, SearchCache, SearchOptions, SearchResultNode};
use search_cancel::CancellationToken;
use std::{
    env, fs,
    path::{Path, PathBuf},
    sync::{
        Arc, RwLock,
        atomic::{AtomicBool, Ordering},
    },
    thread::JoinHandle,
    time::Duration,
};

const HISTORY_PATH: &str = "target/search-history.txt";
const IGNORE_PATH: &str = "/System/Volumes/Data";
const MAX_HISTORY_ENTRIES: usize = 200;
static NEVER_STOPPED: AtomicBool = AtomicBool::new(false);

pub struct AppConfig {
    /// The root path to watch and search.
    pub path: PathBuf,
    /// The path to store the search cache.
    pub cache_path: PathBuf,
    /// If we should refresh the cache by full re-scan.
    pub refresh: bool,
}

pub struct AppRuntime {
    command_tx: Sender<Command>,
    worker: Option<JoinHandle<Result<()>>>,
    status: Arc<RwLock<RuntimeStatus>>,
    history: Arc<RwLock<Vec<String>>>,
}

#[derive(Debug)]
pub struct SearchResponse {
    pub results: Vec<SearchResultNode>,
    pub total_indexed: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppLifecycleStatus {
    /// Initializing the cache by restoring the previous
    /// cache or walking the filesystem if no cache is available.
    Initializing,
    /// Handling filesystem events and updating the cache accordingly.
    Updating,
    /// Cache is ready and search requests can be handled.
    Ready,
}

#[derive(Debug, Clone)]
pub struct RuntimeStatus {
    pub lifecycle: AppLifecycleStatus,
    pub scanned_files: usize,
}

enum Command {
    Search {
        query: String,
        respond_to: Sender<Result<SearchResponse>>,
    },
    Shutdown {
        respond_to: Sender<Result<()>>,
    },
}

impl AppRuntime {
    pub fn start(config: AppConfig) -> Result<Self> {
        let watch_path = watcher_path(&config.path);
        let (command_tx, command_rx) = unbounded();
        let status = Arc::new(RwLock::new(RuntimeStatus {
            lifecycle: AppLifecycleStatus::Initializing,
            scanned_files: 0,
        }));
        let history = Arc::new(RwLock::new(load_history(Path::new(HISTORY_PATH))?));
        let worker_status = Arc::clone(&status);
        let worker =
            std::thread::spawn(move || worker_loop(config, watch_path, worker_status, command_rx));

        Ok(Self {
            command_tx,
            worker: Some(worker),
            status,
            history,
        })
    }

    pub fn search(&self, query: impl Into<String>) -> Result<SearchResponse> {
        let (respond_to, response_rx) = bounded(1);
        self.command_tx
            .send(Command::Search {
                query: query.into(),
                respond_to,
            })
            .context("search worker is closed")?;
        response_rx
            .recv()
            .context("search response channel is closed")?
    }

    pub fn record_history(&self, query: impl Into<String>) -> Result<Vec<String>> {
        let query = query.into();
        let mut history = self
            .history
            .write()
            .map_err(|_| anyhow::anyhow!("history lock poisoned"))?;
        record_history_entry(&mut history, &query)?;
        save_history(Path::new(HISTORY_PATH), &history)?;
        Ok(history.clone())
    }

    pub fn history(&self) -> Result<Vec<String>> {
        self.history
            .read()
            .map(|history| history.clone())
            .map_err(|_| anyhow::anyhow!("history lock poisoned"))
    }

    pub fn status(&self) -> Result<RuntimeStatus> {
        self.status
            .read()
            .map(|status| status.clone())
            .map_err(|_| anyhow::anyhow!("runtime status lock poisoned"))
    }

    pub fn shutdown(mut self) -> Result<()> {
        let (respond_to, response_rx) = bounded(1);
        self.command_tx
            .send(Command::Shutdown { respond_to })
            .context("search worker is closed")?;
        response_rx
            .recv()
            .context("shutdown response channel is closed")??;

        if let Some(worker) = self.worker.take() {
            worker
                .join()
                .map_err(|_| anyhow::anyhow!("search worker panicked"))??;
        }

        Ok(())
    }
}

fn load_cache(config: &AppConfig, status: &Arc<RwLock<RuntimeStatus>>) -> Result<SearchCache> {
    let ignore_paths = vec![PathBuf::from(IGNORE_PATH)];
    if config.refresh {
        return walk_cache_with_progress(&config.path, &ignore_paths, status);
    }

    SearchCache::try_read_persistent_cache(
        &config.path,
        &config.cache_path,
        &ignore_paths,
        &NEVER_STOPPED,
    )
    .or_else(|_| walk_cache_with_progress(&config.path, &ignore_paths, status))
}

fn worker_loop(
    config: AppConfig,
    watch_path: String,
    status: Arc<RwLock<RuntimeStatus>>,
    command_rx: Receiver<Command>,
) -> Result<()> {
    let mut cache = load_cache(&config, &status)?;
    set_status(&status, AppLifecycleStatus::Ready, cache.get_total_files());
    let (_, mut event_watcher) = EventWatcher::spawn(
        watch_path.clone(),
        cache.last_event_id(),
        0.1,
        cache.ignore_paths(),
    );
    let mut cache = Some(cache);

    loop {
        crossbeam_channel::select! {
            recv(command_rx) -> command => match command.context("command channel is closed")? {
                Command::Search { query, respond_to } => {
                    let cache = cache.as_mut().expect("cache must exist before shutdown");
                    let response = cache
                        .search_with_options(&query, SearchOptions::default(), CancellationToken::noop())
                        .map(|outcome| SearchResponse {
                            results: outcome
                                .nodes
                                .map(|nodes| cache.expand_file_nodes(&nodes))
                                .unwrap_or_default(),
                            total_indexed: cache.get_total_files(),
                    });
                    let _ = respond_to.send(response);
                }
                Command::Shutdown { respond_to } => {
                    let result = cache
                        .take()
                        .expect("cache must exist before shutdown")
                        .flush_to_file(&config.cache_path)
                        .context("failed to write cache to file");
                    let _ = respond_to.send(result);
                    break;
                }
            },
            recv(event_watcher) -> events => {
                let cache = cache.as_mut().expect("cache must exist before shutdown");
                let events = events.context("event stream is closed")?;
                if let Err(HandleFSEError::Rescan) = cache.handle_fs_events(events) {
                    set_status(&status, AppLifecycleStatus::Updating, cache.get_total_files());
                    #[allow(unused_assignments)]
                    {
                        event_watcher = EventWatcher::noop();
                    }
                    let mut scan_root = PathBuf::new();
                    let mut scan_ignore_paths = Vec::new();
                    let walk_data = cache.walk_data(
                        &mut scan_root,
                        &mut scan_ignore_paths,
                        CancellationToken::new_scan(),
                    );
                    let _ = cache.rescan_with_walk_data(&walk_data);
                    set_status(&status, AppLifecycleStatus::Ready, cache.get_total_files());
                    event_watcher = EventWatcher::spawn(
                        watch_path.clone(),
                        cache.last_event_id(),
                        0.1,
                        cache.ignore_paths(),
                    )
                    .1;
                }
            }
        }
    }

    Ok(())
}

fn watcher_path(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

pub fn resolve_cache_path(path: &Path) -> Result<PathBuf> {
    let raw = path.to_string_lossy();
    if raw == "~" || raw.starts_with("~/") {
        let home = env::var_os("HOME").ok_or_else(|| anyhow::anyhow!("HOME is not set"))?;
        let mut expanded = PathBuf::from(home);
        if raw.len() > 2 {
            expanded.push(&raw[2..]);
        }
        Ok(expanded)
    } else {
        Ok(path.to_path_buf())
    }
}

fn set_status(
    status: &Arc<RwLock<RuntimeStatus>>,
    lifecycle: AppLifecycleStatus,
    scanned_files: usize,
) {
    if let Ok(mut state) = status.write() {
        state.lifecycle = lifecycle;
        state.scanned_files = scanned_files;
    }
}

fn walk_cache_with_progress(
    path: &Path,
    ignore_paths: &[PathBuf],
    status: &Arc<RwLock<RuntimeStatus>>,
) -> Result<SearchCache> {
    let walk_data = WalkData::new(path, ignore_paths, false, || false);
    let done = AtomicBool::new(false);

    std::thread::scope(|scope| {
        scope.spawn(|| {
            while !done.load(Ordering::Relaxed) {
                let scanned = walk_data.num_files.load(Ordering::Relaxed)
                    + walk_data.num_dirs.load(Ordering::Relaxed);
                set_status(status, AppLifecycleStatus::Initializing, scanned);
                std::thread::sleep(Duration::from_millis(75));
            }
        });

        let cache = SearchCache::walk_fs_with_walk_data(&walk_data, &NEVER_STOPPED)
            .expect("filesystem walk should complete");
        done.store(true, Ordering::Relaxed);
        let scanned = walk_data.num_files.load(Ordering::Relaxed)
            + walk_data.num_dirs.load(Ordering::Relaxed);
        set_status(status, AppLifecycleStatus::Initializing, scanned);
        Ok(cache)
    })
}

fn load_history(path: &Path) -> Result<Vec<String>> {
    match fs::read_to_string(path) {
        Ok(contents) => Ok(contents
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(ToOwned::to_owned)
            .collect()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(err) => Err(err.into()),
    }
}

fn save_history(path: &Path, history: &[String]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut content = history.join("\n");
    if !content.is_empty() {
        content.push('\n');
    }
    fs::write(path, content)?;
    Ok(())
}

fn record_history_entry(history: &mut Vec<String>, query: &str) -> Result<()> {
    let query = query.trim();
    if query.is_empty() {
        return Ok(());
    }
    if query.contains('\n') {
        anyhow::bail!("history entries must be single-line");
    }
    history.retain(|entry| entry != query);
    history.push(query.to_string());
    if history.len() > MAX_HISTORY_ENTRIES {
        let drain = history.len() - MAX_HISTORY_ENTRIES;
        history.drain(..drain);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{record_history_entry, resolve_cache_path, watcher_path};
    use std::path::Path;

    #[test]
    fn watcher_path_uses_requested_root() {
        assert_eq!(
            watcher_path(Path::new("/usr/local/go-1.20")),
            "/usr/local/go-1.20"
        );
    }

    #[test]
    fn watcher_path_preserves_filesystem_root() {
        assert_eq!(watcher_path(Path::new("/")), "/");
    }

    #[test]
    fn record_history_moves_existing_query_to_end() {
        let mut history = vec!["alpha".to_string(), "beta".to_string()];
        record_history_entry(&mut history, "alpha").unwrap();
        assert_eq!(history, vec!["beta".to_string(), "alpha".to_string()]);
    }

    #[test]
    fn resolve_cache_path_keeps_absolute_path() {
        let path = Path::new("/tmp/cardinal.zstd");
        assert_eq!(resolve_cache_path(path).unwrap(), path);
    }
}
