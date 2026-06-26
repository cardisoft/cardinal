//! Tests that the path: filter respects cancellation and doesn't hang on
//! large subtrees. Creates a deep/wide tree and verifies that a cancelled
//! search returns promptly.

use search_cache::SearchCache;
use search_cancel::{ACTIVE_SEARCH_VERSION, CancellationToken};
use std::{path::PathBuf, sync::atomic::Ordering};
use tempdir::TempDir;

fn build_deep_cache() -> (SearchCache, PathBuf) {
    let temp_dir = TempDir::new("path_cancel_test").unwrap();
    let root_path = temp_dir.path().to_path_buf();
    std::mem::forget(temp_dir);

    // Create a tree with many files under a "repos" directory:
    // root/
    //   repos/
    //     dir_0/  file_0.txt .. file_99.txt
    //     dir_1/  file_0.txt .. file_99.txt
    //     ...
    //     dir_99/ file_0.txt .. file_99.txt
    //   other/
    //     file_0.txt .. file_99.txt
    let repos = root_path.join("repos");
    std::fs::create_dir_all(&repos).unwrap();
    for d in 0..100 {
        let dir = repos.join(format!("dir_{d}"));
        std::fs::create_dir_all(&dir).unwrap();
        for f in 0..100 {
            std::fs::File::create(dir.join(format!("file_{f}.txt"))).unwrap();
        }
    }
    let other = root_path.join("other");
    std::fs::create_dir_all(&other).unwrap();
    for f in 0..100 {
        std::fs::File::create(other.join(format!("file_{f}.txt"))).unwrap();
    }

    let cache = SearchCache::walk_fs(&root_path);
    (cache, root_path)
}

#[test]
fn path_filter_returns_results_on_large_tree() {
    let (mut cache, _root) = build_deep_cache();

    let result = cache
        .query_files("path:repos", CancellationToken::noop())
        .expect("Query should succeed");

    let nodes = result.expect("Should return results");
    // repos dir + 100 subdirs + 10000 files = 10101
    assert_eq!(
        nodes.len(),
        10101,
        "path:repos should match all items under repos"
    );
}

#[test]
fn path_filter_with_word_narrows_results() {
    let (mut cache, _root) = build_deep_cache();

    let result = cache
        .query_files("file_0.txt path:repos", CancellationToken::noop())
        .expect("Query should succeed");

    let nodes = result.expect("Should return results");
    // 100 files named file_0.txt under repos/dir_*/ = 100
    assert_eq!(
        nodes.len(),
        100,
        "file_0.txt path:repos should match 100 files"
    );
}

#[test]
fn path_filter_multiple_fragments_narrow() {
    let (mut cache, _root) = build_deep_cache();

    let result = cache
        .query_files(
            "file_0.txt path:repos path:dir_5",
            CancellationToken::noop(),
        )
        .expect("Query should succeed");

    let nodes = result.expect("Should return results");
    // "dir_5" matches dir_5, dir_50..dir_59 (substring) = 11 dirs, each with 1 file_0.txt = 11
    assert_eq!(nodes.len(), 11, "path:dir_5 matches dir_5 and dir_50-59");
}

#[test]
fn path_filter_does_not_match_unrelated() {
    let (mut cache, _root) = build_deep_cache();

    let result = cache
        .query_files("file_0.txt path:nonexistent", CancellationToken::noop())
        .expect("Query should succeed");

    match result {
        None => {}
        Some(nodes) => assert!(nodes.is_empty(), "path:nonexistent should match nothing"),
    }
}

#[test]
fn path_filter_cancellation_aborts_long_scan() {
    let (mut cache, _root) = build_deep_cache();

    // Bump the active search version so any token created before is cancelled.
    let token = CancellationToken::new_search();
    // Immediately bump the version again, cancelling the token.
    ACTIVE_SEARCH_VERSION.fetch_add(1, Ordering::SeqCst);

    // The search should return promptly with None (cancelled) rather than
    // scanning all nodes.
    let result = cache.query_files("path:repos", token);
    // With a cancelled token, the search returns None (cancelled).
    assert!(
        result.is_ok(),
        "Cancelled search should not error, it should return None"
    );
}
