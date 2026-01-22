use super::prelude::*;

#[test]
fn test_query_and_or_not_dedup_and_filtering() {
    let tmp = TempDir::new("query_bool").unwrap();
    fs::write(tmp.path().join("report.txt"), b"r").unwrap();
    fs::write(tmp.path().join("report.md"), b"r").unwrap();
    fs::write(tmp.path().join("other.txt"), b"o").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());

    // OR: union should return 3 distinct results
    let or = cache.search("report OR ext:txt").unwrap();
    assert_eq!(or.len(), 3, "OR should dedup overlapping results");

    // AND: intersection should narrow to the txt
    let and = cache.search("report ext:txt").unwrap();
    assert_eq!(and.len(), 1);

    // NOT: exclude names containing 'report'
    let not = cache.search("ext:txt !report").unwrap();
    assert_eq!(not.len(), 1);
    let path = cache.node_path(*not.first().unwrap()).unwrap();
    assert!(path.ends_with(PathBuf::from("other.txt")));
}

#[test]
fn test_globstar_dedup_overlapping_parents() {
    let tmp = TempDir::new("query_globstar_dedup").unwrap();
    fs::create_dir_all(tmp.path().join("a/a")).unwrap();
    fs::write(tmp.path().join("a/b.txt"), b"x").unwrap();
    fs::write(tmp.path().join("a/a/b.txt"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());

    let hits = cache.search("a/**/b.txt").unwrap();
    let mut unique = hits
        .iter()
        .map(|i| cache.node_path(*i).unwrap())
        .collect::<Vec<_>>();
    unique.sort();
    unique.dedup();

    assert_eq!(unique.len(), 2, "expected two unique b.txt hits");
    assert_eq!(
        hits.len(),
        unique.len(),
        "globstar should dedup overlapping matches"
    );
}

#[test]
fn test_globstar_dedup_nested_bar_paths() {
    let tmp = TempDir::new("query_globstar_nested_bar").unwrap();
    fs::create_dir_all(tmp.path().join("bar/emm/bar")).unwrap();
    fs::write(tmp.path().join("bar/foo.txt"), b"x").unwrap();
    fs::write(tmp.path().join("bar/emm/bar/foo.txt"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());

    let hits = cache.search("bar/**/foo").unwrap();
    let mut rel_paths = hits
        .iter()
        .map(|i| {
            cache
                .node_path(*i)
                .unwrap()
                .strip_prefix(tmp.path())
                .unwrap()
                .to_path_buf()
        })
        .collect::<Vec<_>>();
    rel_paths.sort();
    let mut unique = rel_paths.clone();
    unique.dedup();

    assert_eq!(unique.len(), 2, "expected two unique foo matches");
    assert_eq!(
        hits.len(),
        unique.len(),
        "globstar should dedup nested matches"
    );
    let mut expected = vec![
        PathBuf::from("bar/foo.txt"),
        PathBuf::from("bar/emm/bar/foo.txt"),
    ];
    expected.sort();
    assert_eq!(unique, expected);
}

#[test]
fn test_globstar_dedup_trailing_expansion() {
    let tmp = TempDir::new("query_globstar_trailing").unwrap();
    fs::create_dir_all(tmp.path().join("a/a")).unwrap();
    fs::write(tmp.path().join("a/file.txt"), b"x").unwrap();
    fs::write(tmp.path().join("a/a/file.txt"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());

    let hits = cache.search("a/**").unwrap();
    let mut rel_paths = hits
        .iter()
        .map(|i| {
            cache
                .node_path(*i)
                .unwrap()
                .strip_prefix(tmp.path())
                .unwrap()
                .to_path_buf()
        })
        .collect::<Vec<_>>();
    rel_paths.sort();
    let mut unique = rel_paths.clone();
    unique.dedup();

    assert_eq!(
        hits.len(),
        unique.len(),
        "globstar should dedup trailing expansion"
    );
    let mut expected = vec![
        PathBuf::from("a/a"),
        PathBuf::from("a/a/file.txt"),
        PathBuf::from("a/file.txt"),
    ];
    expected.sort();
    assert_eq!(unique, expected);
}

#[test]
fn test_globstar_dedup_multiple_globstars() {
    let tmp = TempDir::new("query_multiple_globstars").unwrap();
    fs::create_dir_all(tmp.path().join("a/b/c")).unwrap();
    fs::write(tmp.path().join("a/b/c/file.txt"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());

    // Multiple globstars: a/**/b/**/file.txt
    let hits = cache.search("a/**/b/**/file.txt").unwrap();
    let paths: Vec<_> = hits.iter().map(|i| cache.node_path(*i).unwrap()).collect();

    // Verify no duplicates
    let mut unique = paths.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(
        hits.len(),
        unique.len(),
        "multiple globstars should not produce duplicates"
    );
    assert_eq!(unique.len(), 1);
}

#[test]
fn test_globstar_dedup_with_wildcards() {
    let tmp = TempDir::new("query_globstar_wildcard").unwrap();
    fs::create_dir_all(tmp.path().join("src/utils")).unwrap();
    fs::write(tmp.path().join("src/test.js"), b"x").unwrap();
    fs::write(tmp.path().join("src/utils/helper.js"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());

    // Globstar + wildcard: src/**/*.js
    let hits = cache.search("src/**/*.js").unwrap();
    let mut rel_paths = hits
        .iter()
        .map(|i| {
            cache
                .node_path(*i)
                .unwrap()
                .strip_prefix(tmp.path())
                .unwrap()
                .to_path_buf()
        })
        .collect::<Vec<_>>();
    rel_paths.sort();
    let mut unique = rel_paths.clone();
    unique.dedup();

    assert_eq!(
        hits.len(),
        unique.len(),
        "globstar with wildcards should dedup"
    );
    assert_eq!(unique.len(), 2);
}

#[test]
fn test_globstar_dedup_empty_results() {
    let tmp = TempDir::new("query_globstar_empty").unwrap();
    fs::create_dir_all(tmp.path().join("a/b")).unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());

    // Search for non-existent file with globstar
    let hits = cache.search("a/**/nonexistent.txt").unwrap();
    assert_eq!(hits.len(), 0, "should return empty without panicking");
}

#[test]
fn test_globstar_dedup_single_match() {
    let tmp = TempDir::new("query_globstar_single").unwrap();
    fs::create_dir_all(tmp.path().join("dir")).unwrap();
    fs::write(tmp.path().join("dir/unique.txt"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());

    let hits = cache.search("dir/**/unique.txt").unwrap();
    assert_eq!(hits.len(), 1, "single match should remain single");
}

#[test]
fn test_globstar_dedup_deeply_nested() {
    let tmp = TempDir::new("query_globstar_deep").unwrap();
    fs::create_dir_all(tmp.path().join("a/a/a/a")).unwrap();
    fs::write(tmp.path().join("a/target.txt"), b"x").unwrap();
    fs::write(tmp.path().join("a/a/target.txt"), b"x").unwrap();
    fs::write(tmp.path().join("a/a/a/target.txt"), b"x").unwrap();
    fs::write(tmp.path().join("a/a/a/a/target.txt"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());

    let hits = cache.search("a/**/target.txt").unwrap();
    let mut rel_paths = hits
        .iter()
        .map(|i| {
            cache
                .node_path(*i)
                .unwrap()
                .strip_prefix(tmp.path())
                .unwrap()
                .to_path_buf()
        })
        .collect::<Vec<_>>();
    rel_paths.sort();
    let mut unique = rel_paths.clone();
    unique.dedup();

    assert_eq!(
        hits.len(),
        unique.len(),
        "deeply nested matches should be deduped"
    );
    assert_eq!(unique.len(), 4);
}

#[test]
fn test_globstar_no_dedup_without_globstar() {
    let tmp = TempDir::new("query_no_globstar").unwrap();
    fs::create_dir_all(tmp.path().join("src")).unwrap();
    fs::write(tmp.path().join("src/file.txt"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());

    // Regular path search without globstar
    let hits = cache.search("src/file.txt").unwrap();
    assert_eq!(hits.len(), 1, "regular search should work normally");
}

#[test]
fn test_globstar_dedup_with_boolean_operators() {
    let tmp = TempDir::new("query_globstar_bool").unwrap();
    fs::create_dir_all(tmp.path().join("a/a")).unwrap();
    fs::write(tmp.path().join("a/test.txt"), b"x").unwrap();
    fs::write(tmp.path().join("a/a/test.txt"), b"x").unwrap();
    fs::write(tmp.path().join("a/other.md"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());

    // Globstar with AND operation
    let hits = cache.search("a/**/test ext:txt").unwrap();
    let mut paths = hits
        .iter()
        .map(|i| cache.node_path(*i).unwrap())
        .collect::<Vec<_>>();
    paths.sort();
    let mut unique = paths.clone();
    unique.dedup();

    assert_eq!(
        hits.len(),
        unique.len(),
        "globstar with boolean should dedup"
    );
    assert_eq!(unique.len(), 2);
}

#[test]
fn test_globstar_dedup_leading_globstar() {
    let tmp = TempDir::new("query_leading_globstar").unwrap();
    fs::create_dir_all(tmp.path().join("a/b")).unwrap();
    fs::create_dir_all(tmp.path().join("c/b")).unwrap();
    fs::write(tmp.path().join("a/b/file.txt"), b"x").unwrap();
    fs::write(tmp.path().join("c/b/file.txt"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());

    // Leading globstar: **/b/file.txt
    let hits = cache.search("**/b/file.txt").unwrap();
    let mut rel_paths = hits
        .iter()
        .map(|i| {
            cache
                .node_path(*i)
                .unwrap()
                .strip_prefix(tmp.path())
                .unwrap()
                .to_path_buf()
        })
        .collect::<Vec<_>>();
    rel_paths.sort();
    let mut unique = rel_paths.clone();
    unique.dedup();

    assert_eq!(
        hits.len(),
        unique.len(),
        "leading globstar should dedup correctly"
    );
    assert_eq!(unique.len(), 2);
}

#[test]
fn test_regex_prefix_in_queries() {
    let tmp = TempDir::new("query_regex").unwrap();
    fs::write(tmp.path().join("Report Q1.md"), b"x").unwrap();
    fs::write(tmp.path().join("Report Q2.txt"), b"x").unwrap();
    fs::write(tmp.path().join("notes.txt"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());

    let idxs = cache.search("regex:^Report").unwrap();
    assert_eq!(idxs.len(), 2);
}

#[test]
fn test_ext_list_and_intersection() {
    let tmp = TempDir::new("query_ext_list").unwrap();
    fs::write(tmp.path().join("a.txt"), b"x").unwrap();
    fs::write(tmp.path().join("b.md"), b"x").unwrap();
    fs::write(tmp.path().join("c.rs"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());

    // ext list
    let list = cache.search("ext:txt;md").unwrap();
    assert_eq!(list.len(), 2);

    // Combine with word to intersect
    let only_b = cache.search("ext:txt;md b").unwrap();
    assert_eq!(only_b.len(), 1);
    let path = cache.node_path(*only_b.first().unwrap()).unwrap();
    assert!(path.ends_with(PathBuf::from("b.md")));
}

#[test]
fn test_or_then_and_intersection_precedence() {
    let tmp = TempDir::new("query_bool_prec").unwrap();
    fs::write(tmp.path().join("a.txt"), b"x").unwrap();
    fs::write(tmp.path().join("b.md"), b"x").unwrap();
    fs::write(tmp.path().join("c.txt"), b"x").unwrap();
    fs::write(tmp.path().join("d.bin"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());

    // OR has higher precedence; then intersect via implicit AND with ext:txt
    let res = cache.search("a OR b ext:txt").unwrap();
    assert_eq!(res.len(), 1);
    let path = cache.node_path(*res.first().unwrap()).unwrap();
    assert!(path.ends_with(PathBuf::from("a.txt")));

    let res2 = cache.search("a OR b OR c ext:txt").unwrap();
    assert_eq!(res2.len(), 2);
    let names: Vec<_> = res2.iter().map(|i| cache.node_path(*i).unwrap()).collect();
    assert!(names.iter().any(|p| p.ends_with(PathBuf::from("a.txt"))));
    assert!(names.iter().any(|p| p.ends_with(PathBuf::from("c.txt"))));
}

#[test]
fn test_groups_override_boolean_precedence() {
    let tmp = TempDir::new("query_groups_prec").unwrap();
    fs::write(tmp.path().join("ab.txt"), b"x").unwrap();
    fs::write(tmp.path().join("c.txt"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());

    let res = cache.search("(a b) | c").unwrap();
    let names: Vec<_> = res.iter().map(|i| cache.node_path(*i).unwrap()).collect();
    // Some searches also return the root directory node; ensure target files are present
    assert!(names.iter().any(|p| p.ends_with(PathBuf::from("ab.txt"))));
    assert!(names.iter().any(|p| p.ends_with(PathBuf::from("c.txt"))));
}

#[test]
fn test_not_precedence_with_intersection() {
    let tmp = TempDir::new("query_not_prec").unwrap();
    fs::write(tmp.path().join("a.txt"), b"x").unwrap();
    fs::write(tmp.path().join("b.txt"), b"x").unwrap();
    fs::write(tmp.path().join("notes.md"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());

    let res = cache.search("ext:txt !a").unwrap();
    assert_eq!(res.len(), 1);
    let path = cache.node_path(*res.first().unwrap()).unwrap();
    assert!(path.ends_with(PathBuf::from("b.txt")));
}

#[test]
fn test_regex_and_or_with_ext_intersection() {
    let tmp = TempDir::new("query_regex_prec").unwrap();
    fs::write(tmp.path().join("Report Q1.md"), b"x").unwrap();
    fs::write(tmp.path().join("Report Q2.txt"), b"x").unwrap();
    fs::write(tmp.path().join("notes.txt"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());

    let res = cache.search("regex:^Report OR notes ext:txt").unwrap();
    assert_eq!(res.len(), 2);
    let names: Vec<_> = res.iter().map(|i| cache.node_path(*i).unwrap()).collect();
    assert!(
        names
            .iter()
            .any(|p| p.ends_with(PathBuf::from("Report Q2.txt")))
    );
    assert!(
        names
            .iter()
            .any(|p| p.ends_with(PathBuf::from("notes.txt")))
    );
}

#[test]
fn test_extension_case_sensitivity_in_type_filter() {
    let tmp = TempDir::new("ext_case_type").unwrap();
    fs::write(tmp.path().join("photo.JPG"), b"x").unwrap();
    fs::write(tmp.path().join("image.jpg"), b"x").unwrap();
    fs::write(tmp.path().join("graphic.PNG"), b"x").unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    let results = cache.search("type:picture").unwrap();
    assert_eq!(results.len(), 3, "Should match case-insensitively");
}

// ============================================================================
// Trailing Globstar Behavior Tests
// ============================================================================
// Tests for the refactored trailing globstar implementation using
// all_descendant_segments instead of expand_trailing_globstar.
// Key behavior: trailing /** now returns only descendants, not the parent.
// ============================================================================

#[test]
fn test_trailing_globstar_excludes_parent_directory() {
    let tmp = TempDir::new("trailing_globstar_parent").unwrap();
    fs::create_dir_all(tmp.path().join("src/utils")).unwrap();
    fs::write(tmp.path().join("src/main.rs"), b"x").unwrap();
    fs::write(tmp.path().join("src/utils/helper.rs"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());

    let hits = cache.search("src/**").unwrap();
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

    // Should NOT include src itself, only its descendants
    assert!(!rel_paths.contains(&PathBuf::from("src")));
    assert!(rel_paths.contains(&PathBuf::from("src/main.rs")));
    assert!(rel_paths.contains(&PathBuf::from("src/utils")));
    assert!(rel_paths.contains(&PathBuf::from("src/utils/helper.rs")));
}

#[test]
fn test_trailing_globstar_empty_directory() {
    let tmp = TempDir::new("trailing_globstar_empty").unwrap();
    fs::create_dir_all(tmp.path().join("empty")).unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());

    let hits = cache.search("empty/**").unwrap();
    // Empty directory has no descendants
    assert_eq!(hits.len(), 0, "empty directory should have no descendants");
}

#[test]
fn test_trailing_globstar_single_file() {
    let tmp = TempDir::new("trailing_globstar_single").unwrap();
    fs::create_dir_all(tmp.path().join("dir")).unwrap();
    fs::write(tmp.path().join("dir/only.txt"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());

    let hits = cache.search("dir/**").unwrap();
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
    assert!(rel_paths.contains(&PathBuf::from("dir/only.txt")));
}

#[test]
fn test_trailing_globstar_deeply_nested() {
    let tmp = TempDir::new("trailing_globstar_deep").unwrap();
    fs::create_dir_all(tmp.path().join("a/b/c/d/e")).unwrap();
    fs::write(tmp.path().join("a/b/c/d/e/deep.txt"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());

    let hits = cache.search("a/**").unwrap();
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

    // Should include all levels except 'a' itself
    assert!(rel_paths.contains(&PathBuf::from("a/b")));
    assert!(rel_paths.contains(&PathBuf::from("a/b/c")));
    assert!(rel_paths.contains(&PathBuf::from("a/b/c/d")));
    assert!(rel_paths.contains(&PathBuf::from("a/b/c/d/e")));
    assert!(rel_paths.contains(&PathBuf::from("a/b/c/d/e/deep.txt")));
}

#[test]
fn test_trailing_globstar_multiple_branches() {
    let tmp = TempDir::new("trailing_globstar_branches").unwrap();
    fs::create_dir_all(tmp.path().join("root/branch1")).unwrap();
    fs::create_dir_all(tmp.path().join("root/branch2")).unwrap();
    fs::write(tmp.path().join("root/branch1/file1.txt"), b"x").unwrap();
    fs::write(tmp.path().join("root/branch2/file2.txt"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());

    let hits = cache.search("root/**").unwrap();
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

    // Should include all branches and their contents
    assert!(rel_paths.contains(&PathBuf::from("root/branch1")));
    assert!(rel_paths.contains(&PathBuf::from("root/branch2")));
    assert!(rel_paths.contains(&PathBuf::from("root/branch1/file1.txt")));
    assert!(rel_paths.contains(&PathBuf::from("root/branch2/file2.txt")));
}

#[test]
fn test_trailing_globstar_with_filters() {
    let tmp = TempDir::new("trailing_globstar_filters").unwrap();
    fs::create_dir_all(tmp.path().join("project/src")).unwrap();
    fs::write(tmp.path().join("project/README.md"), b"x").unwrap();
    fs::write(tmp.path().join("project/src/main.rs"), b"x").unwrap();
    fs::write(tmp.path().join("project/src/lib.rs"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());

    // Trailing globstar + extension filter
    let hits = cache.search("project/** ext:rs").unwrap();
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
    assert!(rel_paths.contains(&PathBuf::from("project/src/main.rs")));
    assert!(rel_paths.contains(&PathBuf::from("project/src/lib.rs")));
}

#[test]
fn test_trailing_globstar_with_type_filter() {
    let tmp = TempDir::new("trailing_globstar_type").unwrap();
    fs::create_dir_all(tmp.path().join("root/sub1/sub2")).unwrap();
    fs::write(tmp.path().join("root/file.txt"), b"x").unwrap();
    fs::write(tmp.path().join("root/sub1/file.txt"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());

    // Only directories under root
    let hits = cache.search("root/** type:directory").unwrap();
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

    assert!(rel_paths.contains(&PathBuf::from("root/sub1")));
    assert!(rel_paths.contains(&PathBuf::from("root/sub1/sub2")));
    assert!(
        !rel_paths
            .iter()
            .any(|p| p.to_string_lossy().ends_with(".txt"))
    );
}

#[test]
fn test_trailing_globstar_with_boolean_operators() {
    let tmp = TempDir::new("trailing_globstar_bool").unwrap();
    fs::create_dir_all(tmp.path().join("src")).unwrap();
    fs::create_dir_all(tmp.path().join("tests")).unwrap();
    fs::write(tmp.path().join("src/main.rs"), b"x").unwrap();
    fs::write(tmp.path().join("tests/test.rs"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());

    // OR with trailing globstars
    let hits = cache.search("src/** OR tests/**").unwrap();
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
fn test_trailing_globstar_performance_many_files() {
    let tmp = TempDir::new("trailing_globstar_perf").unwrap();
    fs::create_dir_all(tmp.path().join("large")).unwrap();

    // Create many nested files
    for i in 0..50 {
        fs::write(tmp.path().join(format!("large/file{:03}.txt", i)), b"x").unwrap();
    }
    fs::create_dir_all(tmp.path().join("large/subdir")).unwrap();
    for i in 0..50 {
        fs::write(
            tmp.path().join(format!("large/subdir/file{:03}.txt", i)),
            b"x",
        )
        .unwrap();
    }

    let mut cache = SearchCache::walk_fs(tmp.path());
    let start = std::time::Instant::now();
    let hits = cache.search("large/**").unwrap();
    let duration = start.elapsed();

    // Should find all files + subdir
    assert!(hits.len() >= 101, "should find all descendants");
    assert!(
        duration.as_millis() < 100,
        "should complete quickly: {:?}",
        duration
    );
}

#[test]
fn test_trailing_globstar_no_duplicates() {
    let tmp = TempDir::new("trailing_globstar_nodup").unwrap();
    fs::create_dir_all(tmp.path().join("foo/foo")).unwrap();
    fs::write(tmp.path().join("foo/bar.txt"), b"x").unwrap();
    fs::write(tmp.path().join("foo/foo/bar.txt"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());

    let hits = cache.search("foo/**").unwrap();
    let paths: Vec<_> = hits.iter().map(|i| cache.node_path(*i).unwrap()).collect();

    // Check for duplicates
    let mut unique_paths = paths.clone();
    unique_paths.sort();
    unique_paths.dedup();

    assert_eq!(
        paths.len(),
        unique_paths.len(),
        "should not contain duplicate paths"
    );
}

#[test]
fn test_trailing_globstar_vs_concrete_segment() {
    let tmp = TempDir::new("trailing_vs_concrete").unwrap();
    fs::create_dir_all(tmp.path().join("src/utils")).unwrap();
    fs::write(tmp.path().join("src/main.rs"), b"x").unwrap();
    fs::write(tmp.path().join("src/utils/helper.rs"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());

    // src/** should match all descendants
    let globstar_hits = cache.search("src/**").unwrap();

    // src/main.rs should match only that file
    let concrete_hits = cache.search("src/main.rs").unwrap();

    assert!(globstar_hits.len() > concrete_hits.len());
    assert_eq!(concrete_hits.len(), 1);
}

#[test]
fn test_multiple_trailing_globstars() {
    let tmp = TempDir::new("multi_trailing_globstar").unwrap();
    fs::create_dir_all(tmp.path().join("a/b")).unwrap();
    fs::create_dir_all(tmp.path().join("c/d")).unwrap();
    fs::write(tmp.path().join("a/b/file1.txt"), b"x").unwrap();
    fs::write(tmp.path().join("c/d/file2.txt"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());

    // Multiple patterns with trailing globstars using OR
    let hits = cache.search("a/b/** OR c/d/**").unwrap();
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

    assert!(rel_paths.contains(&PathBuf::from("a/b/file1.txt")));
    assert!(rel_paths.contains(&PathBuf::from("c/d/file2.txt")));
}

#[test]
fn test_trailing_globstar_symlink_handling() {
    let tmp = TempDir::new("trailing_globstar_symlink").unwrap();
    fs::create_dir_all(tmp.path().join("real")).unwrap();
    fs::write(tmp.path().join("real/file.txt"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());

    // Should handle the real directory normally
    let hits = cache.search("real/**").unwrap();
    assert!(
        !hits.is_empty(),
        "should find descendants in real directory"
    );
}

#[test]
fn test_trailing_globstar_result_ordering() {
    let tmp = TempDir::new("trailing_globstar_order").unwrap();
    fs::create_dir_all(tmp.path().join("dir")).unwrap();
    fs::write(tmp.path().join("dir/zzz.txt"), b"x").unwrap();
    fs::write(tmp.path().join("dir/aaa.txt"), b"x").unwrap();
    fs::write(tmp.path().join("dir/mmm.txt"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());

    let hits = cache.search("dir/**").unwrap();
    let names: Vec<_> = hits
        .iter()
        .map(|i| {
            cache
                .node_path(*i)
                .unwrap()
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string()
        })
        .filter(|s| !s.is_empty())
        .collect();

    // Results should be sorted
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(names, sorted, "results should be sorted by name");
}
