//! Tests for the `path:` filter, which keeps items whose full absolute path
//! contains the argument as a substring of any path component. Multiple
//! `path:` filters combine with AND, each narrowing the result set further.

use search_cache::{SearchCache, SearchOptions};
use search_cancel::CancellationToken;
use std::path::PathBuf;
use tempdir::TempDir;

/// Build a test cache with nested directory structure:
/// root/
///   main.js
///   Ayla/
///     repos/
///       main.js
///       other.js
///     other/
///       main.js
///   repos/
///     main.js
fn build_path_cache() -> (SearchCache, PathBuf) {
    let temp_dir = TempDir::new("path_filter_test").unwrap();
    let root_path = temp_dir.path().to_path_buf();
    std::mem::forget(temp_dir);

    let files = [
        "main.js",
        "Ayla/repos/main.js",
        "Ayla/repos/other.js",
        "Ayla/other/main.js",
        "repos/main.js",
    ];

    for file in files {
        let full = root_path.join(file);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::File::create(full).unwrap();
    }

    let cache = SearchCache::walk_fs(&root_path);
    (cache, root_path)
}

#[test]
fn path_filter_single_fragment_matches_descendants() {
    let (mut cache, _root) = build_path_cache();

    let query = "main.js path:Ayla";
    let result = cache
        .query_files(query, CancellationToken::noop())
        .expect("Query should succeed");
    let nodes = result.expect("Should return results");

    // Only main.js files whose path contains "Ayla":
    // Ayla/repos/main.js, Ayla/other/main.js (2). root/main.js and repos/main.js excluded.
    assert_eq!(
        nodes.len(),
        2,
        "path:Ayla should narrow to files under Ayla"
    );
    for node in &nodes {
        assert!(
            node.path.to_string_lossy().contains("Ayla"),
            "all results should live under an Ayla directory"
        );
    }
}

#[test]
fn path_filter_multiple_fragments_narrow_with_and() {
    let (mut cache, _root) = build_path_cache();

    // main.js path:Ayla path:repos -> only Ayla/repos/main.js
    let query = "main.js path:Ayla path:repos";
    let result = cache
        .query_files(query, CancellationToken::noop())
        .expect("Query should succeed");
    let nodes = result.expect("Should return results");

    assert_eq!(
        nodes.len(),
        1,
        "two path: fragments should AND together to a single match"
    );
    let path = nodes[0].path.to_string_lossy().to_string();
    assert!(path.contains("Ayla"));
    assert!(path.contains("repos"));
    assert!(path.ends_with("main.js"));
}

#[test]
fn path_filter_without_word_matches_all_under_fragment() {
    let (mut cache, _root) = build_path_cache();

    let query = "path:repos";
    let result = cache
        .query_files(query, CancellationToken::noop())
        .expect("Query should succeed");
    let nodes = result.expect("Should return results");

    // Nodes whose path contains "repos": the repos dirs themselves plus their
    // contents -> repos, repos/main.js, Ayla/repos, Ayla/repos/main.js,
    // Ayla/repos/other.js (5).
    assert_eq!(
        nodes.len(),
        5,
        "path:repos should match dirs and files under repos"
    );
    for node in &nodes {
        assert!(node.path.to_string_lossy().contains("repos"));
    }
}

#[test]
fn path_filter_is_case_insensitive_when_enabled() {
    let (mut cache, _root) = build_path_cache();

    // With case-insensitive matching, lowercase "ayla" should match "Ayla".
    let query = "main.js path:ayla";
    let case_insensitive = SearchOptions {
        case_insensitive: true,
    };
    let result = cache
        .search_with_options(query, case_insensitive, CancellationToken::noop())
        .expect("Query should succeed");
    let nodes = result.nodes.expect("Should return results");
    let expanded = cache.expand_file_nodes(&nodes);

    assert_eq!(
        expanded.len(),
        2,
        "case-insensitive path:ayla should match Ayla"
    );
}

#[test]
fn path_filter_is_case_sensitive_by_default() {
    let (mut cache, _root) = build_path_cache();

    // query_files uses SearchOptions::default() which is case-sensitive, so
    // lowercase "ayla" must not match the "Ayla" directory.
    let query = "main.js path:ayla";
    let result = cache
        .query_files(query, CancellationToken::noop())
        .expect("Query should succeed");

    match result {
        None => {}
        Some(nodes) => assert!(
            nodes.is_empty(),
            "case-sensitive path:ayla should not match Ayla"
        ),
    }
}

#[test]
fn path_filter_strips_leading_slash() {
    let (mut cache, _root) = build_path_cache();

    // A leading slash is meaningless for a substring path filter; trim it so
    // "path:/Ayla" behaves the same as "path:Ayla".
    let query = "main.js path:/Ayla";
    let result = cache
        .query_files(query, CancellationToken::noop())
        .expect("Query should succeed");
    let nodes = result.expect("Should return results");

    assert_eq!(nodes.len(), 2, "leading slash should be ignored");
}

#[test]
fn path_filter_requires_argument() {
    let (mut cache, _root) = build_path_cache();

    let result = cache.query_files("main.js path:", CancellationToken::noop());
    assert!(result.is_err(), "path: without an argument should error");
}

#[test]
fn path_filter_uses_expanded_node_paths() {
    let (mut cache, _root) = build_path_cache();

    let query = "path:Ayla path:repos";
    let result = cache
        .query_files(query, CancellationToken::noop())
        .expect("Query should succeed");
    let nodes = result.expect("Should return results");

    // Ayla/repos, Ayla/repos/main.js, Ayla/repos/other.js (3)
    assert_eq!(nodes.len(), 3);
    for node in &nodes {
        let path = node.path.to_string_lossy();
        assert!(path.contains("Ayla"));
        assert!(path.contains("repos"));
    }
}
