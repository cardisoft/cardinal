use super::prelude::*;

// ============================================================================
// Single-Star Wildcard Regression Tests
// ============================================================================
// These tests verify the correctness and performance optimization of the
// single-star wildcard (`*`) operator, which matches exactly one path segment.
// The optimization (commit: opt(query): speed up single-star wildcard)
// introduced a fast path for `*` to avoid treating it as a generic wildcard.
//
// Key behaviors to maintain:
// - `*` matches any single segment (file or directory)
// - `*` does NOT match across directory boundaries (unlike `**`)
// - `*` can be combined with other path segments
// - `*` can appear multiple times in a query
// - `*` interacts correctly with filters (ext:, type:, etc.)
// - `*` interacts correctly with boolean operators (AND, OR, NOT)
// ============================================================================

// --- Basic Single-Star Matching ---

#[test]
fn star_matches_single_segment() {
    let tmp = TempDir::new("star_single").unwrap();
    fs::create_dir_all(tmp.path().join("dir")).unwrap();
    fs::write(tmp.path().join("file.txt"), b"x").unwrap();
    fs::write(tmp.path().join("dir/nested.txt"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());

    // Single star at root matches everything (acts like search_empty)
    let hits = cache.search("*").unwrap();

    // When * is standalone, it returns all nodes
    assert!(hits.len() >= 3); // dir, file.txt, and nested.txt at minimum
}

#[test]
fn star_does_not_cross_directory_boundaries() {
    let tmp = TempDir::new("star_boundary").unwrap();
    fs::create_dir_all(tmp.path().join("a/b/c")).unwrap();
    fs::write(tmp.path().join("a/file.txt"), b"x").unwrap();
    fs::write(tmp.path().join("a/b/file.txt"), b"x").unwrap();
    fs::write(tmp.path().join("a/b/c/file.txt"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());

    // a/* should only match direct children of a/
    let hits = cache.search("a/*").unwrap();
    let rel_paths: Vec<_> = hits
        .iter()
        .map(|i| {
            cache
                .node_path(*i)
                .unwrap()
                .strip_prefix(tmp.path())
                .unwrap()
                .to_path_buf()
        })
        .collect();

    assert!(rel_paths.contains(&PathBuf::from("a/file.txt")));
    assert!(rel_paths.contains(&PathBuf::from("a/b")));
    // Should NOT match deeper files
    assert!(!rel_paths.contains(&PathBuf::from("a/b/file.txt")));
    assert!(!rel_paths.contains(&PathBuf::from("a/b/c/file.txt")));
}

#[test]
fn star_with_prefix_segment() {
    let tmp = TempDir::new("star_prefix").unwrap();
    fs::create_dir_all(tmp.path().join("src/utils")).unwrap();
    fs::write(tmp.path().join("src/main.rs"), b"x").unwrap();
    fs::write(tmp.path().join("src/lib.rs"), b"x").unwrap();
    fs::write(tmp.path().join("src/utils/helper.rs"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());

    // src/* should match all direct children of src/
    let hits = cache.search("src/*").unwrap();
    let rel_paths: Vec<_> = hits
        .iter()
        .map(|i| {
            cache
                .node_path(*i)
                .unwrap()
                .strip_prefix(tmp.path())
                .unwrap()
                .to_path_buf()
        })
        .collect();

    assert!(rel_paths.contains(&PathBuf::from("src/main.rs")));
    assert!(rel_paths.contains(&PathBuf::from("src/lib.rs")));
    assert!(rel_paths.contains(&PathBuf::from("src/utils")));
    assert!(!rel_paths.contains(&PathBuf::from("src/utils/helper.rs")));
}

#[test]
fn star_with_suffix_segment() {
    let tmp = TempDir::new("star_suffix").unwrap();
    fs::create_dir_all(tmp.path().join("foo/bar")).unwrap();
    fs::create_dir_all(tmp.path().join("baz/bar")).unwrap();
    fs::write(tmp.path().join("foo/bar/test.txt"), b"x").unwrap();
    fs::write(tmp.path().join("baz/bar/test.txt"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());

    // */bar should match bar directories under any parent
    let hits = cache.search("*/bar").unwrap();
    let rel_paths: Vec<_> = hits
        .iter()
        .map(|i| {
            cache
                .node_path(*i)
                .unwrap()
                .strip_prefix(tmp.path())
                .unwrap()
                .to_path_buf()
        })
        .collect();

    assert!(rel_paths.contains(&PathBuf::from("foo/bar")));
    assert!(rel_paths.contains(&PathBuf::from("baz/bar")));
}

// --- Multiple Stars ---

#[test]
fn multiple_stars_in_sequence() {
    let tmp = TempDir::new("multi_star").unwrap();
    fs::create_dir_all(tmp.path().join("a/b/c")).unwrap();
    fs::write(tmp.path().join("a/b/c/file.txt"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());

    // a/*/*/file.txt should match exactly three segments deep
    let hits = cache.search("a/*/*/file.txt").unwrap();
    let rel_paths: Vec<_> = hits
        .iter()
        .map(|i| {
            cache
                .node_path(*i)
                .unwrap()
                .strip_prefix(tmp.path())
                .unwrap()
                .to_path_buf()
        })
        .collect();

    assert_eq!(rel_paths.len(), 1);
    assert!(rel_paths.contains(&PathBuf::from("a/b/c/file.txt")));
}

#[test]
fn star_at_different_positions() {
    let tmp = TempDir::new("star_positions").unwrap();
    fs::create_dir_all(tmp.path().join("src/components")).unwrap();
    fs::create_dir_all(tmp.path().join("tests/components")).unwrap();
    fs::write(tmp.path().join("src/components/Button.tsx"), b"x").unwrap();
    fs::write(tmp.path().join("tests/components/Button.test.tsx"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());

    // */components/Button* should match both
    let hits = cache.search("*/components/Button").unwrap();
    let rel_paths: Vec<_> = hits
        .iter()
        .map(|i| {
            cache
                .node_path(*i)
                .unwrap()
                .strip_prefix(tmp.path())
                .unwrap()
                .to_path_buf()
        })
        .collect();

    assert!(
        rel_paths
            .iter()
            .any(|p| p.to_string_lossy().contains("src/components"))
    );
    assert!(
        rel_paths
            .iter()
            .any(|p| p.to_string_lossy().contains("tests/components"))
    );
}

// --- Star vs Globstar Comparison ---

#[test]
fn star_vs_globstar_difference() {
    let tmp = TempDir::new("star_vs_globstar").unwrap();
    fs::create_dir_all(tmp.path().join("a/b/c")).unwrap();
    fs::write(tmp.path().join("a/file.txt"), b"x").unwrap();
    fs::write(tmp.path().join("a/b/file.txt"), b"x").unwrap();
    fs::write(tmp.path().join("a/b/c/file.txt"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());

    // a/*/file.txt should only match one level deep
    let star_hits = cache.search("a/*/file.txt").unwrap();
    assert_eq!(star_hits.len(), 1, "single star should match one level");

    // a/**/file.txt should match all levels
    let globstar_hits = cache.search("a/**/file.txt").unwrap();
    assert_eq!(globstar_hits.len(), 3, "globstar should match all levels");
}

#[test]
fn star_then_globstar() {
    let tmp = TempDir::new("star_then_globstar").unwrap();
    fs::create_dir_all(tmp.path().join("src/modules/auth/utils")).unwrap();
    fs::write(tmp.path().join("src/modules/auth/login.ts"), b"x").unwrap();
    fs::write(tmp.path().join("src/modules/auth/utils/hash.ts"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());

    // src/*/auth/** should match auth under any direct child of src, then everything under auth
    let hits = cache.search("src/*/auth/**").unwrap();
    let rel_paths: Vec<_> = hits
        .iter()
        .map(|i| {
            cache
                .node_path(*i)
                .unwrap()
                .strip_prefix(tmp.path())
                .unwrap()
                .to_path_buf()
        })
        .collect();

    assert!(
        rel_paths
            .iter()
            .any(|p| p == &PathBuf::from("src/modules/auth"))
    );
    assert!(
        rel_paths
            .iter()
            .any(|p| p == &PathBuf::from("src/modules/auth/login.ts"))
    );
    assert!(
        rel_paths
            .iter()
            .any(|p| p == &PathBuf::from("src/modules/auth/utils/hash.ts"))
    );
}

#[test]
fn globstar_then_star() {
    let tmp = TempDir::new("globstar_then_star").unwrap();
    fs::create_dir_all(tmp.path().join("a/b/c")).unwrap();
    fs::write(tmp.path().join("a/b/file.txt"), b"x").unwrap();
    fs::write(tmp.path().join("a/b/c/file.txt"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());

    // a/**/*/file.txt - globstar to any depth, then exactly one more segment
    let hits = cache.search("a/**/*/file.txt").unwrap();
    let rel_paths: Vec<_> = hits
        .iter()
        .map(|i| {
            cache
                .node_path(*i)
                .unwrap()
                .strip_prefix(tmp.path())
                .unwrap()
                .to_path_buf()
        })
        .collect();

    // Should match both since globstar can match zero segments
    assert!(rel_paths.contains(&PathBuf::from("a/b/file.txt")));
    assert!(rel_paths.contains(&PathBuf::from("a/b/c/file.txt")));
}

// --- Star with Filters ---

#[test]
fn star_with_extension_filter() {
    let tmp = TempDir::new("star_ext").unwrap();
    fs::create_dir_all(tmp.path().join("src")).unwrap();
    fs::write(tmp.path().join("src/main.rs"), b"x").unwrap();
    fs::write(tmp.path().join("src/lib.rs"), b"x").unwrap();
    fs::write(tmp.path().join("src/test.txt"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());

    // src/* with ext:rs filter
    let hits = cache.search("src/* ext:rs").unwrap();
    let rel_paths: Vec<_> = hits
        .iter()
        .map(|i| {
            cache
                .node_path(*i)
                .unwrap()
                .strip_prefix(tmp.path())
                .unwrap()
                .to_path_buf()
        })
        .collect();

    assert_eq!(rel_paths.len(), 2);
    assert!(rel_paths.contains(&PathBuf::from("src/main.rs")));
    assert!(rel_paths.contains(&PathBuf::from("src/lib.rs")));
    assert!(!rel_paths.contains(&PathBuf::from("src/test.txt")));
}

#[test]
fn star_with_type_filter() {
    let tmp = TempDir::new("star_type").unwrap();
    fs::create_dir_all(tmp.path().join("dir1")).unwrap();
    fs::create_dir_all(tmp.path().join("dir2")).unwrap();
    fs::write(tmp.path().join("file1.txt"), b"x").unwrap();
    fs::write(tmp.path().join("file2.txt"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());

    // * with type:directory filter
    let hits = cache.search("* type:directory").unwrap();
    let paths: Vec<_> = hits.iter().map(|i| cache.node_path(*i).unwrap()).collect();

    let dir1 = tmp.path().join("dir1");
    let dir2 = tmp.path().join("dir2");
    let file1 = tmp.path().join("file1.txt");
    let file2 = tmp.path().join("file2.txt");

    assert!(paths.contains(&dir1));
    assert!(paths.contains(&dir2));
    assert!(!paths.contains(&file1));
    assert!(!paths.contains(&file2));
}

#[test]
fn star_with_size_filter() {
    let tmp = TempDir::new("star_size").unwrap();
    fs::create_dir_all(tmp.path().join("data")).unwrap();
    fs::write(tmp.path().join("data/small.txt"), b"x").unwrap();
    fs::write(tmp.path().join("data/large.txt"), b"x".repeat(1000)).unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());

    // data/* with size filter
    let hits = cache.search("data/* size:>100b").unwrap();
    let rel_paths: Vec<_> = hits
        .iter()
        .map(|i| {
            cache
                .node_path(*i)
                .unwrap()
                .strip_prefix(tmp.path())
                .unwrap()
                .to_path_buf()
        })
        .collect();

    assert_eq!(rel_paths.len(), 1);
    assert!(rel_paths.contains(&PathBuf::from("data/large.txt")));
}

// --- Star with Boolean Operators ---

#[test]
fn star_with_or_operator() {
    let tmp = TempDir::new("star_or").unwrap();
    fs::create_dir_all(tmp.path().join("src")).unwrap();
    fs::create_dir_all(tmp.path().join("tests")).unwrap();
    fs::write(tmp.path().join("src/main.rs"), b"x").unwrap();
    fs::write(tmp.path().join("tests/test.rs"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());

    // src/* OR tests/*
    let hits = cache.search("src/* OR tests/*").unwrap();
    let rel_paths: Vec<_> = hits
        .iter()
        .map(|i| {
            cache
                .node_path(*i)
                .unwrap()
                .strip_prefix(tmp.path())
                .unwrap()
                .to_path_buf()
        })
        .collect();

    assert!(rel_paths.contains(&PathBuf::from("src/main.rs")));
    assert!(rel_paths.contains(&PathBuf::from("tests/test.rs")));
}

#[test]
fn star_with_not_operator() {
    let tmp = TempDir::new("star_not").unwrap();
    fs::create_dir_all(tmp.path().join("src")).unwrap();
    fs::write(tmp.path().join("src/main.rs"), b"x").unwrap();
    fs::write(tmp.path().join("src/test.rs"), b"x").unwrap();
    fs::write(tmp.path().join("src/lib.rs"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());

    // src/* but not test files
    let hits = cache.search("src/* !test").unwrap();
    let rel_paths: Vec<_> = hits
        .iter()
        .map(|i| {
            cache
                .node_path(*i)
                .unwrap()
                .strip_prefix(tmp.path())
                .unwrap()
                .to_path_buf()
        })
        .collect();

    assert!(rel_paths.contains(&PathBuf::from("src/main.rs")));
    assert!(rel_paths.contains(&PathBuf::from("src/lib.rs")));
    assert!(!rel_paths.contains(&PathBuf::from("src/test.rs")));
}

// --- Edge Cases ---

#[test]
fn star_matches_empty_directory() {
    let tmp = TempDir::new("star_empty").unwrap();
    fs::create_dir_all(tmp.path().join("empty")).unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());

    let hits = cache.search("*").unwrap();
    let paths: Vec<_> = hits.iter().map(|i| cache.node_path(*i).unwrap()).collect();

    let empty_dir = tmp.path().join("empty");
    assert!(paths.contains(&empty_dir));
}

#[test]
fn star_with_special_characters_in_names() {
    let tmp = TempDir::new("star_special").unwrap();
    fs::create_dir_all(tmp.path().join("src")).unwrap();
    fs::write(tmp.path().join("src/file-name.txt"), b"x").unwrap();
    fs::write(tmp.path().join("src/file_name.txt"), b"x").unwrap();
    fs::write(tmp.path().join("src/file.name.txt"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());

    let hits = cache.search("src/*").unwrap();
    assert!(hits.len() >= 3, "should match files with special chars");
}

#[test]
fn star_only_query() {
    let tmp = TempDir::new("star_only").unwrap();
    fs::write(tmp.path().join("a.txt"), b"x").unwrap();
    fs::write(tmp.path().join("b.txt"), b"x").unwrap();
    fs::create_dir_all(tmp.path().join("dir")).unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());

    let hits = cache.search("*").unwrap();
    assert!(hits.len() >= 3, "star-only should match all root items");
}

#[test]
fn trailing_star() {
    let tmp = TempDir::new("trailing_star").unwrap();
    fs::create_dir_all(tmp.path().join("src/utils")).unwrap();
    fs::write(tmp.path().join("src/utils/helper.rs"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());

    // src/utils/* should match children of utils
    let hits = cache.search("src/utils/*").unwrap();
    let rel_paths: Vec<_> = hits
        .iter()
        .map(|i| {
            cache
                .node_path(*i)
                .unwrap()
                .strip_prefix(tmp.path())
                .unwrap()
                .to_path_buf()
        })
        .collect();

    assert!(rel_paths.contains(&PathBuf::from("src/utils/helper.rs")));
}

// --- Performance Regression Guards ---

#[test]
fn star_performance_many_siblings() {
    // Ensure star can handle directories with many children efficiently
    let tmp = TempDir::new("star_perf").unwrap();
    fs::create_dir_all(tmp.path().join("large")).unwrap();

    for i in 0..100 {
        fs::write(tmp.path().join(format!("large/file{:03}.txt", i)), b"x").unwrap();
    }

    let mut cache = SearchCache::walk_fs(tmp.path());
    let start = std::time::Instant::now();
    let hits = cache.search("large/*").unwrap();
    let duration = start.elapsed();

    assert_eq!(hits.len(), 100);
    // Should complete in reasonable time (< 100ms for 100 files)
    assert!(
        duration.as_millis() < 100,
        "star search took too long: {:?}",
        duration
    );
}

#[test]
fn star_no_false_positives() {
    let tmp = TempDir::new("star_false_pos").unwrap();
    fs::create_dir_all(tmp.path().join("app/models")).unwrap();
    fs::create_dir_all(tmp.path().join("app/views")).unwrap();
    fs::write(tmp.path().join("app/models/user.rb"), b"x").unwrap();
    fs::write(tmp.path().join("app/views/index.html"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());

    // app/models/* should not match views
    let hits = cache.search("app/models/*").unwrap();
    let rel_paths: Vec<_> = hits
        .iter()
        .map(|i| {
            cache
                .node_path(*i)
                .unwrap()
                .strip_prefix(tmp.path())
                .unwrap()
                .to_path_buf()
        })
        .collect();

    assert!(rel_paths.contains(&PathBuf::from("app/models/user.rb")));
    assert!(
        !rel_paths
            .iter()
            .any(|p| p.to_string_lossy().contains("views"))
    );
}

// --- Interaction with Existing Features ---

#[test]
fn star_preserves_result_ordering() {
    let tmp = TempDir::new("star_order").unwrap();
    fs::create_dir_all(tmp.path().join("src")).unwrap();
    fs::write(tmp.path().join("src/aaa.rs"), b"x").unwrap();
    fs::write(tmp.path().join("src/bbb.rs"), b"x").unwrap();
    fs::write(tmp.path().join("src/ccc.rs"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());

    let hits = cache.search("src/*").unwrap();
    let names: Vec<_> = hits
        .iter()
        .map(|i| {
            cache
                .node_path(*i)
                .unwrap()
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string()
        })
        .collect();

    // Results should be sorted by name
    let mut sorted_names = names.clone();
    sorted_names.sort();
    assert_eq!(names, sorted_names, "results should be sorted by name");
}

#[test]
fn star_with_case_sensitivity() {
    let tmp = TempDir::new("star_case").unwrap();
    fs::create_dir_all(tmp.path().join("src")).unwrap();
    fs::write(tmp.path().join("src/File.txt"), b"x").unwrap();
    fs::write(tmp.path().join("src/file.txt"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());

    // Should match files (note: on case-insensitive filesystems like macOS default,
    // File.txt and file.txt may be the same file)
    let hits = cache.search("src/*").unwrap();
    assert!(!hits.is_empty(), "should match at least one file");
}
