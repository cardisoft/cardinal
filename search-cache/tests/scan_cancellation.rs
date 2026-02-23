use search_cache::SearchCache;
use search_cancel::CancellationToken;
use std::{
    fs,
    path::PathBuf,
    sync::{LazyLock, Mutex},
};
use tempdir::TempDir;

static SCAN_TOKEN_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

fn build_cache() -> (TempDir, SearchCache) {
    let temp_dir = TempDir::new("scan_cancellation").expect("failed to create tempdir");
    let root = temp_dir.path();

    fs::create_dir_all(root.join("src")).expect("failed to create src");
    fs::write(root.join("src/main.rs"), "fn main() {}").expect("failed to create fixture file");

    let cache = SearchCache::walk_fs(root);
    (temp_dir, cache)
}

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
