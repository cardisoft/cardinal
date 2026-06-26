use search_cache::{FlatEntry, FlatIndex, SlabIndex, SlabNodeMetadataCompact};
use std::path::Path;

fn entry(path: &str, slab_idx: usize) -> FlatEntry {
    let path = Path::new(path);
    let path_str = path.to_string_lossy();
    let interned_path = search_cache::PATH_POOL.push(path_str.as_ref());
    let name = path
        .file_name()
        .map(|n| search_cache::NAME_POOL.push(n.to_string_lossy().as_ref()))
        .unwrap_or_else(|| search_cache::NAME_POOL.push(""));
    FlatEntry {
        path: interned_path,
        name,
        slab_index: SlabIndex::new(slab_idx),
        metadata: SlabNodeMetadataCompact::none(),
    }
}

fn build_test_index() -> FlatIndex {
    // Sorted by path:
    let entries = vec![
        entry("/Users/demo", 0),
        entry("/Users/demo/file1.txt", 1),
        entry("/Users/demo/src", 2),
        entry("/Users/demo/src/main.rs", 3),
        entry("/Users/demo/src/lib.rs", 4),
        entry("/Users/demo/src/utils", 5),
        entry("/Users/demo/src/utils/helper.rs", 6),
        entry("/Users/demo/tests", 7),
        entry("/Users/demo/tests/test1.rs", 8),
        entry("/Users/other/readme.md", 9),
    ];
    FlatIndex::build_from_entries(entries)
}

#[test]
fn prefix_range_finds_descendants() {
    let index = build_test_index();

    let range = index.prefix_range("/Users/demo/src");
    // Should match: src, src/main.rs, src/lib.rs, src/utils, src/utils/helper.rs
    assert_eq!(range.end - range.start, 5);

    let indices = index.prefix_indices("/Users/demo/src");
    for idx in &indices {
        let path = index.node_path(*idx).unwrap();
        assert!(path.starts_with("/Users/demo/src"));
    }
}

#[test]
fn prefix_range_exact_match_included() {
    let index = build_test_index();

    // /Users/demo itself starts with /Users/demo
    let range = index.prefix_range("/Users/demo");
    // All 9 entries under /Users/demo (including /Users/demo itself)
    assert_eq!(range.end - range.start, 9);
}

#[test]
fn prefix_range_no_match() {
    let index = build_test_index();
    let range = index.prefix_range("/Users/nonexistent");
    assert_eq!(range.end - range.start, 0);
}

#[test]
fn name_index_lookups() {
    let index = build_test_index();

    // "main.rs" should map to exactly one entry
    let indices = index.name_index.get("main.rs").unwrap();
    assert_eq!(indices.len(), 1);
    let path = index.node_path(indices[0]).unwrap();
    assert_eq!(path, std::path::PathBuf::from("/Users/demo/src/main.rs"));

    // No match
    assert!(index.name_index.get("nonexistent.rs").is_none());
}

#[test]
fn all_indices_returns_everything() {
    let index = build_test_index();
    let all = index.all_indices();
    assert_eq!(all.len(), 10);
}

#[test]
fn node_path_is_o1() {
    let index = build_test_index();
    let path = index.node_path(SlabIndex::new(3)).unwrap();
    assert_eq!(path, std::path::PathBuf::from("/Users/demo/src/main.rs"));
}

#[test]
fn node_name_is_o1() {
    let index = build_test_index();
    let name = index.node_name(SlabIndex::new(3)).unwrap();
    assert_eq!(name, "main.rs");
}

#[test]
fn remove_prefix_removes_subtree() {
    let mut index = build_test_index();
    let removed = index.remove_prefix("/Users/demo/src");
    // src, src/main.rs, src/lib.rs, src/utils, src/utils/helper.rs = 5
    assert_eq!(removed, 5);
    assert_eq!(index.len(), 5);

    // Remaining: /Users/demo, /Users/demo/file1.txt, /Users/demo/tests,
    // /Users/demo/tests/test1.rs, /Users/other/readme.md
    let remaining: Vec<_> = index.iter().map(|(_, e)| e.path.to_string()).collect();
    assert!(remaining.contains(&"/Users/demo".to_string()));
    assert!(remaining.contains(&"/Users/other/readme.md".to_string()));
    assert!(!remaining.iter().any(|p| p.contains("src")));
}

#[test]
fn insert_maintains_sort_order() {
    let mut index = build_test_index();
    index.insert(entry("/Users/demo/src/new.rs", 10));

    // Should be inserted between src/lib.rs and src/utils
    let range = index.prefix_range("/Users/demo/src/");
    let paths: Vec<_> = (range.start..range.end)
        .map(|i| index.get_by_pos(i).unwrap().path.to_string())
        .collect();
    assert!(paths.contains(&"/Users/demo/src/new.rs".to_string()));
}

#[test]
fn prefix_range_handles_trailing_slash() {
    let index = build_test_index();

    // "/Users/demo/src/" should match descendants but not "src" itself
    let range = index.prefix_range("/Users/demo/src/");
    // src/main.rs, src/lib.rs, src/utils, src/utils/helper.rs = 4
    assert_eq!(range.end - range.start, 4);
}
