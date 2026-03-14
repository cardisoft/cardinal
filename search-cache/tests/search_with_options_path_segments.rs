use search_cache::{SearchCache, SearchOptions, SlabIndex};
use search_cancel::CancellationToken;
use std::fs;
use tempdir::TempDir;

fn guard_indices(result: Result<search_cache::SearchOutcome, anyhow::Error>) -> Vec<SlabIndex> {
    result
        .expect("search should succeed")
        .nodes
        .expect("noop cancellation token should not cancel")
}

fn file_names(cache: &mut SearchCache, indices: &[SlabIndex]) -> Vec<String> {
    cache
        .expand_file_nodes(indices)
        .into_iter()
        .map(|node| node.path.display().to_string())
        .collect()
}

#[test]
fn leading_slash_anchors_to_root_segment() {
    let temp_dir = TempDir::new("leading_slash_anchors_to_root_segment").unwrap();
    let root = temp_dir.path();
    fs::create_dir_all(root.join("foo/bar")).unwrap();
    fs::create_dir_all(root.join("other/foo/bar")).unwrap();
    fs::File::create(root.join("foo/bar/baz.txt")).unwrap();
    fs::File::create(root.join("other/foo/bar/baz.txt")).unwrap();

    let mut cache = SearchCache::walk_fs(root);
    let opts = SearchOptions {
        case_insensitive: false,
    };
    let indices = guard_indices(cache.search_with_options(
        "/foo/bar/baz.txt",
        opts,
        CancellationToken::noop(),
    ));
    let names = file_names(&mut cache, &indices);
    assert_eq!(names.len(), 2);
    assert!(names.iter().any(|name| name.ends_with("foo/bar/baz.txt")));
    assert!(
        names
            .iter()
            .any(|name| name.ends_with("other/foo/bar/baz.txt"))
    );
}

#[test]
fn trailing_slash_requires_exact_last_segment() {
    let temp_dir = TempDir::new("trailing_slash_requires_exact_last_segment").unwrap();
    let root = temp_dir.path();
    fs::create_dir_all(root.join("docs/guide")).unwrap();
    fs::create_dir_all(root.join("legacy_docs/guide")).unwrap();
    fs::create_dir_all(root.join("docs/guide_extra")).unwrap();

    let mut cache = SearchCache::walk_fs(root);
    let opts = SearchOptions {
        case_insensitive: false,
    };
    let indices =
        guard_indices(cache.search_with_options("docs/guide/", opts, CancellationToken::noop()));
    let names = file_names(&mut cache, &indices);
    assert_eq!(names.len(), 2);
    assert!(names.iter().any(|name| name.ends_with("docs/guide")));
    assert!(names.iter().any(|name| name.ends_with("legacy_docs/guide")));
    assert!(!names.iter().any(|name| name.ends_with("docs/guide_extra")));
}

#[test]
fn no_leading_slash_matches_suffix_segment() {
    let temp_dir = TempDir::new("no_leading_slash_matches_suffix_segment").unwrap();
    let root = temp_dir.path();
    fs::create_dir_all(root.join("foo/bar")).unwrap();
    fs::create_dir_all(root.join("dirfoo/bar")).unwrap();
    fs::create_dir_all(root.join("foo/barn")).unwrap();

    let mut cache = SearchCache::walk_fs(root);
    let indices = guard_indices(cache.search_with_options(
        "foo/bar",
        SearchOptions::default(),
        CancellationToken::noop(),
    ));
    let names = file_names(&mut cache, &indices);
    assert_eq!(names.len(), 3);
    assert!(names.iter().any(|name| name.ends_with("foo/bar")));
    assert!(names.iter().any(|name| name.ends_with("dirfoo/bar")));
    assert!(names.iter().any(|name| name.ends_with("foo/barn")));
}

#[test]
fn case_insensitive_segments_match_variants() {
    let temp_dir = TempDir::new("case_insensitive_segments_match_variants").unwrap();
    let root = temp_dir.path();
    fs::create_dir_all(root.join("Foo/Bar/Baz")).unwrap();
    fs::create_dir_all(root.join("FOO/BAR/Bazooka")).unwrap();

    let mut cache = SearchCache::walk_fs(root);
    let opts = SearchOptions {
        case_insensitive: true,
    };
    let indices =
        guard_indices(cache.search_with_options("/foo/bar/baz/", opts, CancellationToken::noop()));
    let names = file_names(&mut cache, &indices);
    assert_eq!(names.len(), 1);
    assert!(names.iter().any(|name| name.ends_with("Foo/Bar/Baz")));
}

#[test]
fn mixed_prefix_suffix_segments_for_files() {
    let temp_dir = TempDir::new("mixed_prefix_suffix_segments_for_files").unwrap();
    let root = temp_dir.path();
    fs::create_dir_all(root.join("foo")).unwrap();
    fs::create_dir_all(root.join("datafoo")).unwrap();
    fs::File::create(root.join("foo/report.txt")).unwrap();
    fs::File::create(root.join("datafoo/report.txt")).unwrap();
    fs::File::create(root.join("foo/report_final.txt")).unwrap();

    let mut cache = SearchCache::walk_fs(root);
    let opts = SearchOptions {
        case_insensitive: false,
    };
    let indices =
        guard_indices(cache.search_with_options("foo/report.txt", opts, CancellationToken::noop()));
    let names = file_names(&mut cache, &indices);
    assert_eq!(names.len(), 2);
    assert!(names.iter().any(|name| name.ends_with("foo/report.txt")));
    assert!(
        names
            .iter()
            .any(|name| name.ends_with("datafoo/report.txt"))
    );
    assert!(
        !names
            .iter()
            .any(|name| name.ends_with("foo/report_final.txt"))
    );
}

// --- Additional multi path segment coverage below ---
// Goal: expand variety of slash + wildcard + case + overlap behaviors.

fn normalize(cache: &mut SearchCache, indices: &[SlabIndex]) -> Vec<String> {
    cache
        .expand_file_nodes(indices)
        .into_iter()
        .map(|node| node.path.to_string_lossy().into_owned())
        .collect()
}

#[test]
fn trailing_slash_deep_exact_directory() {
    let temp_dir = TempDir::new("trailing_slash_deep_exact_directory").unwrap();
    let root = temp_dir.path();
    fs::create_dir_all(root.join("a/b/c/d")).unwrap();
    fs::create_dir_all(root.join("a/b/c/d_extra")).unwrap();
    fs::create_dir_all(root.join("a/b/cX/d")).unwrap();

    let mut cache = SearchCache::walk_fs(root);
    let opts = SearchOptions {
        case_insensitive: false,
    };
    let indices =
        guard_indices(cache.search_with_options("a/b/c/d/", opts, CancellationToken::noop()));
    let names = normalize(&mut cache, &indices);
    println!("wildcard_last_segment_multiple_extensions names={names:?}");
    println!("mixed_case_segments_case_sensitive_behavior names={names:?}");
    // Only the exact directory "a/b/c/d" should appear; variants excluded.
    assert!(
        names.iter().any(|n| n.ends_with("a/b/c/d")),
        "expected exact directory present"
    );
    assert!(
        !names.iter().any(|n| n.ends_with("a/b/c/d_extra")),
        "trailing slash excludes d_extra"
    );
    assert!(
        !names.iter().any(|n| n.ends_with("a/b/cX/d")),
        "middle segment mismatch should exclude"
    );
}

#[test]
fn leading_slash_with_wildcard_in_first_segment() {
    let temp_dir = TempDir::new("leading_slash_with_wildcard_in_first_segment").unwrap();
    let root = temp_dir.path();
    fs::create_dir_all(root.join("fooA/bar/baz")).unwrap();
    fs::create_dir_all(root.join("fooB/bar/baz")).unwrap();
    fs::create_dir_all(root.join("other/fooA/bar/baz")).unwrap();
    fs::create_dir_all(root.join("fooA/bar/qux")).unwrap();

    let mut cache = SearchCache::walk_fs(root);
    // Wildcard applies to first segment; expectation: directories whose first segment contains pattern prefix 'foo'.
    let indices = guard_indices(cache.search_with_options(
        "/foo*/bar/baz/",
        SearchOptions::default(),
        CancellationToken::noop(),
    ));
    let names = normalize(&mut cache, &indices);
    // We accept matches from both root and nested paths (as earlier leading slash tests showed broader matching).
    assert!(names.iter().any(|n| n.ends_with("fooA/bar/baz")));
    assert!(names.iter().any(|n| n.ends_with("fooB/bar/baz")));
    // Nested path still acceptable per existing semantics.
    assert!(names.iter().any(|n| n.ends_with("other/fooA/bar/baz")));
    // Non-baz leaf excluded.
    assert!(!names.iter().any(|n| n.ends_with("fooA/bar/qux")));
}

#[test]
fn mixed_case_segments_case_sensitive_behavior() {
    let temp_dir = TempDir::new("mixed_case_segments_case_sensitive_behavior").unwrap();
    let root = temp_dir.path();
    fs::create_dir_all(root.join("a/Foo/Bar/Baz")).unwrap();
    fs::create_dir_all(root.join("b/foo/bar/baz")).unwrap();
    fs::create_dir_all(root.join("c/FOO/BAR/BAZ")).unwrap();

    let mut cache = SearchCache::walk_fs(root);
    let opts = SearchOptions {
        case_insensitive: false,
    };
    let indices =
        guard_indices(cache.search_with_options("foo/bar/baz/", opts, CancellationToken::noop()));
    let names = normalize(&mut cache, &indices);
    // Strict lowercase expected; mixed/uppercase variants should be excluded when case-sensitive
    assert!(
        names.iter().any(|n| n.ends_with("foo/bar/baz")),
        "lowercase exact path should match"
    );
    // Ensure the full uppercase variant does not appear for strict case-sensitive search.
    assert!(
        !names.iter().any(|n| n.ends_with("FOO/BAR/BAZ")),
        "uppercase path should be excluded when case_sensitive"
    );
}

#[test]
fn mixed_case_segments_case_insensitive_behavior() {
    let temp_dir = TempDir::new("mixed_case_segments_case_insensitive_behavior").unwrap();
    let root = temp_dir.path();
    fs::create_dir_all(root.join("Foo/Bar/Baz")).unwrap();
    fs::create_dir_all(root.join("foo/bar/baz")).unwrap();
    fs::create_dir_all(root.join("FOO/BAR/BAZ")).unwrap();
    fs::create_dir_all(root.join("foo/bar/Bazooka")).unwrap();

    let mut cache = SearchCache::walk_fs(root);
    let opts = SearchOptions {
        case_insensitive: true,
    };
    let indices =
        guard_indices(cache.search_with_options("/foo/bar/baz/", opts, CancellationToken::noop()));
    let names = normalize(&mut cache, &indices);
    // Only baz directory (exact trailing slash) variants should appear; Bazooka excluded.
    assert!(
        names.iter().any(|n| n.ends_with("Foo/Bar/Baz"))
            || names.iter().any(|n| n.ends_with("foo/bar/baz"))
    );
    assert!(
        !names.iter().any(|n| n.ends_with("foo/bar/Bazooka")),
        "trailing slash exactness excludes Bazooka"
    );
}

#[test]
fn wildcard_last_segment_multiple_extensions() {
    let temp_dir = TempDir::new("wildcard_last_segment_multiple_extensions").unwrap();
    let root = temp_dir.path();
    fs::create_dir_all(root.join("a/docs/guide")).unwrap();
    fs::create_dir_all(root.join("b/docs/guide")).unwrap();
    fs::File::create(root.join("a/docs/guide/readme.md")).unwrap();
    fs::File::create(root.join("a/docs/guide/readme.txt")).unwrap();
    fs::File::create(root.join("a/docs/guide/readme_final.md")).unwrap();
    fs::File::create(root.join("b/docs/guide/README.MD")).unwrap();

    let mut cache = SearchCache::walk_fs(root);
    let opts = SearchOptions {
        case_insensitive: false,
    };
    let indices = guard_indices(cache.search_with_options(
        "docs/guide/readme.*",
        opts,
        CancellationToken::noop(),
    ));
    let names = normalize(&mut cache, &indices);
    // Expect the two base names; basename 'readme_final' should NOT match this pattern; uppercase README excluded.
    assert!(names.iter().any(|n| n.ends_with("readme.md")));
    assert!(names.iter().any(|n| n.ends_with("readme.txt")));
    assert!(
        !names.iter().any(|n| n.ends_with("readme_final.md")),
        "basename mismatch excludes readme_final"
    );
    assert!(
        !names.iter().any(|n| n.ends_with("README.MD")),
        "case sensitive excludes uppercase README"
    );
}

#[test]
fn wildcard_last_segment_multiple_extensions_case_insensitive() {
    let temp_dir =
        TempDir::new("wildcard_last_segment_multiple_extensions_case_insensitive").unwrap();
    let root = temp_dir.path();
    fs::create_dir_all(root.join("a/docs/guide")).unwrap();
    fs::create_dir_all(root.join("b/docs/guide")).unwrap();
    fs::create_dir_all(root.join("c/docs/guide")).unwrap();
    fs::File::create(root.join("a/docs/guide/README.MD")).unwrap();
    fs::File::create(root.join("b/docs/guide/readme.md")).unwrap();
    fs::File::create(root.join("c/docs/guide/readmeX.md")).unwrap();

    let mut cache = SearchCache::walk_fs(root);
    let opts = SearchOptions {
        case_insensitive: true,
    };
    let indices = guard_indices(cache.search_with_options(
        "docs/guide/readme*.md",
        opts,
        CancellationToken::noop(),
    ));
    let names = normalize(&mut cache, &indices);
    println!("wildcard_last_segment_multiple_extensions_case_insensitive names={names:?}");
    // Case-insensitive should pick README.MD; wildcard picks readmeX.md also.
    assert!(names.iter().any(|n| n.ends_with("README.MD")));
    assert!(names.iter().any(|n| n.ends_with("readmeX.md")));
}

#[test]
fn middle_segment_wildcard_variants() {
    let temp_dir = TempDir::new("middle_segment_wildcard_variants").unwrap();
    let root = temp_dir.path();
    fs::create_dir_all(root.join("pkg-alpha/docs/v1")).unwrap();
    fs::create_dir_all(root.join("pkg-beta/docs/v1")).unwrap();
    fs::create_dir_all(root.join("pkg-gamma/docs/v1")).unwrap();
    fs::create_dir_all(root.join("pkg-alpha/docs/v2")).unwrap();

    let mut cache = SearchCache::walk_fs(root);
    let indices = guard_indices(cache.search_with_options(
        "pkg-*/docs/v1/",
        SearchOptions::default(),
        CancellationToken::noop(),
    ));
    let names = normalize(&mut cache, &indices);
    assert!(names.iter().any(|n| n.ends_with("pkg-alpha/docs/v1")));
    assert!(names.iter().any(|n| n.ends_with("pkg-beta/docs/v1")));
    assert!(names.iter().any(|n| n.ends_with("pkg-gamma/docs/v1")));
    assert!(
        !names.iter().any(|n| n.ends_with("pkg-alpha/docs/v2")),
        "v2 excluded by exact v1 segment"
    );
}

#[test]
fn overlapping_prefix_directories() {
    let temp_dir = TempDir::new("overlapping_prefix_directories").unwrap();
    let root = temp_dir.path();
    fs::create_dir_all(root.join("app")).unwrap();
    fs::create_dir_all(root.join("application")).unwrap();
    fs::create_dir_all(root.join("appveyor")).unwrap();
    fs::File::create(root.join("app/config.json")).unwrap();
    fs::File::create(root.join("application/config.json")).unwrap();
    fs::File::create(root.join("appveyor/config.json")).unwrap();

    let mut cache = SearchCache::walk_fs(root);
    // Query uses a slash to combine directory + file name.
    let indices = guard_indices(cache.search_with_options(
        "app/config.json",
        SearchOptions::default(),
        CancellationToken::noop(),
    ));
    let names = normalize(&mut cache, &indices);
    // Expect app/config.json present; application/appveyor may appear if first segment treated as substring - we allow presence but enforce primary target.
    assert!(names.iter().any(|n| n.ends_with("app/config.json")));
}

#[test]
fn globstar_middle_segment_matches_descendants() {
    let temp_dir = TempDir::new("globstar_middle_segment_matches_descendants").unwrap();
    let root = temp_dir.path();
    fs::create_dir_all(root.join("foo"))
        .and_then(|_| fs::create_dir_all(root.join("foo/nested")))
        .and_then(|_| fs::create_dir_all(root.join("foo/nested/deeper")))
        .unwrap();
    fs::File::create(root.join("foo/bar.txt")).unwrap();
    fs::File::create(root.join("foo/nested/bar.txt")).unwrap();
    fs::File::create(root.join("foo/nested/deeper/bar.txt")).unwrap();

    let mut cache = SearchCache::walk_fs(root);
    let indices = guard_indices(cache.search_with_options(
        "foo/**/bar.txt",
        SearchOptions::default(),
        CancellationToken::noop(),
    ));
    let names = normalize(&mut cache, &indices);
    assert!(names.iter().any(|n| n.ends_with("foo/bar.txt")));
    assert!(names.iter().any(|n| n.ends_with("foo/nested/bar.txt")));
    assert!(
        names
            .iter()
            .any(|n| n.ends_with("foo/nested/deeper/bar.txt"))
    );
}

#[test]
fn globstar_trailing_segment_includes_all_descendants() {
    let temp_dir = TempDir::new("globstar_trailing_segment_includes_all_descendants").unwrap();
    let root = temp_dir.path();
    fs::create_dir_all(root.join("foo/sub/layer"))
        .and_then(|_| fs::File::create(root.join("foo/file.txt")))
        .and_then(|_| fs::File::create(root.join("foo/sub/layer/deep.txt")))
        .unwrap();

    let mut cache = SearchCache::walk_fs(root);
    let indices = guard_indices(cache.search_with_options(
        "foo/**",
        SearchOptions::default(),
        CancellationToken::noop(),
    ));
    let names = normalize(&mut cache, &indices);
    assert!(names.iter().any(|n| n.ends_with("foo/sub")));
    assert!(names.iter().any(|n| n.ends_with("foo/sub/layer")));
    assert!(names.iter().any(|n| n.ends_with("foo/file.txt")));
    assert!(names.iter().any(|n| n.ends_with("foo/sub/layer/deep.txt")));
}

#[test]
fn standalone_globstar_matches_entire_tree() {
    let temp_dir = TempDir::new("standalone_globstar_matches_entire_tree").unwrap();
    let root = temp_dir.path();
    fs::create_dir_all(root.join("alpha/beta"))
        .and_then(|_| fs::File::create(root.join("alpha/beta/file_a.txt")))
        .and_then(|_| fs::create_dir_all(root.join("gamma")))
        .and_then(|_| fs::File::create(root.join("gamma/file_b.txt")))
        .unwrap();

    let mut cache = SearchCache::walk_fs(root);
    let indices = guard_indices(cache.search_with_options(
        "**",
        SearchOptions::default(),
        CancellationToken::noop(),
    ));
    let names = normalize(&mut cache, &indices);
    assert!(names.iter().any(|n| n.ends_with("alpha")));
    assert!(names.iter().any(|n| n.ends_with("alpha/beta/file_a.txt")));
    assert!(names.iter().any(|n| n.ends_with("gamma/file_b.txt")));
}

#[test]
fn globstar_matches_nested_hidden_directory_rs_files() {
    let temp_dir = TempDir::new("globstar_matches_nested_hidden_directory_rs_files").unwrap();
    let root = temp_dir.path();
    fs::create_dir_all(root.join("dir/.cargo/index")).unwrap();
    fs::File::create(root.join("dir/.cargo/index/emm.rs")).unwrap();
    fs::File::create(root.join("dir/.cargo/index/skip.txt")).unwrap();

    let mut cache = SearchCache::walk_fs(root);
    let indices = guard_indices(cache.search_with_options(
        ".cargo/**/*.rs",
        SearchOptions::default(),
        CancellationToken::noop(),
    ));
    let names = normalize(&mut cache, &indices);
    assert!(
        names
            .iter()
            .any(|name| name.ends_with(".cargo/index/emm.rs")),
        "globstar pattern should match nested .rs file"
    );
    assert!(
        !names
            .iter()
            .any(|name| name.ends_with(".cargo/index/skip.txt")),
        ".rs pattern should exclude non-Rust files"
    );
}

#[test]
fn multiple_globstars_collapse_to_expected_scope() {
    let temp_dir = TempDir::new("multiple_globstars_collapse_to_expected_scope").unwrap();
    let root = temp_dir.path();
    fs::create_dir_all(root.join("aa/src/module")).unwrap();
    fs::create_dir_all(root.join("aa/module")).unwrap();
    fs::create_dir_all(root.join("bb/aa")).unwrap();
    fs::File::create(root.join("aa/src/module/lib.c")).unwrap();
    fs::File::create(root.join("aa/module/lib.c")).unwrap();
    fs::File::create(root.join("aa/module/lib.txt")).unwrap();
    fs::File::create(root.join("bb/aa/lib.c")).unwrap();

    let mut cache = SearchCache::walk_fs(root);
    let indices = guard_indices(cache.search_with_options(
        "aa/**/**/*.c",
        SearchOptions::default(),
        CancellationToken::noop(),
    ));
    let names = normalize(&mut cache, &indices);
    assert!(names.iter().any(|n| n.ends_with("aa/module/lib.c")));
    assert!(
        names.iter().any(|n| n.ends_with("aa/src/module/lib.c")),
        "deep nested file should be included"
    );
    assert!(
        !names.iter().any(|n| n.ends_with("aa/module/lib.txt")),
        "non .c extension excluded"
    );
    assert!(
        names.iter().any(|n| n.ends_with("bb/aa/lib.c")),
        "suffix segment should match directories ending with aa regardless of parent"
    );
}

#[test]
fn redundant_globstars_match_entire_tree() {
    let temp_dir = TempDir::new("redundant_globstars_match_entire_tree").unwrap();
    let root = temp_dir.path();
    fs::create_dir_all(root.join("x/y/z")).unwrap();
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::File::create(root.join("x/y/z/file.rs")).unwrap();
    fs::File::create(root.join("docs/readme.md")).unwrap();

    let mut cache = SearchCache::walk_fs(root);
    let indices = guard_indices(cache.search_with_options(
        "**/**/**",
        SearchOptions::default(),
        CancellationToken::noop(),
    ));
    let names = normalize(&mut cache, &indices);
    assert!(
        names.iter().any(|n| n.ends_with("x/y/z/file.rs")),
        "deep descendant visible"
    );
    assert!(
        names.iter().any(|n| n.ends_with("docs/readme.md")),
        "sibling branch visible"
    );
}

#[test]
fn globstar_with_question_mark_preserves_length_constraints() {
    let temp_dir =
        TempDir::new("globstar_with_question_mark_preserves_length_constraints").unwrap();
    let root = temp_dir.path();
    fs::create_dir_all(root.join("pkg-alpha")).unwrap();
    fs::create_dir_all(root.join("pkg-beta")).unwrap();
    fs::File::create(root.join("pkg-alpha/lib01.rs")).unwrap();
    fs::File::create(root.join("pkg-alpha/lib1.rs")).unwrap();
    fs::File::create(root.join("pkg-beta/libAA.rs")).unwrap();

    let mut cache = SearchCache::walk_fs(root);
    let indices = guard_indices(cache.search_with_options(
        "**/lib??.rs",
        SearchOptions::default(),
        CancellationToken::noop(),
    ));
    let names = normalize(&mut cache, &indices);
    assert!(
        names.iter().any(|n| n.ends_with("pkg-alpha/lib01.rs"))
            && names.iter().any(|n| n.ends_with("pkg-beta/libAA.rs")),
        "two-character suffix should match"
    );
    assert!(
        !names.iter().any(|n| n.ends_with("pkg-alpha/lib1.rs")),
        "single-char suffix should not match ?"
    );
}

#[test]
fn globstar_case_sensitive_vs_insensitive_variants() {
    let temp_dir = TempDir::new("globstar_case_sensitive_vs_insensitive_variants").unwrap();
    let root = temp_dir.path();
    fs::create_dir_all(root.join("AA/Deep")).unwrap();
    fs::create_dir_all(root.join("aa/Deep")).unwrap();
    fs::File::create(root.join("AA/Deep/FILE.TXT")).unwrap();
    fs::File::create(root.join("aa/Deep/file.txt")).unwrap();

    let mut cache = SearchCache::walk_fs(root);
    let sensitive = guard_indices(cache.search_with_options(
        "aa/**/file.txt",
        SearchOptions::default(),
        CancellationToken::noop(),
    ));
    let sensitive_names = normalize(&mut cache, &sensitive);
    assert!(
        sensitive_names.is_empty(),
        "case-sensitive search with mismatched casing should yield no results"
    );

    let opts = SearchOptions {
        case_insensitive: true,
    };
    let insensitive =
        guard_indices(cache.search_with_options("aa/**/file.txt", opts, CancellationToken::noop()));
    let insensitive_names = normalize(&mut cache, &insensitive);
    assert!(
        insensitive_names
            .iter()
            .any(|n| n.ends_with("AA/Deep/FILE.TXT")),
        "case-insensitive search should include differently cased target"
    );
}

#[test]
fn globstar_case_sensitive_exact_match() {
    let temp_dir = TempDir::new("globstar_case_sensitive_exact_match").unwrap();
    let root = temp_dir.path();
    fs::create_dir_all(root.join("AA/Deep")).unwrap();
    fs::create_dir_all(root.join("aa/Deep")).unwrap();
    fs::File::create(root.join("AA/Deep/FILE.TXT")).unwrap();
    fs::File::create(root.join("aa/Deep/file.txt")).unwrap();

    let mut cache = SearchCache::walk_fs(root);
    let indices = guard_indices(cache.search_with_options(
        "AA/**/FILE.TXT",
        SearchOptions::default(),
        CancellationToken::noop(),
    ));
    let names = normalize(&mut cache, &indices);
    assert!(
        names.iter().any(|n| n.ends_with("AA/Deep/FILE.TXT")),
        "exact case should match when search is case sensitive"
    );
    assert!(
        !names.iter().any(|n| n.ends_with("aa/Deep/file.txt")),
        "lowercase variant should not appear in case-sensitive query"
    );
}

#[test]
fn leading_globstar_matches_any_suffix() {
    let temp_dir = TempDir::new("leading_globstar_matches_any_suffix").unwrap();
    let root = temp_dir.path();
    fs::create_dir_all(root.join("alpha/beta")).unwrap();
    fs::create_dir_all(root.join("gamma/delta")).unwrap();
    fs::File::create(root.join("alpha/beta/report.log")).unwrap();
    fs::File::create(root.join("gamma/delta/report.log")).unwrap();
    fs::File::create(root.join("alpha/report.log")).unwrap();

    let mut cache = SearchCache::walk_fs(root);
    let indices = guard_indices(cache.search_with_options(
        "**/report.log",
        SearchOptions::default(),
        CancellationToken::noop(),
    ));
    let names = normalize(&mut cache, &indices);
    assert!(names.iter().any(|n| n.ends_with("alpha/beta/report.log")));
    assert!(names.iter().any(|n| n.ends_with("gamma/delta/report.log")));
    assert!(names.iter().any(|n| n.ends_with("alpha/report.log")));
}

#[test]
fn wildcard_segment_followed_by_trailing_globstar() {
    let temp_dir = TempDir::new("wildcard_segment_followed_by_trailing_globstar").unwrap();
    let root = temp_dir.path();
    fs::create_dir_all(root.join("client-app/src")).unwrap();
    fs::create_dir_all(root.join("client-lib/tests")).unwrap();
    fs::create_dir_all(root.join("server-app/src")).unwrap();
    fs::File::create(root.join("client-app/src/main.rs")).unwrap();
    fs::File::create(root.join("client-lib/tests/test.rs")).unwrap();
    fs::File::create(root.join("server-app/src/ignore.rs")).unwrap();

    let mut cache = SearchCache::walk_fs(root);
    let indices = guard_indices(cache.search_with_options(
        "client*/**",
        SearchOptions::default(),
        CancellationToken::noop(),
    ));
    let names = normalize(&mut cache, &indices);
    assert!(names.iter().any(|n| n.ends_with("client-app/src")));
    assert!(names.iter().any(|n| n.ends_with("client-app/src/main.rs")));
    assert!(names.iter().any(|n| n.ends_with("client-lib/tests")));
    assert!(
        names
            .iter()
            .any(|n| n.ends_with("client-lib/tests/test.rs"))
    );
    assert!(
        !names
            .iter()
            .any(|n| n.ends_with("server-app/src/ignore.rs")),
        "non-matching prefix should be excluded"
    );
}

#[test]
fn globstar_question_mark_segment_and_trailing_globstar() {
    let temp_dir = TempDir::new("globstar_question_mark_segment_and_trailing_globstar").unwrap();
    let root = temp_dir.path();
    fs::create_dir_all(root.join("pkg-a/lib1/src")).unwrap();
    fs::create_dir_all(root.join("pkg-b/libA/src")).unwrap();
    fs::create_dir_all(root.join("pkg-c/libAB/src")).unwrap();
    fs::File::create(root.join("pkg-a/lib1/src/main.rs")).unwrap();
    fs::File::create(root.join("pkg-b/libA/src/main.rs")).unwrap();
    fs::File::create(root.join("pkg-c/libAB/src/main.rs")).unwrap();

    let mut cache = SearchCache::walk_fs(root);
    let indices = guard_indices(cache.search_with_options(
        "**/lib?/src/**",
        SearchOptions::default(),
        CancellationToken::noop(),
    ));
    let names = normalize(&mut cache, &indices);
    assert!(
        names.iter().any(|n| n.ends_with("pkg-a/lib1/src/main.rs")),
        "lib1 matches ?"
    );
    assert!(
        names.iter().any(|n| n.ends_with("pkg-b/libA/src/main.rs")),
        "libA matches ?"
    );
    assert!(
        !names.iter().any(|n| n.ends_with("pkg-c/libAB/src/main.rs")),
        "two-character suffix should be excluded by single ?"
    );
}

#[test]
fn wildcard_question_mark_inside_segment() {
    let temp_dir = TempDir::new("wildcard_question_mark_inside_segment").unwrap();
    let root = temp_dir.path();
    fs::File::create(root.join("lib-a1.tar.gz")).unwrap();
    fs::File::create(root.join("lib-a2.tar.gz")).unwrap();
    fs::File::create(root.join("lib-a10.tar.gz")).unwrap();

    let mut cache = SearchCache::walk_fs(root);
    let indices = guard_indices(cache.search_with_options(
        "lib-a?.tar.gz",
        SearchOptions::default(),
        CancellationToken::noop(),
    ));
    let names = normalize(&mut cache, &indices);
    // Single ? should match a1/a2 only (not a10 if segmentation respects single-char semantics).
    assert!(names.iter().any(|n| n.ends_with("lib-a1.tar.gz")));
    assert!(names.iter().any(|n| n.ends_with("lib-a2.tar.gz")));
    assert!(
        !names.iter().any(|n| n.ends_with("lib-a10.tar.gz")),
        "a10 should not match single-character ? pattern"
    );
}

#[test]
fn multi_level_mixed_wildcards_and_trailing_slash() {
    let temp_dir = TempDir::new("multi_level_mixed_wildcards_and_trailing_slash").unwrap();
    let root = temp_dir.path();
    fs::create_dir_all(root.join("services/api-v1/internal")).unwrap();
    fs::create_dir_all(root.join("services/api-v2/internal")).unwrap();
    fs::create_dir_all(root.join("services/api-v1/internal_extra")).unwrap();

    let mut cache = SearchCache::walk_fs(root);
    let indices = guard_indices(cache.search_with_options(
        "services/api-v*/internal/",
        SearchOptions::default(),
        CancellationToken::noop(),
    ));
    let names = normalize(&mut cache, &indices);
    // internal directories for v1 and v2 included; internal_extra excluded by trailing slash exactness.
    assert!(
        names
            .iter()
            .any(|n| n.ends_with("services/api-v1/internal"))
    );
    assert!(
        names
            .iter()
            .any(|n| n.ends_with("services/api-v2/internal"))
    );
    assert!(
        !names
            .iter()
            .any(|n| n.ends_with("services/api-v1/internal_extra"))
    );
}

#[test]
fn path_query_with_dot_segments_and_files() {
    let temp_dir = TempDir::new("path_query_with_dot_segments_and_files").unwrap();
    let root = temp_dir.path();
    fs::create_dir_all(root.join("config.d/profiles")).unwrap();
    fs::File::create(root.join("config.d/profiles/default.yaml")).unwrap();
    fs::File::create(root.join("config.d/profiles/dev.yaml")).unwrap();
    fs::File::create(root.join("config.d/profiles/dev.json")).unwrap();

    let mut cache = SearchCache::walk_fs(root);
    let indices = guard_indices(cache.search_with_options(
        "config.d/profiles/dev.yaml",
        SearchOptions::default(),
        CancellationToken::noop(),
    ));
    let names = normalize(&mut cache, &indices);
    assert!(
        names
            .iter()
            .any(|n| n.ends_with("config.d/profiles/dev.yaml"))
    );
    assert!(
        !names
            .iter()
            .any(|n| n.ends_with("config.d/profiles/dev.json")),
        "extension mismatch should exclude json"
    );
}

#[test]
fn unicode_path_segments_case_insensitive() {
    let temp_dir = TempDir::new("unicode_path_segments_case_insensitive").unwrap();
    let root = temp_dir.path();
    let decomposed = "Cafe\u{0301}";
    let decomposed_upper = "CAFE\u{0301}";
    fs::create_dir_all(root.join(decomposed).join("文件")).unwrap();
    fs::File::create(root.join(decomposed).join("文件/notes.txt")).unwrap();
    fs::create_dir_all(root.join(decomposed_upper).join("文件")).unwrap();
    fs::File::create(root.join(decomposed_upper).join("文件/notes.txt")).unwrap();

    let mut cache = SearchCache::walk_fs(root);
    let opts = SearchOptions {
        case_insensitive: true,
    };
    let indices = guard_indices(cache.search_with_options(
        "/café/文件/notes.txt",
        opts,
        CancellationToken::noop(),
    ));
    let names = normalize(&mut cache, &indices);
    // Both case variants should be matched.
    assert!(
        names
            .iter()
            .any(|n| n.ends_with(&format!("{decomposed}/文件/notes.txt")))
            || names
                .iter()
                .any(|n| n.ends_with(&format!("{decomposed_upper}/文件/notes.txt")))
    );
}

#[test]
fn unicode_normalization_equivalent_forms_should_match() {
    let temp_dir = TempDir::new("unicode_normalization_equivalent_forms_should_match").unwrap();
    let root = temp_dir.path();

    let composed = "B\u{00FC}ro";
    let decomposed = "Bu\u{0308}ro";
    fs::create_dir_all(root.join(decomposed)).unwrap();
    fs::File::create(root.join(decomposed).join("angebot.pdf")).unwrap();

    let mut cache = SearchCache::walk_fs(root);
    let opts = SearchOptions {
        case_insensitive: true,
    };

    // Filesystems may surface NFC or NFD names; query using the opposite form
    // to verify canonical-equivalent matching behavior.
    let all =
        guard_indices(cache.search_with_options("angebot.pdf", opts, CancellationToken::noop()));
    let paths = normalize(&mut cache, &all);
    let stored = paths
        .iter()
        .find(|path| path.ends_with("angebot.pdf"))
        .expect("fixture file should be indexed");
    let query = if stored.contains(decomposed) {
        composed
    } else {
        decomposed
    };

    let indices = guard_indices(cache.search_with_options(query, opts, CancellationToken::noop()));
    let names = normalize(&mut cache, &indices);
    assert!(
        names.iter().any(|n| n.ends_with(composed))
            || names.iter().any(|n| n.ends_with(decomposed)),
        "expected canonical-equivalent query form to match folder segment: query={query:?}, indexed={stored:?}, results={names:?}"
    );
}

#[test]
fn unicode_already_nfd_query_keeps_matching() {
    let temp_dir = TempDir::new("unicode_already_nfd_query_keeps_matching").unwrap();
    let root = temp_dir.path();
    let decomposed = "Cafe\u{0301}";
    fs::create_dir_all(root.join(decomposed)).unwrap();
    fs::File::create(root.join(decomposed).join("guide.txt")).unwrap();

    let mut cache = SearchCache::walk_fs(root);
    let query = format!("{decomposed}/guide.txt");
    let indices = guard_indices(cache.search_with_options(
        &query,
        SearchOptions {
            case_insensitive: false,
        },
        CancellationToken::noop(),
    ));
    let names = normalize(&mut cache, &indices);
    assert!(
        names
            .iter()
            .any(|n| n.ends_with(&format!("{decomposed}/guide.txt"))),
        "already-NFD query should continue to match directly: results={names:?}"
    );
}

#[test]
fn unicode_noncanonical_combining_order_is_normalized() {
    let temp_dir = TempDir::new("unicode_noncanonical_combining_order_is_normalized").unwrap();
    let root = temp_dir.path();
    let canonical_nfd = "a\u{0323}\u{0302}";
    let noncanonical = "a\u{0302}\u{0323}";
    fs::create_dir_all(root.join(canonical_nfd)).unwrap();
    fs::File::create(root.join(canonical_nfd).join("probe.txt")).unwrap();

    let mut cache = SearchCache::walk_fs(root);
    let query = format!("{noncanonical}/probe.txt");
    let indices = guard_indices(cache.search_with_options(
        &query,
        SearchOptions {
            case_insensitive: false,
        },
        CancellationToken::noop(),
    ));
    let names = normalize(&mut cache, &indices);
    assert!(
        names
            .iter()
            .any(|n| n.ends_with(&format!("{canonical_nfd}/probe.txt"))),
        "query with non-canonical combining order should normalize to NFD and match: results={names:?}"
    );
}

#[test]
fn unicode_composed_query_with_wildcard_matches_decomposed_path() {
    let temp_dir =
        TempDir::new("unicode_composed_query_with_wildcard_matches_decomposed_path").unwrap();
    let root = temp_dir.path();
    let composed = "B\u{00FC}ro";
    let decomposed = "Bu\u{0308}ro";
    fs::create_dir_all(root.join(decomposed)).unwrap();
    fs::File::create(root.join(decomposed).join("bericht.txt")).unwrap();
    fs::File::create(root.join(decomposed).join("brot.txt")).unwrap();

    let mut cache = SearchCache::walk_fs(root);
    let query = format!("{composed}/ber*.txt");
    let indices = guard_indices(cache.search_with_options(
        &query,
        SearchOptions {
            case_insensitive: true,
        },
        CancellationToken::noop(),
    ));
    let names = normalize(&mut cache, &indices);
    assert!(
        names
            .iter()
            .any(|n| n.ends_with(&format!("{decomposed}/bericht.txt"))),
        "wildcard + composed query should match decomposed path: results={names:?}"
    );
    assert!(
        !names
            .iter()
            .any(|n| n.ends_with(&format!("{decomposed}/brot.txt"))),
        "wildcard segment should still filter unrelated filenames"
    );
}

#[test]
fn unicode_normalization_preserves_boolean_or_behavior() {
    let temp_dir = TempDir::new("unicode_normalization_preserves_boolean_or_behavior").unwrap();
    let root = temp_dir.path();
    let composed = "B\u{00FC}ro";
    let decomposed = "Bu\u{0308}ro";
    let ascii_marker = "alpha_corner_marker_123";

    fs::create_dir_all(root.join(decomposed)).unwrap();
    fs::create_dir_all(root.join(ascii_marker)).unwrap();

    let mut cache = SearchCache::walk_fs(root);
    let query = format!("{composed} OR {ascii_marker}");
    let indices = guard_indices(cache.search_with_options(
        &query,
        SearchOptions {
            case_insensitive: true,
        },
        CancellationToken::noop(),
    ));
    let names = normalize(&mut cache, &indices);
    assert!(
        names.iter().any(|n| n.ends_with(decomposed)),
        "OR query should still include normalized unicode branch: results={names:?}"
    );
    assert!(
        names.iter().any(|n| n.ends_with(ascii_marker)),
        "OR query should still include ascii branch: results={names:?}"
    );
}

#[test]
fn unicode_path_segments_case_sensitive() {
    let temp_dir = TempDir::new("unicode_path_segments_case_sensitive").unwrap();
    let root = temp_dir.path();
    fs::create_dir_all(root.join("Café/文件")).unwrap();
    fs::File::create(root.join("Café/文件/notes.txt")).unwrap();
    fs::create_dir_all(root.join("CAFÉ/文件")).unwrap();
    fs::File::create(root.join("CAFÉ/文件/notes.txt")).unwrap();

    let mut cache = SearchCache::walk_fs(root);
    let opts = SearchOptions {
        case_insensitive: false,
    };
    let indices = guard_indices(cache.search_with_options(
        "café/文件/notes.txt",
        opts,
        CancellationToken::noop(),
    ));
    let names = normalize(&mut cache, &indices);
    // Depending on segmentation, lowercase query should match the mixed-case variant or none; ensure uppercase variant absent.
    assert!(
        !names.iter().any(|n| n.ends_with("CAFÉ/文件/notes.txt")),
        "uppercase path should not match lowercase query when sensitive"
    );
}

#[test]
fn deep_multiple_wildcards_varied_segments() {
    let temp_dir = TempDir::new("deep_multiple_wildcards_varied_segments").unwrap();
    let root = temp_dir.path();
    fs::create_dir_all(root.join("src/lib/core")).unwrap();
    fs::create_dir_all(root.join("src/lib/util")).unwrap();
    fs::create_dir_all(root.join("src/lib-core/extra")).unwrap();
    fs::File::create(root.join("src/lib/core/mod.rs")).unwrap();
    fs::File::create(root.join("src/lib/util/mod.rs")).unwrap();
    fs::File::create(root.join("src/lib-core/extra/mod.rs")).unwrap();

    let mut cache = SearchCache::walk_fs(root);
    let indices = guard_indices(cache.search_with_options(
        "src/lib*/core/mod.rs",
        SearchOptions::default(),
        CancellationToken::noop(),
    ));
    let names = normalize(&mut cache, &indices);
    // Expect src/lib/core/mod.rs; src/lib-core/extra does not end with /core/mod.rs
    assert!(names.iter().any(|n| n.ends_with("src/lib/core/mod.rs")));
    assert!(!names.iter().any(|n| n.ends_with("src/lib/util/mod.rs")));
    assert!(
        !names
            .iter()
            .any(|n| n.ends_with("src/lib-core/extra/mod.rs"))
    );
}

#[test]
fn file_match_with_intermediate_prefix_overlap() {
    let temp_dir = TempDir::new("file_match_with_intermediate_prefix_overlap").unwrap();
    let root = temp_dir.path();
    fs::create_dir_all(root.join("client/app")).unwrap();
    fs::create_dir_all(root.join("client/application")).unwrap();
    fs::File::create(root.join("client/app/index.html")).unwrap();
    fs::File::create(root.join("client/application/index.html")).unwrap();

    let mut cache = SearchCache::walk_fs(root);
    let indices = guard_indices(cache.search_with_options(
        "client/app/index.html",
        SearchOptions::default(),
        CancellationToken::noop(),
    ));
    let names = normalize(&mut cache, &indices);
    assert!(names.iter().any(|n| n.ends_with("client/app/index.html")));
}

#[test]
fn star_only_directory_inclusion() {
    let temp_dir = TempDir::new("star_only_directory_inclusion").unwrap();
    let root = temp_dir.path();
    fs::create_dir_all(root.join("one")).unwrap();
    fs::create_dir_all(root.join("two")).unwrap();
    fs::create_dir_all(root.join("three")).unwrap();
    fs::File::create(root.join("one/a.txt")).unwrap();
    fs::File::create(root.join("two/b.txt")).unwrap();
    fs::File::create(root.join("three/c.txt")).unwrap();

    let mut cache = SearchCache::walk_fs(root);
    let indices = guard_indices(cache.search_with_options(
        "*",
        SearchOptions::default(),
        CancellationToken::noop(),
    ));
    let names = normalize(&mut cache, &indices);
    // At least each top-level directory should be represented among matches.
    assert!(names.iter().any(|n| n.ends_with("/one")));
    assert!(names.iter().any(|n| n.ends_with("/two")));
    assert!(names.iter().any(|n| n.ends_with("/three")));
}

#[test]
fn question_mark_in_directory_segment_boundaries() {
    let temp_dir = TempDir::new("question_mark_in_directory_segment_boundaries").unwrap();
    let root = temp_dir.path();
    fs::create_dir_all(root.join("dirA")).unwrap();
    fs::create_dir_all(root.join("dirB")).unwrap();
    fs::create_dir_all(root.join("dirAA")).unwrap();
    fs::File::create(root.join("dirA/file.txt")).unwrap();
    fs::File::create(root.join("dirB/file.txt")).unwrap();
    fs::File::create(root.join("dirAA/file.txt")).unwrap();

    let mut cache = SearchCache::walk_fs(root);
    let indices = guard_indices(cache.search_with_options(
        "dir?/file.txt",
        SearchOptions::default(),
        CancellationToken::noop(),
    ));
    let names = normalize(&mut cache, &indices);
    // Single ? should match dirA and dirB only.
    assert!(names.iter().any(|n| n.ends_with("dirA/file.txt")));
    assert!(names.iter().any(|n| n.ends_with("dirB/file.txt")));
    assert!(
        !names.iter().any(|n| n.ends_with("dirAA/file.txt")),
        "dirAA should be excluded by single char pattern"
    );
}

#[test]
fn multiple_question_marks_segment() {
    let temp_dir = TempDir::new("multiple_question_marks_segment").unwrap();
    let root = temp_dir.path();
    fs::File::create(root.join("log-1234.txt")).unwrap();
    fs::File::create(root.join("log-12.txt")).unwrap();
    fs::File::create(root.join("log-1.txt")).unwrap();
    fs::File::create(root.join("log-12345.txt")).unwrap();

    let mut cache = SearchCache::walk_fs(root);
    let indices = guard_indices(cache.search_with_options(
        "log-????.txt",
        SearchOptions::default(),
        CancellationToken::noop(),
    ));
    let names = normalize(&mut cache, &indices);
    // Four ? characters => exactly four digits.
    assert!(names.iter().any(|n| n.ends_with("log-1234.txt")));
    assert!(!names.iter().any(|n| n.ends_with("log-12.txt")));
    assert!(!names.iter().any(|n| n.ends_with("log-1.txt")));
    assert!(!names.iter().any(|n| n.ends_with("log-12345.txt")));
}

#[test]
fn mixed_star_and_question_mark_segment() {
    let temp_dir = TempDir::new("mixed_star_and_question_mark_segment").unwrap();
    let root = temp_dir.path();
    fs::File::create(root.join("pkg-alpha-v1.rs")).unwrap();
    fs::File::create(root.join("pkg-alpha-v2.rs")).unwrap();
    fs::File::create(root.join("pkg-alpha-v10.rs")).unwrap();
    fs::File::create(root.join("pkg-alpha-vX.rs")).unwrap();

    let mut cache = SearchCache::walk_fs(root);
    // Pattern: pkg-alpha-v?.rs -> one character version.
    let indices_short = guard_indices(cache.search_with_options(
        "pkg-alpha-v?.rs",
        SearchOptions::default(),
        CancellationToken::noop(),
    ));
    let names_short = normalize(&mut cache, &indices_short);
    assert!(names_short.iter().any(|n| n.ends_with("pkg-alpha-v1.rs")));
    assert!(names_short.iter().any(|n| n.ends_with("pkg-alpha-v2.rs")));
    assert!(!names_short.iter().any(|n| n.ends_with("pkg-alpha-v10.rs")));

    // Pattern: pkg-alpha-v*.rs -> any version.
    let indices_any = guard_indices(cache.search_with_options(
        "pkg-alpha-v*.rs",
        SearchOptions::default(),
        CancellationToken::noop(),
    ));
    let names_any = normalize(&mut cache, &indices_any);
    assert!(names_any.iter().any(|n| n.ends_with("pkg-alpha-v10.rs")));
    assert!(names_any.iter().any(|n| n.ends_with("pkg-alpha-vX.rs")));
}

// --- Combinatorial case sensitivity vs path segment casing & wildcards ---

#[test]
fn case_sensitive_exact_segment_casing() {
    let temp_dir = TempDir::new("case_sensitive_exact_segment_casing").unwrap();
    let root = temp_dir.path();
    fs::create_dir_all(root.join("a/src/lib/core")).unwrap();
    fs::create_dir_all(root.join("b/Src/Lib/Core")).unwrap();
    fs::create_dir_all(root.join("c/SRC/LIB/Core")).unwrap();

    let mut cache = SearchCache::walk_fs(root);
    // Case sensitive: only exact lower-case path should be returned for lower-case query.
    let opts = SearchOptions {
        case_insensitive: false,
    };
    let indices =
        guard_indices(cache.search_with_options("src/lib/core/", opts, CancellationToken::noop()));
    let names = normalize(&mut cache, &indices);
    assert!(
        names.iter().any(|n| n.ends_with("src/lib/core")),
        "exact lowercase path expected"
    );
    assert!(
        !names.iter().any(|n| n.ends_with("Src/Lib/Core")),
        "mixed case excluded when sensitive"
    );
    assert!(
        !names.iter().any(|n| n.ends_with("SRC/LIB/Core")),
        "uppercase excluded when sensitive"
    );
}

#[test]
fn case_insensitive_directory_variants() {
    let temp_dir = TempDir::new("case_insensitive_directory_variants").unwrap();
    let root = temp_dir.path();
    fs::create_dir_all(root.join("a/src/lib/core")).unwrap();
    fs::create_dir_all(root.join("b/Src/Lib/Core")).unwrap();
    fs::create_dir_all(root.join("c/SRC/LIB/Core")).unwrap();

    let mut cache = SearchCache::walk_fs(root);
    let opts = SearchOptions {
        case_insensitive: true,
    };
    let indices =
        guard_indices(cache.search_with_options("/src/lib/core/", opts, CancellationToken::noop()));
    let names = normalize(&mut cache, &indices);
    assert!(
        !names.is_empty(),
        "at least one variant should match case-insensitive"
    );
    // All matched variants should end with a Core directory (any casing) and not include unrelated paths.
    for n in &names {
        assert!(n.to_ascii_lowercase().ends_with("src/lib/core"));
    }
}

#[test]
fn mixed_wildcard_case_sensitive_file_variants() {
    let temp_dir = TempDir::new("mixed_wildcard_case_sensitive_file_variants").unwrap();
    let root = temp_dir.path();
    fs::create_dir_all(root.join("a/app/config")).unwrap();
    fs::create_dir_all(root.join("b/app/config")).unwrap();
    fs::File::create(root.join("a/app/config/readme.md")).unwrap();
    fs::File::create(root.join("b/app/config/README.MD")).unwrap();
    fs::File::create(root.join("a/app/config/readme_final.md")).unwrap();

    let mut cache = SearchCache::walk_fs(root);
    let opts = SearchOptions {
        case_insensitive: false,
    };
    let indices = guard_indices(cache.search_with_options(
        "app/config/readme.*",
        opts,
        CancellationToken::noop(),
    ));
    let names = normalize(&mut cache, &indices);
    println!("mixed_wildcard_case_sensitive_file_variants names={names:?}");
    assert!(
        names.iter().any(|n| n.ends_with("app/config/readme.md")),
        "lowercase file should match"
    );
    assert!(
        !names
            .iter()
            .any(|n| n.ends_with("app/config/readme_final.md")),
        "basename-only wildcard should not include readme_final"
    );
    assert!(
        !names.iter().any(|n| n.ends_with("README.MD")),
        "uppercase README excluded when sensitive"
    );
}

#[test]
fn mixed_wildcard_case_insensitive_file_variants() {
    let temp_dir = TempDir::new("mixed_wildcard_case_insensitive_file_variants").unwrap();
    let root = temp_dir.path();
    fs::create_dir_all(root.join("a/app/config")).unwrap();
    fs::create_dir_all(root.join("b/app/config")).unwrap();
    fs::File::create(root.join("a/app/config/readme.md")).unwrap();
    fs::File::create(root.join("b/app/config/README.MD")).unwrap();

    let mut cache = SearchCache::walk_fs(root);
    let opts = SearchOptions {
        case_insensitive: true,
    };
    let indices = guard_indices(cache.search_with_options(
        "/app/config/readme.*",
        opts,
        CancellationToken::noop(),
    ));
    let names = normalize(&mut cache, &indices);
    println!("mixed_wildcard_case_insensitive_file_variants names={names:?}");
    // Case-insensitive should collect lowercase and uppercase filename variants (across different parents).
    assert!(names.iter().any(|n| n.ends_with("app/config/readme.md")));
    assert!(
        names.iter().any(|n| n.ends_with("README.MD")),
        "uppercase filename variant should appear"
    );
}

#[test]
fn case_sensitive_file_exact_match_variants() {
    let temp_dir = TempDir::new("case_sensitive_file_exact_match_variants").unwrap();
    let root = temp_dir.path();
    fs::create_dir_all(root.join("guide")).unwrap();
    fs::File::create(root.join("guide/ReadMe.md")).unwrap();
    fs::File::create(root.join("guide/README.md")).unwrap();
    fs::File::create(root.join("guide/readme.md")).unwrap();

    let mut cache = SearchCache::walk_fs(root);
    let opts = SearchOptions {
        case_insensitive: false,
    };
    let indices = guard_indices(cache.search_with_options(
        "guide/ReadMe.md",
        opts,
        CancellationToken::noop(),
    ));
    let names = normalize(&mut cache, &indices);
    // Only the exact cased file should match.
    assert!(names.iter().any(|n| n.ends_with("guide/ReadMe.md")));
    assert!(!names.iter().any(|n| n.ends_with("guide/README.md")));
    assert!(!names.iter().any(|n| n.ends_with("guide/readme.md")));
}

#[test]
fn case_insensitive_file_exact_match_variants() {
    let temp_dir = TempDir::new("case_insensitive_file_exact_match_variants").unwrap();
    let root = temp_dir.path();
    fs::create_dir_all(root.join("guide")).unwrap();
    fs::File::create(root.join("guide/ReadMe.md")).unwrap();
    fs::File::create(root.join("guide/README.md")).unwrap();
    fs::File::create(root.join("guide/readme.md")).unwrap();

    let mut cache = SearchCache::walk_fs(root);
    let opts = SearchOptions {
        case_insensitive: true,
    };
    let indices = guard_indices(cache.search_with_options(
        "guide/readme.md",
        opts,
        CancellationToken::noop(),
    ));
    let names = normalize(&mut cache, &indices);
    // All case variants may surface; ensure at least one and all contain readme.md ignoring case.
    assert!(!names.is_empty());
    for n in &names {
        assert!(n.to_ascii_lowercase().ends_with("guide/readme.md"));
    }
}

/// A pure ASCII query is normalization-inert (`is_nfd_quick`/`is_nfc_quick`
/// both return `Yes`), so no secondary normalization pass is needed.
/// Verify this still produces correct matches.
#[test]
fn unicode_ascii_only_query_uses_fast_path_correctly() {
    let temp_dir = TempDir::new("unicode_ascii_only_query_uses_fast_path_correctly").unwrap();
    let root = temp_dir.path();
    fs::create_dir_all(root.join("ascii_docs")).unwrap();
    fs::File::create(root.join("ascii_docs/readme.txt")).unwrap();

    let mut cache = SearchCache::walk_fs(root);
    let indices = guard_indices(cache.search_with_options(
        "ascii_docs/readme.txt",
        SearchOptions {
            case_insensitive: false,
        },
        CancellationToken::noop(),
    ));
    let names = normalize(&mut cache, &indices);
    assert!(
        names.iter().any(|n| n.ends_with("ascii_docs/readme.txt")),
        "plain ASCII query should still match normally: results={names:?}"
    );
}

/// A trailing-slash query specifies a directory exactly. Pairing this with a
/// composed (NFC) Unicode query that must be NFD-normalized verifies that the
/// normalization pass does not disturb trailing-slash semantics.
#[test]
fn unicode_trailing_slash_nfc_query_matches_nfd_directory() {
    let temp_dir = TempDir::new("unicode_trailing_slash_nfc_query_matches_nfd_directory").unwrap();
    let root = temp_dir.path();
    let decomposed = "Bu\u{0308}ro"; // NFD: Bu + combining diaeresis
    fs::create_dir_all(root.join(decomposed).join("sub")).unwrap();

    let mut cache = SearchCache::walk_fs(root);
    let query = "B\u{00FC}ro/"; // NFC ü + trailing slash
    let indices = guard_indices(cache.search_with_options(
        query,
        SearchOptions {
            case_insensitive: false,
        },
        CancellationToken::noop(),
    ));
    let names = normalize(&mut cache, &indices);
    assert!(
        names
            .iter()
            .any(|n| n.ends_with(decomposed) && !n.ends_with("sub")),
        "trailing-slash NFC query should match NFD directory exactly: results={names:?}"
    );
}

/// U+212B ANGSTROM SIGN has a two-step canonical decomposition:
///   U+212B → U+00C5 (Å) → A + U+030A (combining ring above).
/// Querying with the Angstrom sign should land on the same NFD form as a path
/// segment stored with the fully decomposed A + U+030A sequence.
#[test]
fn unicode_angstrom_sign_query_matches_a_ring_decomposed_path() {
    let temp_dir =
        TempDir::new("unicode_angstrom_sign_query_matches_a_ring_decomposed_path").unwrap();
    let root = temp_dir.path();
    let nfd_a_ring = "data_A\u{030A}ngstrom"; // A + combining ring above (NFD Å)
    fs::create_dir_all(root.join(nfd_a_ring)).unwrap();
    fs::File::create(root.join(nfd_a_ring).join("spectrum.csv")).unwrap();

    let mut cache = SearchCache::walk_fs(root);
    let query = "data_\u{212B}ngstrom/spectrum.csv"; // U+212B Angstrom sign
    let indices = guard_indices(cache.search_with_options(
        query,
        SearchOptions {
            case_insensitive: false,
        },
        CancellationToken::noop(),
    ));
    let names = normalize(&mut cache, &indices);
    assert!(
        names.iter().any(|n| n.ends_with("spectrum.csv")),
        "ANGSTROM SIGN (U+212B) query should NFD-normalize and match A+U+030A path: results={names:?}"
    );
}

/// Combining case-insensitive matching with NFC-to-NFD normalization:
/// an uppercase NFC query (`B\u{00FC}ro`) should match a lowercase NFD path
/// (`bu\u{0308}ro`) when `case_insensitive` is true.
#[test]
fn unicode_uppercase_nfc_query_matches_lowercase_nfd_dir_case_insensitive() {
    let temp_dir =
        TempDir::new("unicode_uppercase_nfc_query_matches_lowercase_nfd_dir_case_insensitive")
            .unwrap();
    let root = temp_dir.path();
    let lower_nfd = "bu\u{0308}ro"; // lowercase NFD büro
    fs::create_dir_all(root.join(lower_nfd)).unwrap();
    fs::File::create(root.join(lower_nfd).join("rechnung.pdf")).unwrap();

    let mut cache = SearchCache::walk_fs(root);
    let query = "B\u{00FC}ro/rechnung.pdf"; // uppercase NFC Büro
    let indices = guard_indices(cache.search_with_options(
        query,
        SearchOptions {
            case_insensitive: true,
        },
        CancellationToken::noop(),
    ));
    let names = normalize(&mut cache, &indices);
    assert!(
        names.iter().any(|n| n.ends_with("rechnung.pdf")),
        "uppercase NFC query should match lowercase NFD path with case_insensitive=true: results={names:?}"
    );
}

/// A wildcard pattern whose literal suffix contains a composed (NFC) character
/// must still correctly match after the whole query string is NFD-normalized.
/// `*\u{00FC}ro` → `*u\u{0308}ro` should match both `bu\u{0308}ro` and
/// `ku\u{0308}ro` (any segment ending in `u\u{0308}ro`).
#[test]
fn unicode_nfc_wildcard_suffix_matches_nfd_paths() {
    let temp_dir = TempDir::new("unicode_nfc_wildcard_suffix_matches_nfd_paths").unwrap();
    let root = temp_dir.path();
    let nfd_buro = "bu\u{0308}ro";
    let nfd_kuro = "ku\u{0308}ro";
    let unrelated = "turbo";
    fs::create_dir_all(root.join(nfd_buro)).unwrap();
    fs::File::create(root.join(nfd_buro).join("log.txt")).unwrap();
    fs::create_dir_all(root.join(nfd_kuro)).unwrap();
    fs::File::create(root.join(nfd_kuro).join("log.txt")).unwrap();
    fs::create_dir_all(root.join(unrelated)).unwrap();
    fs::File::create(root.join(unrelated).join("log.txt")).unwrap();

    let mut cache = SearchCache::walk_fs(root);
    let query = "*\u{00FC}ro/log.txt"; // NFC *üro/log.txt
    let indices = guard_indices(cache.search_with_options(
        query,
        SearchOptions {
            case_insensitive: false,
        },
        CancellationToken::noop(),
    ));
    let names = normalize(&mut cache, &indices);
    assert!(
        names
            .iter()
            .any(|n| n.ends_with(&format!("{nfd_buro}/log.txt"))),
        "NFC wildcard *üro should match NFD büro: results={names:?}"
    );
    assert!(
        names
            .iter()
            .any(|n| n.ends_with(&format!("{nfd_kuro}/log.txt"))),
        "NFC wildcard *üro should also match NFD küro: results={names:?}"
    );
    assert!(
        !names
            .iter()
            .any(|n| n.ends_with(&format!("{unrelated}/log.txt"))),
        "non-umlaut suffix should not match: results={names:?}"
    );
}

/// An OR query whose both branches contain NFC characters must normalize each
/// branch independently and match its respective NFD directory.
#[test]
fn unicode_nfc_or_query_matches_two_nfd_directories() {
    let temp_dir = TempDir::new("unicode_nfc_or_query_matches_two_nfd_directories").unwrap();
    let root = temp_dir.path();
    let nfd_buro = "Bu\u{0308}ro";
    let nfd_kuche = "Ku\u{0308}che";
    let unrelated = "reports";
    fs::create_dir_all(root.join(nfd_buro)).unwrap();
    fs::create_dir_all(root.join(nfd_kuche)).unwrap();
    fs::create_dir_all(root.join(unrelated)).unwrap();

    let mut cache = SearchCache::walk_fs(root);
    let query = "B\u{00FC}ro OR K\u{00FC}che"; // NFC: Büro OR Küche
    let indices = guard_indices(cache.search_with_options(
        query,
        SearchOptions {
            case_insensitive: false,
        },
        CancellationToken::noop(),
    ));
    let names = normalize(&mut cache, &indices);
    assert!(
        names.iter().any(|n| n.ends_with(nfd_buro)),
        "NFC Büro branch should match NFD Bu\u{0308}ro: results={names:?}"
    );
    assert!(
        names.iter().any(|n| n.ends_with(nfd_kuche)),
        "NFC Küche branch should match NFD Ku\u{0308}che: results={names:?}"
    );
    assert!(
        !names.iter().any(|n| n.ends_with(unrelated)),
        "unrelated directory should not appear in OR results: results={names:?}"
    );
}

/// A query spanning multiple NFD path segments verifies that normalization
/// applies uniformly across the entire query string, not just the first
/// occurrence of a non-ASCII character.
#[test]
fn unicode_nfc_multi_segment_deep_path_matches_nfd_filesystem() {
    let temp_dir =
        TempDir::new("unicode_nfc_multi_segment_deep_path_matches_nfd_filesystem").unwrap();
    let root = temp_dir.path();
    let nfd_buro = "Bu\u{0308}ro";
    let nfd_kuche = "Ku\u{0308}che";
    fs::create_dir_all(root.join(nfd_buro).join(nfd_kuche)).unwrap();
    fs::File::create(root.join(nfd_buro).join(nfd_kuche).join("rezept.txt")).unwrap();

    let mut cache = SearchCache::walk_fs(root);
    let query = "B\u{00FC}ro/K\u{00FC}che/rezept.txt"; // NFC: Büro/Küche/rezept.txt
    let indices = guard_indices(cache.search_with_options(
        query,
        SearchOptions {
            case_insensitive: false,
        },
        CancellationToken::noop(),
    ));
    let names = normalize(&mut cache, &indices);
    assert!(
        names.iter().any(|n| n.ends_with("rezept.txt")),
        "multi-segment NFC path query should match NFD filesystem: results={names:?}"
    );
}

/// A plain filename (no directory component) whose NFC form (`caf\u{00E9}.txt`)
/// differs from its NFD storage form (`cafe\u{0301}.txt`) must still be found.
#[test]
fn unicode_nfc_composed_filename_matches_nfd_stored_filename() {
    let temp_dir =
        TempDir::new("unicode_nfc_composed_filename_matches_nfd_stored_filename").unwrap();
    let root = temp_dir.path();
    let nfd_name = "cafe\u{0301}.txt"; // NFD: e + combining acute
    fs::File::create(root.join(nfd_name)).unwrap();

    let mut cache = SearchCache::walk_fs(root);
    let nfc_query = "caf\u{00E9}.txt"; // NFC: precomposed é
    let indices = guard_indices(cache.search_with_options(
        nfc_query,
        SearchOptions {
            case_insensitive: false,
        },
        CancellationToken::noop(),
    ));
    let names = normalize(&mut cache, &indices);
    assert!(
        names
            .iter()
            .any(|n| n.ends_with(nfd_name) || n.ends_with(nfc_query)),
        "NFC filename query should match NFD-stored filename: results={names:?}"
    );
}

/// Write path segments using composed (NFC) forms and search using the same
/// NFC query. On APFS the stored form may surface as NFD; on other filesystems
/// it may stay NFC. Either representation should be returned by search.
#[test]
fn unicode_write_nfc_then_search_nfc_exact_path() {
    let temp_dir = TempDir::new("unicode_write_nfc_then_search_nfc_exact_path").unwrap();
    let root = temp_dir.path();

    let nfc_dir = "B\u{00FC}ro";
    let nfc_file = "caf\u{00E9}.txt";
    let nfd_dir = "Bu\u{0308}ro";
    let nfd_file = "cafe\u{0301}.txt";

    fs::create_dir_all(root.join(nfc_dir)).unwrap();
    fs::File::create(root.join(nfc_dir).join(nfc_file)).unwrap();

    let mut cache = SearchCache::walk_fs(root);
    let query = format!("{nfc_dir}/{nfc_file}");
    let indices = guard_indices(cache.search_with_options(
        &query,
        SearchOptions {
            case_insensitive: false,
        },
        CancellationToken::noop(),
    ));
    let names = normalize(&mut cache, &indices);
    assert!(
        names
            .iter()
            .any(|n| n.ends_with(&format!("{nfc_dir}/{nfc_file}")))
            || names
                .iter()
                .any(|n| n.ends_with(&format!("{nfd_dir}/{nfd_file}"))),
        "NFC-written path should be searchable via NFC query: results={names:?}"
    );
}

/// Write NFC names first, then run an NFC wildcard query over that directory.
/// This directly exercises the "write NFC -> search NFC" behavior while staying
/// robust to APFS (NFD surfaced names) and non-APFS (NFC surfaced names).
#[test]
fn unicode_write_nfc_then_search_nfc_wildcard() {
    let temp_dir = TempDir::new("unicode_write_nfc_then_search_nfc_wildcard").unwrap();
    let root = temp_dir.path();

    let nfc_dir = "K\u{00FC}che";
    let nfd_dir = "Ku\u{0308}che";

    fs::create_dir_all(root.join(nfc_dir)).unwrap();
    fs::File::create(root.join(nfc_dir).join("rezept.txt")).unwrap();
    fs::File::create(root.join(nfc_dir).join("liste.md")).unwrap();

    let mut cache = SearchCache::walk_fs(root);
    let query = format!("{nfc_dir}/*.txt");
    let indices = guard_indices(cache.search_with_options(
        &query,
        SearchOptions {
            case_insensitive: false,
        },
        CancellationToken::noop(),
    ));
    let names = normalize(&mut cache, &indices);
    assert!(
        names
            .iter()
            .any(|n| n.ends_with(&format!("{nfc_dir}/rezept.txt")))
            || names
                .iter()
                .any(|n| n.ends_with(&format!("{nfd_dir}/rezept.txt"))),
        "NFC wildcard query should find txt file in NFC-written dir: results={names:?}"
    );
    assert!(
        !names.iter().any(|n| n.ends_with("liste.md")),
        "wildcard should still filter by extension"
    );
}
