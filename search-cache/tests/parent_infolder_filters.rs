//! Tests for parent: and infolder: filter improvements in commit a688ace.
//! Verifies the optimized implementation that directly accesses child nodes
//! instead of scanning the entire file tree.

use search_cache::SearchCache;
use search_cancel::CancellationToken;
use std::path::PathBuf;
use tempdir::TempDir;

/// Build a test cache with nested directory structure.
fn build_nested_cache() -> (SearchCache, PathBuf) {
    let temp_dir = TempDir::new("parent_infolder_test").unwrap();
    let root_path = temp_dir.path().to_path_buf();
    // Leak the TempDir so files remain accessible.
    std::mem::forget(temp_dir);

    // Create nested structure:
    // root/
    //   file1.txt
    //   src/
    //     main.rs
    //     lib.rs
    //     utils/
    //       helper.rs
    //       config.rs
    //   tests/
    //     test1.rs
    //   docs/
    //     readme.md
    let files = [
        "file1.txt",
        "src/main.rs",
        "src/lib.rs",
        "src/utils/helper.rs",
        "src/utils/config.rs",
        "tests/test1.rs",
        "docs/readme.md",
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
fn test_parent_filter_direct_children() {
    let (mut cache, root) = build_nested_cache();

    // Test parent:src - should return only direct children of src
    let src_path = root.join("src");
    let query = format!("parent:{}", src_path.display());

    let result = cache
        .query_files(query, CancellationToken::noop())
        .expect("Query should succeed");

    let nodes = result.expect("Should return results");

    // Should only find main.rs, lib.rs, and utils directory (direct children)
    assert_eq!(nodes.len(), 3, "parent:src should return 3 direct children");

    let paths: Vec<PathBuf> = nodes.iter().map(|node| node.path.clone()).collect();

    // Verify all paths are direct children of src
    for path in &paths {
        assert_eq!(
            path.parent().unwrap(),
            src_path,
            "All results should be direct children of src"
        );
    }
}

#[test]
fn test_infolder_filter_recursive() {
    let (mut cache, root) = build_nested_cache();

    // Test infolder:src - should return all descendants recursively
    let src_path = root.join("src");
    let query = format!("infolder:{}", src_path.display());

    let result = cache
        .query_files(query, CancellationToken::noop())
        .expect("Query should succeed");

    let nodes = result.expect("Should return results");

    // Should find: main.rs, lib.rs, utils/, helper.rs, config.rs (5 items)
    assert_eq!(
        nodes.len(),
        5,
        "infolder:src should return 5 descendants (including nested)"
    );

    let paths: Vec<PathBuf> = nodes.iter().map(|node| node.path.clone()).collect();

    // Verify all paths are under src
    for path in &paths {
        assert!(
            path.starts_with(&src_path),
            "All results should be under src directory"
        );
    }
}

#[test]
fn test_parent_filter_with_pattern() {
    let (mut cache, root) = build_nested_cache();

    // Test parent:src *.rs - should find only .rs files directly under src
    let src_path = root.join("src");
    let query = format!("parent:{} *.rs", src_path.display());

    let result = cache
        .query_files(query, CancellationToken::noop())
        .expect("Query should succeed");

    let nodes = result.expect("Should return results");

    // Should only find main.rs and lib.rs (not helper.rs or config.rs which are in utils)
    assert_eq!(
        nodes.len(),
        2,
        "parent:src *.rs should return 2 direct .rs files"
    );

    let paths: Vec<PathBuf> = nodes.iter().map(|node| node.path.clone()).collect();

    for path in &paths {
        assert_eq!(path.parent().unwrap(), src_path);
        assert_eq!(path.extension().and_then(|s| s.to_str()), Some("rs"));
    }
}

#[test]
fn test_infolder_filter_with_pattern() {
    let (mut cache, root) = build_nested_cache();

    // Test infolder:src *.rs - should find all .rs files recursively under src
    let src_path = root.join("src");
    let query = format!("infolder:{} *.rs", src_path.display());

    let result = cache
        .query_files(query, CancellationToken::noop())
        .expect("Query should succeed");

    let nodes = result.expect("Should return results");

    // Should find: main.rs, lib.rs, helper.rs, config.rs (4 files)
    assert_eq!(
        nodes.len(),
        4,
        "infolder:src *.rs should return 4 .rs files recursively"
    );

    let paths: Vec<PathBuf> = nodes.iter().map(|node| node.path.clone()).collect();

    for path in &paths {
        assert!(path.starts_with(&src_path));
        assert_eq!(path.extension().and_then(|s| s.to_str()), Some("rs"));
    }
}

#[test]
fn test_parent_filter_nonexistent_path() {
    let (mut cache, root) = build_nested_cache();

    // Test parent with non-existent directory
    let nonexistent = root.join("nonexistent");
    let query = format!("parent:{}", nonexistent.display());

    let result = cache.query_files(query, CancellationToken::noop());

    // Should return error for non-existent path
    assert!(result.is_err(), "Should error for non-existent parent path");
}

#[test]
fn test_infolder_filter_nonexistent_path() {
    let (mut cache, root) = build_nested_cache();

    // Test infolder with non-existent directory
    let nonexistent = root.join("nonexistent");
    let query = format!("infolder:{}", nonexistent.display());

    let result = cache.query_files(query, CancellationToken::noop());

    // Should return error for non-existent path
    assert!(
        result.is_err(),
        "Should error for non-existent infolder path"
    );
}

#[test]
fn test_parent_filter_root() {
    let (mut cache, root) = build_nested_cache();

    // Test parent at root level
    let query = format!("parent:{}", root.display());

    let result = cache
        .query_files(query, CancellationToken::noop())
        .expect("Query should succeed");

    let nodes = result.expect("Should return results");

    // Should find: file1.txt, src/, tests/, docs/ (4 direct children)
    assert_eq!(nodes.len(), 4, "parent at root should return 4 items");
}

#[test]
fn test_infolder_filter_empty_directory() {
    let temp_dir = TempDir::new("empty_dir_test").unwrap();
    let root_path = temp_dir.path().to_path_buf();
    std::mem::forget(temp_dir);

    // Create empty directory
    let empty_dir = root_path.join("empty");
    std::fs::create_dir_all(&empty_dir).unwrap();

    let mut cache = SearchCache::walk_fs(&root_path);

    // Test infolder on empty directory
    let query = format!("infolder:{}", empty_dir.display());

    let result = cache
        .query_files(query, CancellationToken::noop())
        .expect("Query should succeed");

    // Should return empty or None for directory with no children
    match result {
        None => {} // Acceptable: no results
        Some(nodes) => assert_eq!(nodes.len(), 0, "Empty directory should have no subnodes"),
    }
}

#[test]
fn test_parent_infolder_difference() {
    let (mut cache, root) = build_nested_cache();

    let src_path = root.join("src");

    // Get parent results
    let parent_query = format!("parent:{}", src_path.display());
    let parent_result = cache
        .query_files(parent_query, CancellationToken::noop())
        .expect("Query should succeed")
        .expect("Should return results");

    // Get infolder results
    let infolder_query = format!("infolder:{}", src_path.display());
    let infolder_result = cache
        .query_files(infolder_query, CancellationToken::noop())
        .expect("Query should succeed")
        .expect("Should return results");

    // infolder should return more items than parent (includes nested files)
    assert!(
        infolder_result.len() > parent_result.len(),
        "infolder should return more results than parent for directories with subdirectories"
    );

    // parent results should be a subset of infolder results
    let parent_paths: Vec<PathBuf> = parent_result.iter().map(|node| node.path.clone()).collect();

    let infolder_paths: Vec<PathBuf> = infolder_result
        .iter()
        .map(|node| node.path.clone())
        .collect();

    for parent_path in &parent_paths {
        assert!(
            infolder_paths.contains(parent_path),
            "All parent results should be included in infolder results"
        );
    }
}

#[test]
fn test_parent_filter_path_validation() {
    let (mut cache, _root) = build_nested_cache();

    // Test with path outside the indexed root (should error)
    let outside_path = PathBuf::from("/some/random/path");
    let query = format!("parent:{}", outside_path.display());

    let result = cache.query_files(query, CancellationToken::noop());

    assert!(
        result.is_err(),
        "Should error for path outside indexed root"
    );
}

#[test]
fn test_infolder_filter_path_validation() {
    let (mut cache, _root) = build_nested_cache();

    // Test with path outside the indexed root (should error)
    let outside_path = PathBuf::from("/some/random/path");
    let query = format!("infolder:{}", outside_path.display());

    let result = cache.query_files(query, CancellationToken::noop());

    assert!(
        result.is_err(),
        "Should error for path outside indexed root"
    );
}

#[test]
fn test_parent_with_boolean_operators() {
    let (mut cache, root) = build_nested_cache();

    let src_path = root.join("src");
    let tests_path = root.join("tests");

    // Test parent:src | parent:tests (union)
    let query = format!(
        "parent:{} | parent:{}",
        src_path.display(),
        tests_path.display()
    );

    let result = cache
        .query_files(query, CancellationToken::noop())
        .expect("Query should succeed");

    let nodes = result.expect("Should return results");

    // Should find children of both src and tests
    // src has 3 children (main.rs, lib.rs, utils), tests has 1 (test1.rs)
    assert_eq!(
        nodes.len(),
        4,
        "Union should combine results from both parents"
    );
}

#[test]
fn test_infolder_with_negation() {
    let (mut cache, root) = build_nested_cache();

    let src_path = root.join("src");

    // Test infolder:src ! ext:rs (all items in src except .rs files)
    let query = format!("infolder:{} ! ext:rs", src_path.display());

    let result = cache
        .query_files(query, CancellationToken::noop())
        .expect("Query should succeed");

    let nodes = result.expect("Should return results");

    // Should only find utils/ directory (not the .rs files)
    assert_eq!(nodes.len(), 1, "Should exclude all .rs files");

    let paths: Vec<PathBuf> = nodes.iter().map(|node| node.path.clone()).collect();

    for path in &paths {
        assert_ne!(path.extension().and_then(|s| s.to_str()), Some("rs"));
    }
}
