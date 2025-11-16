#[cfg(test)]
mod extra {
    use crate::SearchCache;
    use search_cancel::CancellationToken;
    use std::{fs, path::PathBuf};
    use tempdir::TempDir;

    #[test]
    fn test_search_empty_returns_all_nodes() {
        let tmp = TempDir::new("search_empty").unwrap();
        fs::File::create(tmp.path().join("a.txt")).unwrap();
        fs::File::create(tmp.path().join("b.txt")).unwrap();
        let cache = SearchCache::walk_fs(tmp.path().to_path_buf());
        let all = cache
            .search_empty(CancellationToken::noop())
            .expect("noop cancellation token should not cancel");
        assert_eq!(all.len(), cache.get_total_files());
    }

    #[test]
    fn test_node_path_root_and_child() {
        let tmp = TempDir::new("node_path").unwrap();
        fs::create_dir(tmp.path().join("dir1")).unwrap();
        fs::File::create(tmp.path().join("dir1/file_x")).unwrap();
        let cache = SearchCache::walk_fs(tmp.path().to_path_buf());
        let idxs = cache.search("file_x").unwrap();
        assert_eq!(idxs.len(), 1);
        let full = cache.node_path(idxs.into_iter().next().unwrap()).unwrap();
        assert!(full.ends_with(PathBuf::from("dir1/file_x")));
    }

    #[test]
    fn test_remove_node_path_nonexistent_returns_none() {
        let tmp = TempDir::new("remove_node_none").unwrap();
        let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());
        // remove_node_path is private via crate; exercise via scan removal scenario
        // create then delete file and ensure second scan removal returns None
        let file = tmp.path().join("temp_remove.txt");
        fs::write(&file, b"x").unwrap();
        let id = cache.last_event_id() + 1;
        cache
            .handle_fs_events(vec![cardinal_sdk::FsEvent {
                path: file.clone(),
                id,
                flag: cardinal_sdk::EventFlag::ItemCreated,
            }])
            .unwrap();
        // delete file and send removal event => handle_fs_events will trigger internal removal
        fs::remove_file(&file).unwrap();
        let id2 = id + 1;
        cache
            .handle_fs_events(vec![cardinal_sdk::FsEvent {
                path: file.clone(),
                id: id2,
                flag: cardinal_sdk::EventFlag::ItemRemoved,
            }])
            .unwrap();
        assert!(cache.search("temp_remove.txt").unwrap().is_empty());
    }

    #[test]
    fn test_expand_file_nodes_fetch_metadata() {
        let tmp = TempDir::new("expand_meta").unwrap();
        fs::write(tmp.path().join("meta.txt"), b"hello world").unwrap();
        let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());
        let idxs = cache.search("meta.txt").unwrap();
        assert_eq!(idxs.len(), 1);
        // First query_files returns metadata None
        let q1 = cache
            .query_files("meta.txt".into(), CancellationToken::noop())
            .expect("query should succeed")
            .expect("noop cancellation token should not cancel");
        assert_eq!(q1.len(), 1);
        assert!(q1[0].metadata.is_none());
        // expand_file_nodes should fetch metadata
        let nodes = cache.expand_file_nodes(&idxs);
        assert_eq!(nodes.len(), 1);
        assert!(
            nodes[0].metadata.is_some(),
            "metadata should be fetched on demand"
        );
        // A second expand should still have metadata (cached)
        let nodes2 = cache.expand_file_nodes(&idxs);
        assert!(nodes2[0].metadata.is_some());
    }

    #[test]
    fn test_persistent_roundtrip() {
        let tmp = TempDir::new("persist_round").unwrap();
        fs::write(tmp.path().join("a.bin"), b"data").unwrap();
        let cache_path = tmp.path().join("cache.zstd");
        let cache = SearchCache::walk_fs(tmp.path().to_path_buf());
        let original_total = cache.get_total_files();
        cache.flush_to_file(&cache_path).unwrap();
        let loaded =
            SearchCache::try_read_persistent_cache(tmp.path(), &cache_path, None, None).unwrap();
        assert_eq!(loaded.get_total_files(), original_total);
    }

    #[test]
    fn test_query_and_or_not_dedup_and_filtering() {
        let tmp = TempDir::new("query_bool").unwrap();
        fs::write(tmp.path().join("report.txt"), b"r").unwrap();
        fs::write(tmp.path().join("report.md"), b"r").unwrap();
        fs::write(tmp.path().join("other.txt"), b"o").unwrap();
        let cache = SearchCache::walk_fs(tmp.path().to_path_buf());

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
    fn test_regex_prefix_in_queries() {
        let tmp = TempDir::new("query_regex").unwrap();
        fs::write(tmp.path().join("Report Q1.md"), b"x").unwrap();
        fs::write(tmp.path().join("Report Q2.txt"), b"x").unwrap();
        fs::write(tmp.path().join("notes.txt"), b"x").unwrap();
        let cache = SearchCache::walk_fs(tmp.path().to_path_buf());

        let idxs = cache.search("regex:^Report").unwrap();
        assert_eq!(idxs.len(), 2);
    }

    #[test]
    fn test_ext_list_and_intersection() {
        let tmp = TempDir::new("query_ext_list").unwrap();
        fs::write(tmp.path().join("a.txt"), b"x").unwrap();
        fs::write(tmp.path().join("b.md"), b"x").unwrap();
        fs::write(tmp.path().join("c.rs"), b"x").unwrap();
        let cache = SearchCache::walk_fs(tmp.path().to_path_buf());

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
        let cache = SearchCache::walk_fs(tmp.path().to_path_buf());

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
        let cache = SearchCache::walk_fs(tmp.path().to_path_buf());

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
        let cache = SearchCache::walk_fs(tmp.path().to_path_buf());

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
        let cache = SearchCache::walk_fs(tmp.path().to_path_buf());

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
}
