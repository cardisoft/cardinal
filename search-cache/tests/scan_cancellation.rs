use search_cache::SearchCache;
use search_cancel::CancellationToken;
use std::{
    fs,
    path::PathBuf,
    sync::{LazyLock, Mutex, atomic::AtomicBool},
};
use tempdir::TempDir;

static SCAN_TOKEN_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

/// A stop flag that is always set — simulates a shutdown-in-progress scenario.
static ALWAYS_STOPPED: AtomicBool = AtomicBool::new(true);

/// A stop flag that is never set — stand-in for a running application.
static NEVER_STOPPED: AtomicBool = AtomicBool::new(false);

fn build_cache() -> (TempDir, SearchCache) {
    let temp_dir = TempDir::new("scan_cancellation").expect("failed to create tempdir");
    let root = temp_dir.path();

    fs::create_dir_all(root.join("src")).expect("failed to create src");
    fs::write(root.join("src/main.rs"), "fn main() {}").expect("failed to create fixture file");

    let cache = SearchCache::walk_fs(root);
    (temp_dir, cache)
}

// ── SearchCache::noop() ──────────────────────────────────────────────────────

#[test]
fn noop_cache_is_noop_returns_true() {
    let cache = SearchCache::noop(PathBuf::from("/some/path"), vec![], &NEVER_STOPPED);
    assert!(
        cache.is_noop(),
        "noop() constructor must produce a cache that reports is_noop() == true"
    );
    assert_eq!(
        cache.get_total_files(),
        0,
        "noop cache should contain zero file nodes"
    );
}

#[test]
fn noop_cache_preserves_ignore_paths() {
    let ignore = vec![
        PathBuf::from("/some/path/node_modules"),
        PathBuf::from("/some/path/.git"),
    ];
    let cache = SearchCache::noop(PathBuf::from("/some/path"), ignore.clone(), &NEVER_STOPPED);
    assert_eq!(
        cache.ignore_paths(),
        &ignore,
        "noop() should store the supplied ignore paths unchanged"
    );
}

#[test]
fn noop_cache_empty_ignore_paths_is_preserved() {
    let cache = SearchCache::noop(PathBuf::from("/x"), vec![], &NEVER_STOPPED);
    assert!(
        cache.ignore_paths().is_empty(),
        "noop() with empty ignore_paths should store an empty vec"
    );
}

// ── SearchCache::is_noop() ───────────────────────────────────────────────────

#[test]
fn populated_cache_is_not_noop() {
    let (_tmp, cache) = build_cache();
    assert!(
        !cache.is_noop(),
        "a cache built by walk_fs must NOT report is_noop() == true"
    );
    assert!(
        cache.get_total_files() > 0,
        "walk_fs cache should contain at least one node"
    );
}

// ── walk_data cancel closure — stale token ───────────────────────────────────

#[test]
fn stale_scan_token_cancels_rescan_walk_data() {
    let _guard = SCAN_TOKEN_LOCK
        .lock()
        .expect("scan token lock should not be poisoned");

    let (_temp_dir, mut cache) = build_cache();
    let before = cache.get_total_files();

    let stale = CancellationToken::new_scan();
    let _latest = CancellationToken::new_scan();

    let mut scan_root = PathBuf::new();
    let mut scan_ignore_paths = Vec::new();
    let walk_data = cache.walk_data(&mut scan_root, &mut scan_ignore_paths, stale);

    let rescan_result = cache.rescan_with_walk_data(&walk_data);
    assert!(
        rescan_result.is_none(),
        "stale scan token should cancel this rescan request"
    );
    assert_eq!(
        cache.get_total_files(),
        before,
        "cancelled rescan should keep cache unchanged"
    );
}

#[test]
fn stale_token_on_noop_cache_stays_noop() {
    let _guard = SCAN_TOKEN_LOCK
        .lock()
        .expect("scan token lock should not be poisoned");

    let tmp = TempDir::new("stale_on_noop").expect("failed to create tempdir");
    fs::write(tmp.path().join("file.txt"), "data").expect("failed to create fixture");

    let mut cache = SearchCache::noop(tmp.path().to_path_buf(), vec![], &NEVER_STOPPED);

    let stale = CancellationToken::new_scan();
    let _newer = CancellationToken::new_scan();

    let mut p1 = PathBuf::new();
    let mut p2 = Vec::new();
    let walk_data = cache.walk_data(&mut p1, &mut p2, stale);
    let result = cache.rescan_with_walk_data(&walk_data);

    assert!(
        result.is_none(),
        "stale token should cancel the rescan even on a noop starting cache"
    );
    assert!(
        cache.is_noop(),
        "noop cache should remain noop after a cancelled rescan"
    );
}

// ── walk_data cancel closure — active token ──────────────────────────────────

#[test]
fn active_scan_token_allows_rescan_to_complete() {
    let _guard = SCAN_TOKEN_LOCK
        .lock()
        .expect("scan token lock should not be poisoned");

    let (tmp, mut cache) = build_cache();

    // Add a new file so we can observe the rescan picking it up.
    fs::write(tmp.path().join("extra.txt"), "new").expect("failed to create extra file");

    let active = CancellationToken::new_scan();

    let mut scan_root = PathBuf::new();
    let mut scan_ignore_paths = Vec::new();
    let walk_data = cache.walk_data(&mut scan_root, &mut scan_ignore_paths, active);

    let result = cache.rescan_with_walk_data(&walk_data);
    assert!(
        result.is_some(),
        "active scan token must allow rescan to complete"
    );
    assert!(
        !cache.is_noop(),
        "cache must not be noop after a successful rescan"
    );
    assert!(
        cache.get_total_files() > 0,
        "cache should contain files after a successful rescan"
    );
}

#[test]
fn is_noop_false_after_rescan_of_noop_cache() {
    let _guard = SCAN_TOKEN_LOCK
        .lock()
        .expect("scan token lock should not be poisoned");

    let tmp = TempDir::new("noop_rescan").expect("failed to create tempdir");
    fs::create_dir_all(tmp.path().join("sub")).expect("failed to create sub");
    fs::write(tmp.path().join("sub/a.rs"), "fn f() {}").expect("failed to create file");

    let mut cache = SearchCache::noop(tmp.path().to_path_buf(), vec![], &NEVER_STOPPED);
    assert!(cache.is_noop(), "precondition: starts as noop");

    let active = CancellationToken::new_scan();
    let mut p1 = PathBuf::new();
    let mut p2 = Vec::new();
    let walk_data = cache.walk_data(&mut p1, &mut p2, active);
    let result = cache.rescan_with_walk_data(&walk_data);

    assert!(result.is_some(), "rescan with active token should succeed");
    assert!(
        !cache.is_noop(),
        "after a successful rescan a previously-noop cache must no longer be noop"
    );
    assert!(
        cache.get_total_files() > 0,
        "rescan should have discovered the fixture files"
    );
}

// ── stop flag (AtomicBool) cancellation ──────────────────────────────────────

#[test]
fn stop_flag_cancels_rescan_independently_of_scan_token() {
    let _guard = SCAN_TOKEN_LOCK
        .lock()
        .expect("scan token lock should not be poisoned");

    let tmp = TempDir::new("stop_flag").expect("failed to create tempdir");
    fs::write(tmp.path().join("b.txt"), "hi").expect("failed to create fixture");

    // noop cache tied to ALWAYS_STOPPED — its stop flag is already true.
    let mut cache = SearchCache::noop(tmp.path().to_path_buf(), vec![], &ALWAYS_STOPPED);

    // Use an *active* scan token so that only the stop flag drives cancellation.
    let active = CancellationToken::new_scan();
    let mut p1 = PathBuf::new();
    let mut p2 = Vec::new();
    let walk_data = cache.walk_data(&mut p1, &mut p2, active);
    let result = cache.rescan_with_walk_data(&walk_data);

    assert!(
        result.is_none(),
        "a raised stop flag should cancel the rescan even when the scan token is active"
    );
    assert!(
        cache.is_noop(),
        "cache must remain noop when rescan is aborted by the stop flag"
    );
}

// ── is_noop drives cache persistence decision ────────────────────────────────

#[test]
fn noop_cache_signals_do_not_persist() {
    // Mirrors the logic in background.rs:
    //   tx.send((!cache.is_noop()).then(|| cache))
    // A noop cache should produce None (skip write); a real cache should produce Some.
    let noop = SearchCache::noop(PathBuf::from("/p"), vec![], &NEVER_STOPPED);
    let persist_noop: Option<()> = (!noop.is_noop()).then_some(());
    assert!(
        persist_noop.is_none(),
        "noop cache must produce None for the persistence gate"
    );

    let (_tmp, real) = build_cache();
    let persist_real: Option<()> = (!real.is_noop()).then_some(());
    assert!(
        persist_real.is_some(),
        "populated cache must produce Some for the persistence gate"
    );
}

// ── noop cache property tests (last_event_id, rescan_count, search) ──────────

#[test]
fn noop_cache_last_event_id_is_zero() {
    let mut cache = SearchCache::noop(PathBuf::from("/w"), vec![], &NEVER_STOPPED);
    assert_eq!(
        cache.last_event_id(),
        0,
        "noop cache must have last_event_id == 0 (used to spawn EventWatcher)"
    );
}

#[test]
fn noop_cache_rescan_count_is_zero() {
    let cache = SearchCache::noop(PathBuf::from("/w"), vec![], &NEVER_STOPPED);
    assert_eq!(
        cache.rescan_count(),
        0,
        "freshly created noop cache must have rescan_count == 0"
    );
}

#[test]
fn noop_cache_search_returns_empty_results() {
    use search_cache::SearchOptions;
    let mut cache = SearchCache::noop(PathBuf::from("/w"), vec![], &NEVER_STOPPED);
    let outcome = cache
        .search_with_options(
            "anything",
            SearchOptions::default(),
            CancellationToken::noop(),
        )
        .expect("search on noop should not error");
    let nodes = outcome.nodes.unwrap_or_default();
    assert!(
        nodes.is_empty(),
        "searching a noop cache must return an empty result set"
    );
}

#[test]
fn noop_cache_search_empty_returns_empty_vec() {
    let cache = SearchCache::noop(PathBuf::from("/w"), vec![], &NEVER_STOPPED);
    let result = cache
        .search_empty(CancellationToken::noop())
        .expect("search_empty on noop should not be cancelled");
    assert!(
        result.is_empty(),
        "search_empty on noop cache must return an empty vec"
    );
}

// ── noop cache walk_data propagation ─────────────────────────────────────────

#[test]
fn noop_cache_walk_data_propagates_path_and_ignore() {
    let _guard = SCAN_TOKEN_LOCK
        .lock()
        .expect("scan token lock should not be poisoned");

    let path = PathBuf::from("/my/root");
    let ignore = vec![PathBuf::from("/my/root/.git")];
    let cache = SearchCache::noop(path.clone(), ignore.clone(), &NEVER_STOPPED);

    let mut p1 = PathBuf::new();
    let mut p2 = Vec::new();
    let token = CancellationToken::new_scan();
    let wd = cache.walk_data(&mut p1, &mut p2, token);

    assert_eq!(
        wd.root_path, &path,
        "walk_data from noop must propagate root path"
    );
    assert_eq!(
        wd.ignore_directories, &ignore,
        "walk_data from noop must propagate ignore paths"
    );
}

// ── handle_fs_events on noop cache ───────────────────────────────────────────

#[test]
fn noop_cache_handle_fs_events_with_empty_events_is_ok() {
    let mut cache = SearchCache::noop(PathBuf::from("/w"), vec![], &NEVER_STOPPED);
    let result = cache.handle_fs_events(vec![]);
    assert!(
        result.is_ok(),
        "handle_fs_events with empty events on noop cache should succeed"
    );
}

#[test]
fn noop_cache_handle_fs_events_with_create_event_panics_on_invalid_slab() {
    use cardinal_sdk::{EventFlag, FsEvent};
    let mut cache = SearchCache::noop(PathBuf::from("/w"), vec![], &NEVER_STOPPED);
    let event = FsEvent {
        path: PathBuf::from("/w/new_file.txt"),
        id: 42,
        flag: EventFlag::ItemCreated | EventFlag::ItemIsFile,
    };
    // The noop cache has an empty slab; attempting to scan a path triggers an
    // "invalid slab index" panic because the root node doesn't exist.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        cache.handle_fs_events(vec![event])
    }));
    assert!(
        result.is_err(),
        "handle_fs_events on noop cache with a real event should panic (invalid slab)"
    );
}
