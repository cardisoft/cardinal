use super::prelude::*;

#[test]
fn test_all_subnodes_returns_all_descendants() {
    let tmp = TempDir::new("all_subnodes").unwrap();
    // Create nested structure:
    // root/
    //   a.txt
    //   src/
    //     main.rs
    //     lib.rs
    //     utils/
    //       helper.rs
    fs::write(tmp.path().join("a.txt"), b"x").unwrap();
    fs::create_dir(tmp.path().join("src")).unwrap();
    fs::write(tmp.path().join("src/main.rs"), b"x").unwrap();
    fs::write(tmp.path().join("src/lib.rs"), b"x").unwrap();
    fs::create_dir(tmp.path().join("src/utils")).unwrap();
    fs::write(tmp.path().join("src/utils/helper.rs"), b"x").unwrap();

    let cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    // Find src directory index
    let src_path = tmp.path().join("src");
    let src_idx = cache
        .node_index_for_raw_path(&src_path)
        .expect("src directory should exist");

    // Get all subnodes
    let subnodes = cache
        .all_subnodes(src_idx, CancellationToken::noop())
        .expect("Should return subnodes");

    // Should include: main.rs, lib.rs, utils/, helper.rs (4 items)
    assert_eq!(subnodes.len(), 4, "Should return all 4 descendants of src");

    // Verify all returned nodes are under src
    for &node_idx in &subnodes {
        let node_path = cache.node_path(node_idx).expect("Node should have path");
        assert!(
            node_path.starts_with(&src_path),
            "All subnodes should be under src"
        );
    }
}

#[test]
fn test_all_subnodes_empty_directory() {
    let tmp = TempDir::new("all_subnodes_empty").unwrap();
    fs::create_dir(tmp.path().join("empty")).unwrap();

    let cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    let empty_path = tmp.path().join("empty");
    let empty_idx = cache
        .node_index_for_raw_path(&empty_path)
        .expect("empty directory should exist");

    let subnodes = cache
        .all_subnodes(empty_idx, CancellationToken::noop())
        .expect("Should return empty vec");

    assert_eq!(subnodes.len(), 0, "Empty directory should have no subnodes");
}

#[test]
fn test_all_subnodes_deep_nesting() {
    let tmp = TempDir::new("all_subnodes_deep").unwrap();
    // Create deep nesting: a/b/c/d/file.txt
    let deep_path = tmp.path().join("a/b/c/d");
    fs::create_dir_all(&deep_path).unwrap();
    fs::write(deep_path.join("file.txt"), b"x").unwrap();

    let cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    // Get subnodes from 'a' directory
    let a_path = tmp.path().join("a");
    let a_idx = cache
        .node_index_for_raw_path(&a_path)
        .expect("a directory should exist");

    let subnodes = cache
        .all_subnodes(a_idx, CancellationToken::noop())
        .expect("Should return subnodes");

    // Should include: b/, c/, d/, file.txt (4 items)
    assert_eq!(
        subnodes.len(),
        4,
        "Should recursively return all nested items"
    );

    // Verify the deepest file is included
    let has_file = subnodes.iter().any(|&idx| {
        cache
            .node_path(idx)
            .map(|p| p.ends_with("file.txt"))
            .unwrap_or(false)
    });
    assert!(has_file, "Should include deeply nested file");
}

#[test]
fn test_all_subnodes_cancellation() {
    let tmp = TempDir::new("all_subnodes_cancel").unwrap();
    // Create many files to test cancellation
    for i in 0..100 {
        fs::write(tmp.path().join(format!("file_{i}.txt")), b"x").unwrap();
    }

    let cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    let root_idx = cache.file_nodes.root();

    // Create a cancelled token by creating a newer version
    let token = CancellationToken::new(1);
    let _newer_token = CancellationToken::new(2); // This cancels the first token

    // Should return None when cancelled
    let result = cache.all_subnodes(root_idx, token);
    assert!(result.is_none(), "Should return None when cancelled");
}
