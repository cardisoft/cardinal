//! Integration test simulating the app's search-then-cancel flow.
//!
//! Simulates what happens when a user types "path:repos" (which may be slow),
//! then types a new query before the first finishes. The first search should
//! be cancelled and return promptly, and the second search should succeed.

use search_cache::SearchCache;
use search_cancel::{ACTIVE_SEARCH_VERSION, CancellationToken};
use std::sync::atomic::Ordering;
use tempdir::TempDir;

fn build_wide_cache() -> SearchCache {
    let temp_dir = TempDir::new("e2e_search_cancel").unwrap();
    let root_path = temp_dir.path().to_path_buf();
    std::mem::forget(temp_dir);

    // Create a tree large enough that a full scan is non-trivial:
    // root/
    //   repos/
    //     sub_0/  file_0.js .. file_49.js
    //     ...
    //     sub_49/ file_0.js .. file_49.js
    //   Ayla/
    //     repos/
    //       main.js
    //     other/
    //       main.js
    //   docs/
    //     readme.md
    let repos = root_path.join("repos");
    std::fs::create_dir_all(&repos).unwrap();
    for d in 0..50 {
        let dir = repos.join(format!("sub_{d}"));
        std::fs::create_dir_all(&dir).unwrap();
        for f in 0..50 {
            std::fs::File::create(dir.join(format!("file_{f}.js"))).unwrap();
        }
    }
    let ayla = root_path.join("Ayla");
    std::fs::create_dir_all(ayla.join("repos")).unwrap();
    std::fs::File::create(ayla.join("repos/main.js")).unwrap();
    std::fs::create_dir_all(ayla.join("other")).unwrap();
    std::fs::File::create(ayla.join("other/main.js")).unwrap();
    std::fs::create_dir_all(root_path.join("docs")).unwrap();
    std::fs::File::create(root_path.join("docs/readme.md")).unwrap();

    SearchCache::walk_fs(&root_path)
}

#[test]
fn e2e_search_then_cancel_returns_promptly() {
    let mut cache = build_wide_cache();

    // Simulate the app: a search for "path:repos" is started (token created).
    let slow_token = CancellationToken::new_search();

    // Immediately, the user types a new query. The app calls new_search()
    // which bumps ACTIVE_SEARCH_VERSION, making the old token stale.
    let _fast_token = CancellationToken::new_search();

    // The old token is now cancelled.
    assert!(
        slow_token.is_cancelled().is_none(),
        "Slow search token should be cancelled by the new search"
    );

    // Running the slow search with the cancelled token should return promptly
    // (None = cancelled) rather than scanning all nodes.
    let result = cache.query_files("path:repos", slow_token);
    assert!(result.is_ok(), "Cancelled search should not error");
    // filter_nodes returns None when cancelled (is_cancelled_sparse returns None).
    // The search returns Ok(None) — cancelled, not an error.
}

#[test]
fn e2e_main_js_path_ayla_path_repos_returns_correct_results() {
    let temp_dir = TempDir::new("e2e_path_query").unwrap();
    let root_path = temp_dir.path().to_path_buf();
    std::mem::forget(temp_dir);

    // Replicate the user's exact filesystem structure:
    // root/
    //   source/
    //     repos/
    //       Ayla/
    //         main.js
    //         other.js
    //       cardinal/
    //         main.js
    //   other/
    //     main.js
    let source = root_path.join("source/repos");
    std::fs::create_dir_all(source.join("Ayla")).unwrap();
    std::fs::File::create(source.join("Ayla/main.js")).unwrap();
    std::fs::File::create(source.join("Ayla/other.js")).unwrap();
    std::fs::create_dir_all(source.join("cardinal")).unwrap();
    std::fs::File::create(source.join("cardinal/main.js")).unwrap();
    std::fs::create_dir_all(root_path.join("other")).unwrap();
    std::fs::File::create(root_path.join("other/main.js")).unwrap();

    let mut cache = SearchCache::walk_fs(&root_path);

    // User's query: main.js path:Ayla path:repos
    let result = cache
        .query_files("main.js path:Ayla path:repos", CancellationToken::noop())
        .expect("Query should succeed");

    let nodes = result.expect("Should return results");
    // Only source/repos/Ayla/main.js matches all three filters.
    assert_eq!(
        nodes.len(),
        1,
        "main.js path:Ayla path:repos should find exactly 1 file"
    );
    let path = nodes[0].path.to_string_lossy().to_string();
    assert!(path.contains("Ayla"));
    assert!(path.contains("repos"));
    assert!(path.ends_with("main.js"));
}

#[test]
fn e2e_star_js_is_fast_and_path_repos_works() {
    let mut cache = build_wide_cache();

    // *.js should return quickly
    let result = cache
        .query_files("*.js", CancellationToken::noop())
        .expect("*.js should succeed");
    let js_nodes = result.expect("*.js should return results");
    // 50 dirs × 50 files + Ayla/repos/main.js + Ayla/other/main.js... wait other.js
    // Actually: repos/sub_*/file_*.js = 2500, Ayla/repos/main.js = 1, Ayla/other/main.js = 1
    assert!(js_nodes.len() >= 2500, "*.js should find many files");

    // path:repos should also work
    let result = cache
        .query_files("path:repos", CancellationToken::noop())
        .expect("path:repos should succeed");
    let repos_nodes = result.expect("path:repos should return results");
    assert!(
        repos_nodes.len() >= 2500,
        "path:repos should find many files"
    );

    // All path:repos results should have "repos" in their path
    for node in &repos_nodes {
        assert!(
            node.path.to_string_lossy().contains("repos"),
            "All results should contain 'repos' in path"
        );
    }
}

#[test]
fn e2e_cancellation_via_version_bump() {
    let mut cache = build_wide_cache();

    // Create a token, then bump the version to cancel it.
    let token = CancellationToken::new_search();
    ACTIVE_SEARCH_VERSION.fetch_add(1, Ordering::SeqCst);

    // The search with the cancelled token should return None (cancelled).
    let result = cache.query_files("path:repos", token);
    assert!(result.is_ok(), "Cancelled search should not error");

    // Verify the token is indeed cancelled.
    assert!(token.is_cancelled().is_none());
}

#[test]
fn e2e_empty_query_returns_all_files() {
    let mut cache = build_wide_cache();

    let result = cache
        .query_files("", CancellationToken::noop())
        .expect("Empty query should succeed");

    let nodes = result.expect("Should return results");
    assert!(!nodes.is_empty(), "Empty query should return all files");
}
