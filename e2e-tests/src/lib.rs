//! End-to-end performance tests for the `path:` search filter.
//!
//! These tests exercise the full search pipeline (parse → optimize → evaluate)
//! against a real filesystem walk, measuring latency and verifying correctness.
//!
//! Run: cargo test -p e2e-tests -- --test-threads=1 --nocapture

use search_cache::{SearchCache, SearchOptions, SearchQuery};
use search_cancel::CancellationToken;
use std::time::Instant;

/// Walk the root filesystem (same as the Cardinal app does).
/// Runs on a thread with a large stack to avoid overflow on deep directory trees.
#[allow(dead_code)]
fn build_cache() -> SearchCache {
    let ignore_paths = vec![
        std::path::PathBuf::from("/Volumes"),
        std::path::PathBuf::from("/System/Volumes/Data"),
        std::path::PathBuf::from("/private/var"),
        std::path::PathBuf::from("/private/tmp"),
    ];
    eprintln!("Walking filesystem (this takes ~90s)...");
    let start = Instant::now();
    let cache = std::thread::Builder::new()
        .stack_size(512 * 1024 * 1024) // 512MB stack
        .spawn(move || SearchCache::walk_fs_with_ignore(std::path::Path::new("/"), &ignore_paths))
        .expect("failed to spawn thread")
        .join()
        .expect("thread panicked");
    eprintln!("Filesystem walk completed in {:?}", start.elapsed());
    eprintln!("Flat index entries: {}", cache.flat_index_len());
    cache
}

#[allow(dead_code)]
fn search(cache: &mut SearchCache, query: &str) -> (usize, std::time::Duration) {
    let token = CancellationToken::new_search();
    let opts = SearchOptions::default();
    let start = Instant::now();
    let outcome = cache
        .search_query_with_options(
            SearchQuery {
                directory_query: None,
                query: Some(query.to_string()),
            },
            opts,
            token,
        )
        .expect("search should not error");
    let elapsed = start.elapsed();
    let count = outcome.nodes.unwrap_or_default().len();
    (count, elapsed)
}

#[test]
fn star_js_search_performance() {
    let mut cache = build_cache();
    let (count, elapsed) = search(&mut cache, "*.js");
    println!("*.js: {count} results in {elapsed:?}");
    assert!(count > 0, "*.js should find files");
    assert!(
        elapsed.as_secs() < 5,
        "*.js should complete in under 5s, took {elapsed:?}"
    );
}

#[test]
fn path_repos_search_performance() {
    let mut cache = build_cache();
    let (count, elapsed) = search(&mut cache, "path:repos");
    println!("path:repos: {count} results in {elapsed:?}");
    assert!(count > 0, "path:repos should find files");
    assert!(
        elapsed.as_secs() < 5,
        "path:repos should complete in under 5s, took {elapsed:?}"
    );
}

#[test]
fn path_repos_vs_star_js_parody() {
    let mut cache = build_cache();

    let (_, js_time) = search(&mut cache, "*.js");
    let (_, repos_time) = search(&mut cache, "path:repos");

    println!("*.js:      {js_time:?}");
    println!("path:repos: {repos_time:?}");

    // path:repos scans all entries (O(N) substring match on flat index paths).
    // *.js uses the name index (O(log N) lookup + O(matches) expansion).
    // path:repos will be slower than *.js because it scans all N entries,
    // but should still complete in ~1-2s on a modern machine.
    let ratio = repos_time.as_secs_f64() / js_time.as_secs_f64();
    println!("Ratio path/js: {ratio:.2}x");
    assert!(
        repos_time.as_secs() < 5,
        "path:repos should complete in under 5s, took {repos_time:?}"
    );
    assert!(
        js_time.as_secs() < 5,
        "*.js should complete in under 5s, took {js_time:?}"
    );
}

#[test]
fn main_js_path_downloads_path_repos_combined_query() {
    let mut cache = build_cache();
    let (count, elapsed) = search(&mut cache, "main.js path:Downloads path:repos");
    println!("main.js path:Downloads path:repos: {count} results in {elapsed:?}");
    // Should complete without hanging.
    assert!(
        elapsed.as_secs() < 10,
        "combined query should complete in under 10s, took {elapsed:?}"
    );
}

#[test]
fn cancellation_works() {
    let mut cache = build_cache();

    // Start a search, then immediately start another — the first should be cancelled.
    let token1 = CancellationToken::new_search();
    let opts = SearchOptions::default();

    // Immediately create a new search (cancels token1).
    let token2 = CancellationToken::new_search();

    let outcome1 = cache
        .search_query_with_options(
            SearchQuery {
                directory_query: None,
                query: Some("path:repos".to_string()),
            },
            opts,
            token1,
        )
        .expect("search should not error");

    assert!(
        outcome1.nodes.is_none(),
        "First search should be cancelled, but got results"
    );

    let outcome2 = cache
        .search_query_with_options(
            SearchQuery {
                directory_query: None,
                query: Some("*.js".to_string()),
            },
            opts,
            token2,
        )
        .expect("search should not error");

    assert!(
        outcome2.nodes.is_some(),
        "Second search should complete successfully"
    );
    println!(
        "Cancellation: first search cancelled, second returned {} results",
        outcome2.nodes.unwrap().len()
    );
}
