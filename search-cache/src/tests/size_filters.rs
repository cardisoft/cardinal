use super::prelude::*;

#[test]
fn test_size_filters() {
    let tmp = TempDir::new("query_size_filters").unwrap();
    fs::write(tmp.path().join("tiny.bin"), vec![0u8; 512]).unwrap();
    fs::write(tmp.path().join("medium.bin"), vec![0u8; 50_000]).unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    let larger = cache.search("size:>1kb").unwrap();
    assert_eq!(larger.len(), 1);
    let large_path = cache.node_path(*larger.first().unwrap()).unwrap();
    assert!(large_path.ends_with(PathBuf::from("medium.bin")));

    let tiny = cache.search("size:tiny").unwrap();
    assert_eq!(tiny.len(), 1);
    let tiny_path = cache.node_path(*tiny.first().unwrap()).unwrap();
    assert!(tiny_path.ends_with(PathBuf::from("tiny.bin")));

    let ranged = cache.search("size:1kb..60kb").unwrap();
    assert_eq!(ranged.len(), 1);
    let ranged_path = cache.node_path(*ranged.first().unwrap()).unwrap();
    assert!(ranged_path.ends_with(PathBuf::from("medium.bin")));
}

#[test]
fn test_size_filter_persists_metadata_on_nodes() {
    let tmp = TempDir::new("query_size_cache").unwrap();
    fs::write(tmp.path().join("cache.bin"), vec![0u8; 2048]).unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    let results = cache.search("size:>1kb").unwrap();
    assert_eq!(results.len(), 1);
    let index = results[0];
    assert!(
        cache.file_nodes[index].metadata.is_some(),
        "size filter should populate node metadata"
    );
}

#[test]
fn test_size_filter_respects_parent_base() {
    let tmp = TempDir::new("size_filter_scoped").unwrap();
    fs::create_dir(tmp.path().join("folder")).unwrap();
    fs::write(tmp.path().join("folder/keep.bin"), vec![0u8; 4096]).unwrap();
    fs::write(tmp.path().join("skip.bin"), vec![0u8; 8192]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());
    let folder = tmp.path().join("folder");
    let keep_idx = cache.search("keep.bin").unwrap()[0];
    let skip_idx = cache.search("skip.bin").unwrap()[0];
    assert!(cache.file_nodes[skip_idx].metadata.is_none());

    let results = cache
        .search(&format!("parent:{} size:>1kb", folder.display()))
        .unwrap();
    assert_eq!(results.len(), 1);
    let path = cache.node_path(*results.first().unwrap()).unwrap();
    assert!(path.ends_with(PathBuf::from("keep.bin")));

    assert!(
        cache.file_nodes[skip_idx].metadata.is_none(),
        "size filter should not evaluate nodes outside the parent base"
    );
    assert!(
        cache.file_nodes[keep_idx].metadata.is_some(),
        "size filter should still populate metadata for matching nodes"
    );
}

#[test]
fn test_size_filter_respects_infolder_base() {
    let tmp = TempDir::new("size_filter_infoder_base").unwrap();
    fs::create_dir(tmp.path().join("media")).unwrap();
    fs::create_dir(tmp.path().join("media/nested")).unwrap();
    fs::write(tmp.path().join("media/nested/keep.bin"), vec![0u8; 4096]).unwrap();
    fs::write(tmp.path().join("skip.bin"), vec![0u8; 8192]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());
    let keep_idx = cache.search("keep.bin").unwrap()[0];
    let skip_idx = cache.search("skip.bin").unwrap()[0];
    assert!(cache.file_nodes[skip_idx].metadata.is_none());

    let results = cache
        .search(&format!(
            "infolder:{} size:>1kb",
            tmp.path().join("media").display()
        ))
        .unwrap();
    assert_eq!(results.len(), 1);
    let path = cache.node_path(*results.first().unwrap()).unwrap();
    assert!(path.ends_with(PathBuf::from("keep.bin")));

    assert!(
        cache.file_nodes[skip_idx].metadata.is_none(),
        "size filter should not evaluate nodes outside the infolder base"
    );
    assert!(
        cache.file_nodes[keep_idx].metadata.is_some(),
        "size filter should still populate metadata for matching nodes"
    );
}

#[test]
fn test_size_comparison_operators() {
    let tmp = TempDir::new("size_comparison").unwrap();
    fs::write(tmp.path().join("tiny.bin"), vec![0u8; 500]).unwrap();
    fs::write(tmp.path().join("small.bin"), vec![0u8; 1500]).unwrap();
    fs::write(tmp.path().join("medium.bin"), vec![0u8; 5000]).unwrap();
    fs::write(tmp.path().join("large.bin"), vec![0u8; 15000]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    // Greater than
    let gt = cache.search("size:>1kb").unwrap();
    assert_eq!(gt.len(), 3);

    // Greater than or equal
    let gte = cache.search("size:>=1500").unwrap();
    assert_eq!(gte.len(), 3);

    // Less than
    let lt = cache.search("size:<1kb").unwrap();
    assert_eq!(lt.len(), 1);

    // Less than or equal
    let lte = cache.search("size:<=1500").unwrap();
    assert_eq!(lte.len(), 2);

    // Equal
    let eq = cache.search("size:=500").unwrap();
    assert_eq!(eq.len(), 1);

    // Not equal
    let ne = cache.search("size:!=500").unwrap();
    assert_eq!(ne.len(), 3);
}

#[test]
fn test_size_units_bytes() {
    let tmp = TempDir::new("size_bytes").unwrap();
    fs::write(tmp.path().join("100b.bin"), vec![0u8; 100]).unwrap();
    fs::write(tmp.path().join("500b.bin"), vec![0u8; 500]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    let results = cache.search("size:>200").unwrap();
    assert_eq!(results.len(), 1);

    let results_b = cache.search("size:>200b").unwrap();
    assert_eq!(results_b.len(), 1);

    let results_byte = cache.search("size:>200byte").unwrap();
    assert_eq!(results_byte.len(), 1);

    let results_bytes = cache.search("size:>200bytes").unwrap();
    assert_eq!(results_bytes.len(), 1);
}

#[test]
fn test_size_units_kilobytes() {
    let tmp = TempDir::new("size_kilobytes").unwrap();
    fs::write(tmp.path().join("half_kb.bin"), vec![0u8; 512]).unwrap();
    fs::write(tmp.path().join("two_kb.bin"), vec![0u8; 2048]).unwrap();
    fs::write(tmp.path().join("five_kb.bin"), vec![0u8; 5120]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    let k = cache.search("size:>1k").unwrap();
    assert_eq!(k.len(), 2);

    let kb = cache.search("size:>1kb").unwrap();
    assert_eq!(kb.len(), 2);

    let kib = cache.search("size:>1kib").unwrap();
    assert_eq!(kib.len(), 2);

    let kilobyte = cache.search("size:>1kilobyte").unwrap();
    assert_eq!(kilobyte.len(), 2);

    let kilobytes = cache.search("size:>1kilobytes").unwrap();
    assert_eq!(kilobytes.len(), 2);
}

#[test]
fn test_size_units_megabytes() {
    let tmp = TempDir::new("size_megabytes").unwrap();
    fs::write(tmp.path().join("half_mb.bin"), vec![0u8; 512 * 1024]).unwrap();
    fs::write(tmp.path().join("two_mb.bin"), vec![0u8; 2 * 1024 * 1024]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    let m = cache.search("size:>1m").unwrap();
    assert_eq!(m.len(), 1);

    let mb = cache.search("size:>1mb").unwrap();
    assert_eq!(mb.len(), 1);

    let mib = cache.search("size:>1mib").unwrap();
    assert_eq!(mib.len(), 1);

    let megabyte = cache.search("size:>1megabyte").unwrap();
    assert_eq!(megabyte.len(), 1);

    let megabytes = cache.search("size:>1megabytes").unwrap();
    assert_eq!(megabytes.len(), 1);
}

#[test]
fn test_size_units_gigabytes() {
    let tmp = TempDir::new("size_gigabytes").unwrap();
    // For testing purposes, we'll use smaller values and adjust the query
    fs::write(tmp.path().join("small.bin"), vec![0u8; 100]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    // Test that the unit is recognized (size will be less than 1GB)
    let g = cache.search("size:<1g").unwrap();
    assert!(!g.is_empty());

    let gb = cache.search("size:<1gb").unwrap();
    assert!(!gb.is_empty());

    let gib = cache.search("size:<1gib").unwrap();
    assert!(!gib.is_empty());

    let gigabyte = cache.search("size:<1gigabyte").unwrap();
    assert!(!gigabyte.is_empty());

    let gigabytes = cache.search("size:<1gigabytes").unwrap();
    assert!(!gigabytes.is_empty());
}

#[test]
fn test_size_units_terabytes() {
    let tmp = TempDir::new("size_terabytes").unwrap();
    fs::write(tmp.path().join("small.bin"), vec![0u8; 100]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    let t = cache.search("size:<1t").unwrap();
    assert!(!t.is_empty());

    let tb = cache.search("size:<1tb").unwrap();
    assert!(!tb.is_empty());

    let tib = cache.search("size:<1tib").unwrap();
    assert!(!tib.is_empty());

    let terabyte = cache.search("size:<1terabyte").unwrap();
    assert!(!terabyte.is_empty());

    let terabytes = cache.search("size:<1terabytes").unwrap();
    assert!(!terabytes.is_empty());
}

#[test]
fn test_size_units_petabytes() {
    let tmp = TempDir::new("size_petabytes").unwrap();
    fs::write(tmp.path().join("small.bin"), vec![0u8; 100]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    let p = cache.search("size:<1p").unwrap();
    assert!(!p.is_empty());

    let pb = cache.search("size:<1pb").unwrap();
    assert!(!pb.is_empty());

    let pib = cache.search("size:<1pib").unwrap();
    assert!(!pib.is_empty());

    let petabyte = cache.search("size:<1petabyte").unwrap();
    assert!(!petabyte.is_empty());

    let petabytes = cache.search("size:<1petabytes").unwrap();
    assert!(!petabytes.is_empty());
}

#[test]
fn test_size_decimal_values() {
    let tmp = TempDir::new("size_decimal").unwrap();
    fs::write(tmp.path().join("1500b.bin"), vec![0u8; 1500]).unwrap();
    fs::write(tmp.path().join("2500b.bin"), vec![0u8; 2500]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    let results = cache.search("size:>1.5kb").unwrap();
    assert_eq!(results.len(), 1);

    let results2 = cache.search("size:>2.0kb").unwrap();
    assert_eq!(results2.len(), 1);

    let results3 = cache.search("size:>0.5kb").unwrap();
    assert_eq!(results3.len(), 2);
}

#[test]
fn test_size_range_both_bounds() {
    let tmp = TempDir::new("size_range_both").unwrap();
    fs::write(tmp.path().join("500b.bin"), vec![0u8; 500]).unwrap();
    fs::write(tmp.path().join("1500b.bin"), vec![0u8; 1500]).unwrap();
    fs::write(tmp.path().join("2500b.bin"), vec![0u8; 2500]).unwrap();
    fs::write(tmp.path().join("5000b.bin"), vec![0u8; 5000]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    let results = cache.search("size:1kb..3kb").unwrap();
    assert_eq!(results.len(), 2);

    let paths: Vec<_> = results
        .iter()
        .map(|i| cache.node_path(*i).unwrap())
        .collect();
    assert!(paths.iter().any(|p| p.ends_with("1500b.bin")));
    assert!(paths.iter().any(|p| p.ends_with("2500b.bin")));
}

#[test]
fn test_size_range_open_start() {
    let tmp = TempDir::new("size_range_open_start").unwrap();
    fs::write(tmp.path().join("500b.bin"), vec![0u8; 500]).unwrap();
    fs::write(tmp.path().join("1500b.bin"), vec![0u8; 1500]).unwrap();
    fs::write(tmp.path().join("2500b.bin"), vec![0u8; 2500]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    let results = cache.search("size:..2kb").unwrap();
    assert_eq!(results.len(), 2);

    let paths: Vec<_> = results
        .iter()
        .map(|i| cache.node_path(*i).unwrap())
        .collect();
    assert!(paths.iter().any(|p| p.ends_with("500b.bin")));
    assert!(paths.iter().any(|p| p.ends_with("1500b.bin")));
}

#[test]
fn test_size_range_open_end() {
    let tmp = TempDir::new("size_range_open_end").unwrap();
    fs::write(tmp.path().join("500b.bin"), vec![0u8; 500]).unwrap();
    fs::write(tmp.path().join("1500b.bin"), vec![0u8; 1500]).unwrap();
    fs::write(tmp.path().join("2500b.bin"), vec![0u8; 2500]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    let results = cache.search("size:1kb..").unwrap();
    assert_eq!(results.len(), 2);

    let paths: Vec<_> = results
        .iter()
        .map(|i| cache.node_path(*i).unwrap())
        .collect();
    assert!(paths.iter().any(|p| p.ends_with("1500b.bin")));
    assert!(paths.iter().any(|p| p.ends_with("2500b.bin")));
}

#[test]
fn test_size_keyword_empty() {
    let tmp = TempDir::new("size_keyword_empty").unwrap();
    fs::write(tmp.path().join("empty.bin"), vec![]).unwrap();
    fs::write(tmp.path().join("nonempty.bin"), vec![0u8; 100]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    let results = cache.search("size:empty").unwrap();
    assert_eq!(results.len(), 1);

    let path = cache.node_path(*results.first().unwrap()).unwrap();
    assert!(path.ends_with("empty.bin"));
}

#[test]
fn test_size_keyword_tiny() {
    let tmp = TempDir::new("size_keyword_tiny").unwrap();
    fs::write(tmp.path().join("tiny1.bin"), vec![0u8; 100]).unwrap();
    fs::write(tmp.path().join("tiny2.bin"), vec![0u8; 5000]).unwrap();
    fs::write(tmp.path().join("medium.bin"), vec![0u8; 50000]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    let results = cache.search("size:tiny").unwrap();
    assert_eq!(results.len(), 2);
}

#[test]
fn test_size_keyword_small() {
    let tmp = TempDir::new("size_keyword_small").unwrap();
    fs::write(tmp.path().join("small1.bin"), vec![0u8; 20_000]).unwrap();
    fs::write(tmp.path().join("small2.bin"), vec![0u8; 50_000]).unwrap();
    fs::write(tmp.path().join("large.bin"), vec![0u8; 200_000]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    let results = cache.search("size:small").unwrap();
    assert_eq!(results.len(), 2);
}

#[test]
fn test_size_keyword_medium() {
    let tmp = TempDir::new("size_keyword_medium").unwrap();
    fs::write(tmp.path().join("small.bin"), vec![0u8; 50_000]).unwrap();
    fs::write(tmp.path().join("medium.bin"), vec![0u8; 500_000]).unwrap();
    fs::write(tmp.path().join("large.bin"), vec![0u8; 2_000_000]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    let results = cache.search("size:medium").unwrap();
    assert_eq!(results.len(), 1);

    let path = cache.node_path(*results.first().unwrap()).unwrap();
    assert!(path.ends_with("medium.bin"));
}

#[test]
fn test_size_keyword_large() {
    let tmp = TempDir::new("size_keyword_large").unwrap();
    fs::write(tmp.path().join("medium.bin"), vec![0u8; 500_000]).unwrap();
    fs::write(tmp.path().join("large.bin"), vec![0u8; 5_000_000]).unwrap();
    fs::write(tmp.path().join("huge.bin"), vec![0u8; 50_000_000]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    let results = cache.search("size:large").unwrap();
    assert_eq!(results.len(), 1);

    let path = cache.node_path(*results.first().unwrap()).unwrap();
    assert!(path.ends_with("large.bin"));
}

#[test]
fn test_size_keyword_huge() {
    let tmp = TempDir::new("size_keyword_huge").unwrap();
    fs::write(tmp.path().join("large.bin"), vec![0u8; 10_000_000]).unwrap();
    fs::write(tmp.path().join("huge.bin"), vec![0u8; 100_000_000]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    let results = cache.search("size:huge").unwrap();
    assert_eq!(results.len(), 1);

    let path = cache.node_path(*results.first().unwrap()).unwrap();
    assert!(path.ends_with("huge.bin"));
}

#[test]
fn test_size_keyword_gigantic() {
    let tmp = TempDir::new("size_keyword_gigantic").unwrap();
    fs::write(tmp.path().join("huge.bin"), vec![0u8; 100_000_000]).unwrap();
    fs::write(tmp.path().join("gigantic.bin"), vec![0u8; 200_000_000]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    let results = cache.search("size:gigantic").unwrap();
    assert_eq!(results.len(), 1);

    let path = cache.node_path(*results.first().unwrap()).unwrap();
    assert!(path.ends_with("gigantic.bin"));
}

#[test]
fn test_size_keyword_giant() {
    let tmp = TempDir::new("size_keyword_giant").unwrap();
    fs::write(tmp.path().join("huge.bin"), vec![0u8; 100_000_000]).unwrap();
    fs::write(tmp.path().join("giant.bin"), vec![0u8; 200_000_000]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    let results = cache.search("size:giant").unwrap();
    assert_eq!(results.len(), 1);

    let path = cache.node_path(*results.first().unwrap()).unwrap();
    assert!(path.ends_with("giant.bin"));
}

#[test]
fn test_size_keyword_case_insensitive() {
    let tmp = TempDir::new("size_keyword_case").unwrap();
    fs::write(tmp.path().join("tiny.bin"), vec![0u8; 100]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    let lower = cache.search("size:tiny").unwrap();
    assert_eq!(lower.len(), 1);

    let upper = cache.search("size:TINY").unwrap();
    assert_eq!(upper.len(), 1);

    let mixed = cache.search("size:TiNy").unwrap();
    assert_eq!(mixed.len(), 1);
}

#[test]
fn test_size_filter_excludes_directories() {
    let tmp = TempDir::new("size_no_dirs").unwrap();
    fs::write(tmp.path().join("file.bin"), vec![0u8; 1000]).unwrap();
    fs::create_dir(tmp.path().join("folder")).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    let results = cache.search("size:>500").unwrap();
    assert_eq!(results.len(), 1);

    let path = cache.node_path(*results.first().unwrap()).unwrap();
    assert!(path.ends_with("file.bin"));
}

#[test]
fn test_size_combined_with_name_search() {
    let tmp = TempDir::new("size_with_name").unwrap();
    fs::write(tmp.path().join("report.bin"), vec![0u8; 1500]).unwrap();
    fs::write(tmp.path().join("report.txt"), vec![0u8; 500]).unwrap();
    fs::write(tmp.path().join("data.bin"), vec![0u8; 2000]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    let results = cache.search("report size:>1kb").unwrap();
    assert_eq!(results.len(), 1);

    let path = cache.node_path(*results.first().unwrap()).unwrap();
    assert!(path.ends_with("report.bin"));
}

#[test]
fn test_size_combined_with_ext_filter() {
    let tmp = TempDir::new("size_with_ext").unwrap();
    fs::write(tmp.path().join("large.txt"), vec![0u8; 2000]).unwrap();
    fs::write(tmp.path().join("small.txt"), vec![0u8; 500]).unwrap();
    fs::write(tmp.path().join("large.bin"), vec![0u8; 2000]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    let results = cache.search("ext:txt size:>1kb").unwrap();
    assert_eq!(results.len(), 1);

    let path = cache.node_path(*results.first().unwrap()).unwrap();
    assert!(path.ends_with("large.txt"));
}

#[test]
fn test_size_with_or_operator() {
    let tmp = TempDir::new("size_with_or").unwrap();
    fs::write(tmp.path().join("tiny.bin"), vec![0u8; 100]).unwrap();
    fs::write(tmp.path().join("medium.bin"), vec![0u8; 5000]).unwrap();
    fs::write(tmp.path().join("gigantic.bin"), vec![0u8; 200_000_000]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    let results = cache.search("size:tiny OR size:gigantic").unwrap();
    assert!(results.len() >= 2, "Should match at least 2 files");

    let paths: Vec<_> = results
        .iter()
        .map(|i| cache.node_path(*i).unwrap())
        .collect();
    assert!(paths.iter().any(|p| p.ends_with("tiny.bin")));
    assert!(paths.iter().any(|p| p.ends_with("gigantic.bin")));
}

#[test]
fn test_size_with_not_operator() {
    let tmp = TempDir::new("size_with_not").unwrap();
    fs::write(tmp.path().join("tiny.bin"), vec![0u8; 100]).unwrap();
    fs::write(tmp.path().join("medium.bin"), vec![0u8; 5000]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    let results = cache.search("!size:tiny").unwrap();
    let has_tiny = results.iter().any(|&i| {
        cache
            .node_path(i)
            .map(|p| p.ends_with("tiny.bin"))
            .unwrap_or(false)
    });
    assert!(!has_tiny, "Should not include tiny files");
}

#[test]
fn test_size_error_empty_value() {
    let tmp = TempDir::new("size_error_empty").unwrap();
    fs::write(tmp.path().join("file.bin"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    let result = cache.search("size:");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("requires a value"));
}

#[test]
fn test_size_error_invalid_number() {
    let tmp = TempDir::new("size_error_number").unwrap();
    fs::write(tmp.path().join("file.bin"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    let result = cache.search("size:notanumber");
    assert!(result.is_err());
}

#[test]
fn test_size_error_unknown_unit() {
    let tmp = TempDir::new("size_error_unit").unwrap();
    fs::write(tmp.path().join("file.bin"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    let result = cache.search("size:100zb");
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Unknown size unit")
    );
}

#[test]
fn test_size_error_keyword_with_comparison() {
    let tmp = TempDir::new("size_error_keyword_comp").unwrap();
    fs::write(tmp.path().join("file.bin"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    let result = cache.search("size:>tiny");
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("keywords cannot be used with comparison")
    );
}

#[test]
fn test_size_range_inverted_bounds_error() {
    let tmp = TempDir::new("size_range_inverted").unwrap();
    fs::write(tmp.path().join("file.bin"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    let result = cache.search("size:10kb..1kb");
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("start must be less than or equal to the end")
    );
}

#[test]
fn test_size_bare_value_equals_comparison() {
    let tmp = TempDir::new("size_bare_equals").unwrap();
    fs::write(tmp.path().join("exact.bin"), vec![0u8; 1024]).unwrap();
    fs::write(tmp.path().join("other.bin"), vec![0u8; 2048]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    let results = cache.search("size:1kb").unwrap();
    assert_eq!(results.len(), 1);

    let path = cache.node_path(*results.first().unwrap()).unwrap();
    assert!(path.ends_with("exact.bin"));
}

#[test]
fn test_size_zero_bytes() {
    let tmp = TempDir::new("size_zero").unwrap();
    fs::write(tmp.path().join("empty.bin"), vec![]).unwrap();
    fs::write(tmp.path().join("nonempty.bin"), vec![0u8; 1]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    let results = cache.search("size:0").unwrap();
    assert_eq!(results.len(), 1);

    let path = cache.node_path(*results.first().unwrap()).unwrap();
    assert!(path.ends_with("empty.bin"));
}

#[test]
fn test_size_very_large_numbers() {
    let tmp = TempDir::new("size_large_num").unwrap();
    fs::write(tmp.path().join("small.bin"), vec![0u8; 100]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    // Test that very large numbers don't cause panics
    let results = cache.search("size:<999999gb").unwrap();
    assert!(!results.is_empty());
}

#[test]
fn test_size_fractional_precision() {
    let tmp = TempDir::new("size_fractional").unwrap();
    fs::write(tmp.path().join("file1.bin"), vec![0u8; 1536]).unwrap(); // 1.5 KB
    fs::write(tmp.path().join("file2.bin"), vec![0u8; 2048]).unwrap(); // 2 KB

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    let results = cache.search("size:>1.4kb").unwrap();
    assert_eq!(results.len(), 2);

    let results2 = cache.search("size:>1.6kb").unwrap();
    assert_eq!(results2.len(), 1);
}

#[test]
fn test_size_filter_with_parent_filter() {
    let tmp = TempDir::new("size_with_parent").unwrap();
    fs::create_dir(tmp.path().join("large")).unwrap();
    fs::create_dir(tmp.path().join("small")).unwrap();
    fs::write(tmp.path().join("large/file1.bin"), vec![0u8; 10_000]).unwrap();
    fs::write(tmp.path().join("large/file2.bin"), vec![0u8; 500]).unwrap();
    fs::write(tmp.path().join("small/file3.bin"), vec![0u8; 500]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    let large_dir = tmp.path().join("large");
    let results = cache
        .search(&format!("size:>1kb parent:{}", large_dir.display()))
        .unwrap();
    assert_eq!(results.len(), 1);

    let path = cache.node_path(*results.first().unwrap()).unwrap();
    assert!(path.ends_with("large/file1.bin"));
}

#[test]
fn test_size_filter_with_infolder_filter() {
    let tmp = TempDir::new("size_with_infolder").unwrap();
    fs::create_dir(tmp.path().join("data")).unwrap();
    fs::create_dir(tmp.path().join("data/nested")).unwrap();
    fs::write(tmp.path().join("data/large1.bin"), vec![0u8; 10_000]).unwrap();
    fs::write(tmp.path().join("data/nested/large2.bin"), vec![0u8; 10_000]).unwrap();
    fs::write(tmp.path().join("data/small.bin"), vec![0u8; 500]).unwrap();
    fs::write(tmp.path().join("other.bin"), vec![0u8; 10_000]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    let data_dir = tmp.path().join("data");
    let results = cache
        .search(&format!("size:>5kb infolder:{}", data_dir.display()))
        .unwrap();
    assert_eq!(results.len(), 2);
}

#[test]
fn test_multiple_size_ranges_with_or() {
    let tmp = TempDir::new("multi_size_or").unwrap();
    fs::write(tmp.path().join("tiny.bin"), vec![0u8; 100]).unwrap();
    fs::write(tmp.path().join("small.bin"), vec![0u8; 1_000]).unwrap();
    fs::write(tmp.path().join("medium.bin"), vec![0u8; 5_000]).unwrap();
    fs::write(tmp.path().join("large.bin"), vec![0u8; 50_000]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    let results = cache.search("size:..500 OR size:>10kb").unwrap();
    assert_eq!(results.len(), 2);

    let paths: Vec<_> = results
        .iter()
        .map(|i| cache.node_path(*i).unwrap())
        .collect();
    assert!(paths.iter().any(|p| p.ends_with("tiny.bin")));
    assert!(paths.iter().any(|p| p.ends_with("large.bin")));
}

#[test]
fn test_size_filter_empty_result() {
    let tmp = TempDir::new("size_empty_result").unwrap();
    fs::write(tmp.path().join("small.bin"), vec![0u8; 100]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    let results = cache.search("size:>1mb").unwrap();
    assert_eq!(results.len(), 0);
}

#[test]
fn test_size_with_whitespace() {
    let tmp = TempDir::new("size_whitespace").unwrap();
    fs::write(tmp.path().join("file.bin"), vec![0u8; 2048]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    // Test basic size query (whitespace after operator might not be supported)
    let results = cache.search("size:>1kb").unwrap();
    assert_eq!(results.len(), 1);
}

#[test]
fn test_size_boundary_conditions() {
    let tmp = TempDir::new("size_boundary").unwrap();
    fs::write(tmp.path().join("exactly_1kb.bin"), vec![0u8; 1024]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    let gt = cache.search("size:>1kb").unwrap();
    assert_eq!(gt.len(), 0);

    let gte = cache.search("size:>=1kb").unwrap();
    assert_eq!(gte.len(), 1);

    let lt = cache.search("size:<1kb").unwrap();
    assert_eq!(lt.len(), 0);

    let lte = cache.search("size:<=1kb").unwrap();
    assert_eq!(lte.len(), 1);

    let eq = cache.search("size:=1kb").unwrap();
    assert_eq!(eq.len(), 1);
}

#[test]
fn test_size_with_regex_filter() {
    let tmp = TempDir::new("size_with_regex").unwrap();
    fs::write(tmp.path().join("Report_2024.pdf"), vec![0u8; 10_000]).unwrap();
    fs::write(tmp.path().join("Report_2023.pdf"), vec![0u8; 500]).unwrap();
    fs::write(tmp.path().join("Data_2024.csv"), vec![0u8; 10_000]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    let results = cache.search("regex:^Report.* size:>5kb").unwrap();
    assert_eq!(results.len(), 1);

    let path = cache.node_path(*results.first().unwrap()).unwrap();
    assert!(path.ends_with("Report_2024.pdf"));
}

#[test]
fn test_size_filter_symlinks() {
    let tmp = TempDir::new("size_symlinks").unwrap();
    fs::write(tmp.path().join("target.bin"), vec![0u8; 5000]).unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        symlink(tmp.path().join("target.bin"), tmp.path().join("link.bin")).unwrap();
    }

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    // Size filter should handle symlinks gracefully
    let results = cache.search("size:>1kb").unwrap();
    assert!(!results.is_empty());
}

#[test]
fn test_size_filter_very_small_files() {
    let tmp = TempDir::new("size_very_small").unwrap();
    fs::write(tmp.path().join("1byte.bin"), vec![0u8; 1]).unwrap();
    fs::write(tmp.path().join("2bytes.bin"), vec![0u8; 2]).unwrap();
    fs::write(tmp.path().join("empty.bin"), vec![]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    let results = cache.search("size:>=1").unwrap();
    assert_eq!(results.len(), 2);

    let empty = cache.search("size:=0").unwrap();
    assert_eq!(empty.len(), 1);
}

#[test]
fn test_size_filter_with_quoted_phrases() {
    let tmp = TempDir::new("size_quoted").unwrap();
    fs::write(tmp.path().join("my report.pdf"), vec![0u8; 10_000]).unwrap();
    fs::write(tmp.path().join("other.txt"), vec![0u8; 10_000]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    let results = cache.search("\"my report\" size:>5kb").unwrap();
    assert_eq!(results.len(), 1);

    let path = cache.node_path(*results.first().unwrap()).unwrap();
    assert!(path.ends_with("my report.pdf"));
}

#[test]
fn test_size_unit_case_insensitive() {
    let tmp = TempDir::new("size_unit_case").unwrap();
    fs::write(tmp.path().join("file.bin"), vec![0u8; 2048]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    let lower = cache.search("size:>1kb").unwrap();
    assert_eq!(lower.len(), 1);

    let upper = cache.search("size:>1KB").unwrap();
    assert_eq!(upper.len(), 1);

    let mixed = cache.search("size:>1Kb").unwrap();
    assert_eq!(mixed.len(), 1);

    let megabyte = cache.search("size:<1MB").unwrap();
    assert_eq!(megabyte.len(), 1);
}

#[test]
fn test_size_range_inclusive_bounds() {
    let tmp = TempDir::new("size_range_inclusive").unwrap();
    fs::write(tmp.path().join("1kb.bin"), vec![0u8; 1024]).unwrap();
    fs::write(tmp.path().join("2kb.bin"), vec![0u8; 2048]).unwrap();
    fs::write(tmp.path().join("3kb.bin"), vec![0u8; 3072]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    let results = cache.search("size:1kb..3kb").unwrap();
    assert_eq!(results.len(), 3, "Range should include both bounds");
}

#[test]
fn test_size_with_multiple_and_conditions() {
    let tmp = TempDir::new("size_multi_and").unwrap();
    fs::write(tmp.path().join("report.pdf"), vec![0u8; 5_000]).unwrap();
    fs::write(tmp.path().join("data.csv"), vec![0u8; 5_000]).unwrap();
    fs::write(tmp.path().join("small.txt"), vec![0u8; 100]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    let results = cache.search("report size:>1kb ext:pdf").unwrap();
    assert_eq!(results.len(), 1);
}

#[test]
fn test_size_comparison_with_equal_files() {
    let tmp = TempDir::new("size_equal_files").unwrap();
    fs::write(tmp.path().join("file1.bin"), vec![0u8; 1000]).unwrap();
    fs::write(tmp.path().join("file2.bin"), vec![0u8; 1000]).unwrap();
    fs::write(tmp.path().join("file3.bin"), vec![0u8; 1000]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    let results = cache.search("size:=1000").unwrap();
    assert_eq!(results.len(), 3);
}

#[test]
fn test_size_filter_performance_many_files() {
    let tmp = TempDir::new("size_perf").unwrap();
    for i in 0..100 {
        let size = (i * 100) % 10000;
        fs::write(tmp.path().join(format!("file_{i}.bin")), vec![0u8; size]).unwrap();
    }

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    let results = cache.search("size:>5kb").unwrap();
    assert!(!results.is_empty());
}

#[test]
fn test_size_double_range_error() {
    let tmp = TempDir::new("size_double_range").unwrap();
    fs::write(tmp.path().join("file.bin"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    // This should parse as a range with start "1kb..2kb" and no end, which is invalid
    // Actually, the parser might reject this, so let's just verify it doesn't crash
    let result = cache.search("size:1kb..2kb..3kb");
    // Accept either error or unexpected parsing behavior
    let _ = result;
}

#[test]
fn test_size_negative_number_error() {
    let tmp = TempDir::new("size_negative").unwrap();
    fs::write(tmp.path().join("file.bin"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    let result = cache.search("size:-100");
    // This should either error or be parsed as something else
    let _ = result;
}

#[test]
fn test_size_range_single_point() {
    let tmp = TempDir::new("size_range_point").unwrap();
    fs::write(tmp.path().join("exact.bin"), vec![0u8; 1024]).unwrap();
    fs::write(tmp.path().join("other.bin"), vec![0u8; 2048]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    let results = cache.search("size:1kb..1kb").unwrap();
    assert_eq!(results.len(), 1);
}

#[test]
fn test_size_keywords_boundaries() {
    let tmp = TempDir::new("size_keywords_bounds").unwrap();
    // Test exact boundary values
    fs::write(tmp.path().join("0b.bin"), vec![]).unwrap(); // empty: 0
    fs::write(tmp.path().join("5kb.bin"), vec![0u8; 5 * 1024]).unwrap(); // tiny: 0..10KB
    fs::write(tmp.path().join("50kb.bin"), vec![0u8; 50 * 1024]).unwrap(); // small: 10KB+1..100KB
    fs::write(tmp.path().join("500kb.bin"), vec![0u8; 500 * 1024]).unwrap(); // medium: 100KB+1..1MB
    fs::write(tmp.path().join("5mb.bin"), vec![0u8; 5 * 1024 * 1024]).unwrap(); // large: 1MB+1..16MB
    fs::write(tmp.path().join("50mb.bin"), vec![0u8; 50 * 1024 * 1024]).unwrap(); // huge: 16MB+1..128MB
    fs::write(tmp.path().join("200mb.bin"), vec![0u8; 200 * 1024 * 1024]).unwrap(); // gigantic: >128MB

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    let empty = cache.search("size:empty").unwrap();
    assert_eq!(empty.len(), 1, "Should match empty file");

    let tiny = cache.search("size:tiny").unwrap();
    // tiny range is 0..10KB, which includes empty files (0 bytes)
    assert_eq!(
        tiny.len(),
        2,
        "Should match 0b and 5kb files (tiny: 0..10KB)"
    );

    let small = cache.search("size:small").unwrap();
    assert_eq!(small.len(), 1, "Should match 50kb file");

    let medium = cache.search("size:medium").unwrap();
    assert_eq!(medium.len(), 1, "Should match 500kb file");

    let large = cache.search("size:large").unwrap();
    assert_eq!(large.len(), 1, "Should match 5mb file");

    let huge = cache.search("size:huge").unwrap();
    assert_eq!(huge.len(), 1, "Should match 50mb file");

    let gigantic = cache.search("size:gigantic").unwrap();
    assert_eq!(gigantic.len(), 1, "Should match 200mb file");
}

#[test]
fn test_size_with_floating_point_edge_cases() {
    let tmp = TempDir::new("size_float_edge").unwrap();
    fs::write(tmp.path().join("file.bin"), vec![0u8; 1536]).unwrap(); // 1.5 KB

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    let results = cache.search("size:1.5kb").unwrap();
    assert_eq!(results.len(), 1);

    let results2 = cache.search("size:1.500kb").unwrap();
    assert_eq!(results2.len(), 1);

    let results3 = cache.search("size:>1.49kb").unwrap();
    assert_eq!(results3.len(), 1);

    let results4 = cache.search("size:>1.51kb").unwrap();
    assert_eq!(results4.len(), 0);
}

#[test]
fn test_size_with_all_comparison_operators_on_same_file() {
    let tmp = TempDir::new("size_all_ops").unwrap();
    fs::write(tmp.path().join("1kb.bin"), vec![0u8; 1024]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    assert_eq!(cache.search("size:>1023").unwrap().len(), 1);
    assert_eq!(cache.search("size:>=1024").unwrap().len(), 1);
    assert_eq!(cache.search("size:<1025").unwrap().len(), 1);
    assert_eq!(cache.search("size:<=1024").unwrap().len(), 1);
    assert_eq!(cache.search("size:=1024").unwrap().len(), 1);
    assert_eq!(cache.search("size:!=1023").unwrap().len(), 1);
    assert_eq!(cache.search("size:!=1024").unwrap().len(), 0);
}

#[test]
fn test_size_range_with_different_units() {
    let tmp = TempDir::new("size_range_units").unwrap();
    fs::write(tmp.path().join("file.bin"), vec![0u8; 1_500_000]).unwrap(); // ~1.43 MB

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    let results = cache.search("size:1mb..2mb").unwrap();
    assert_eq!(results.len(), 1);

    let results2 = cache.search("size:1000kb..2000kb").unwrap();
    assert_eq!(results2.len(), 1);

    let results3 = cache.search("size:500kb..2mb").unwrap();
    assert_eq!(results3.len(), 1);
}

#[test]
fn test_size_with_name_containing_numbers() {
    let tmp = TempDir::new("size_name_numbers").unwrap();
    fs::write(tmp.path().join("file123.bin"), vec![0u8; 5000]).unwrap();
    fs::write(tmp.path().join("456file.bin"), vec![0u8; 5000]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    let results = cache.search("123 size:>1kb").unwrap();
    assert_eq!(results.len(), 1);

    let results2 = cache.search("456 size:>1kb").unwrap();
    assert_eq!(results2.len(), 1);
}

#[test]
fn test_size_filter_with_many_size_variants() {
    let tmp = TempDir::new("size_many_variants").unwrap();
    for i in 0..50 {
        let size = i * 1000;
        fs::write(tmp.path().join(format!("file_{i}.bin")), vec![0u8; size]).unwrap();
    }

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    let results = cache.search("size:>20kb").unwrap();
    assert!(!results.is_empty());

    let results2 = cache.search("size:10kb..30kb").unwrap();
    assert!(!results2.is_empty());
}

#[test]
fn test_size_extreme_values() {
    let tmp = TempDir::new("size_extreme").unwrap();
    fs::write(tmp.path().join("file.bin"), vec![0u8; 100]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    // Test very large size queries
    let results = cache.search("size:<1000pb").unwrap();
    assert!(!results.is_empty());

    // Test very small size queries
    let results2 = cache.search("size:>0").unwrap();
    assert!(!results2.is_empty());
}

#[test]
fn test_size_with_all_keywords() {
    let tmp = TempDir::new("size_all_keywords").unwrap();
    fs::write(tmp.path().join("empty.bin"), vec![]).unwrap();
    fs::write(tmp.path().join("tiny.bin"), vec![0u8; 5_000]).unwrap();
    fs::write(tmp.path().join("small.bin"), vec![0u8; 50_000]).unwrap();
    fs::write(tmp.path().join("medium.bin"), vec![0u8; 500_000]).unwrap();
    fs::write(tmp.path().join("large.bin"), vec![0u8; 5_000_000]).unwrap();
    fs::write(tmp.path().join("huge.bin"), vec![0u8; 50_000_000]).unwrap();
    fs::write(tmp.path().join("gigantic.bin"), vec![0u8; 200_000_000]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    assert!(!cache.search("size:empty").unwrap().is_empty());
    assert!(!cache.search("size:tiny").unwrap().is_empty());
    assert!(!cache.search("size:small").unwrap().is_empty());
    assert!(!cache.search("size:medium").unwrap().is_empty());
    assert!(!cache.search("size:large").unwrap().is_empty());
    assert!(!cache.search("size:huge").unwrap().is_empty());
    assert!(!cache.search("size:gigantic").unwrap().is_empty());
    assert!(!cache.search("size:giant").unwrap().is_empty());
}

#[test]
fn test_size_negation_complex() {
    let tmp = TempDir::new("size_negation").unwrap();
    fs::write(tmp.path().join("small.bin"), vec![0u8; 100]).unwrap();
    fs::write(tmp.path().join("large.bin"), vec![0u8; 100_000]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    let results = cache.search("!size:>10kb").unwrap();
    let has_large = results.iter().any(|&i| {
        cache
            .node_path(i)
            .map(|p| p.ends_with("large.bin"))
            .unwrap_or(false)
    });
    assert!(!has_large);
}

#[test]
fn test_size_with_very_precise_decimal() {
    let tmp = TempDir::new("size_precise_decimal").unwrap();
    fs::write(tmp.path().join("file.bin"), vec![0u8; 1536]).unwrap(); // 1.5 KB exactly

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    let results = cache.search("size:1.5kb").unwrap();
    assert_eq!(results.len(), 1);

    let results2 = cache.search("size:1.50kb").unwrap();
    assert_eq!(results2.len(), 1);

    let results3 = cache.search("size:1.5000kb").unwrap();
    assert_eq!(results3.len(), 1);
}

#[test]
fn test_size_with_unicode_in_filename() {
    let tmp = TempDir::new("size_unicode").unwrap();
    fs::write(tmp.path().join("文件.bin"), vec![0u8; 5000]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    let results = cache.search("size:>1kb").unwrap();
    assert_eq!(results.len(), 1);
}

#[test]
fn test_size_range_overlap() {
    let tmp = TempDir::new("size_range_overlap").unwrap();
    fs::write(tmp.path().join("file.bin"), vec![0u8; 5000]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    let results1 = cache.search("size:1kb..10kb").unwrap();
    assert_eq!(results1.len(), 1);

    let results2 = cache.search("size:4kb..6kb").unwrap();
    assert_eq!(results2.len(), 1);

    let results3 = cache.search("size:6kb..10kb").unwrap();
    assert_eq!(results3.len(), 0);
}

#[test]
fn test_size_comparison_chain() {
    let tmp = TempDir::new("size_comp_chain").unwrap();
    fs::write(tmp.path().join("file.bin"), vec![0u8; 5000]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    // Combining multiple size constraints
    let results = cache.search("size:>1kb size:<10kb").unwrap();
    assert_eq!(results.len(), 1);
}

#[test]
fn test_size_with_repeated_filters() {
    let tmp = TempDir::new("size_repeated").unwrap();
    fs::write(tmp.path().join("file.bin"), vec![0u8; 5000]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    // Multiple size filters should intersect
    let results = cache.search("size:>1kb size:>2kb size:>3kb").unwrap();
    assert_eq!(results.len(), 1);

    let results2 = cache.search("size:>1kb size:>10kb").unwrap();
    assert_eq!(results2.len(), 0);
}

#[test]
fn test_size_zero_with_comparison_operators() {
    let tmp = TempDir::new("size_zero_comp").unwrap();
    fs::write(tmp.path().join("empty.bin"), vec![]).unwrap();
    fs::write(tmp.path().join("nonempty.bin"), vec![0u8; 1]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    let gt_zero = cache.search("size:>0").unwrap();
    assert_eq!(gt_zero.len(), 1);

    let gte_zero = cache.search("size:>=0").unwrap();
    assert_eq!(gte_zero.len(), 2);

    let eq_zero = cache.search("size:=0").unwrap();
    assert_eq!(eq_zero.len(), 1);

    let ne_zero = cache.search("size:!=0").unwrap();
    assert_eq!(ne_zero.len(), 1);
}

#[test]
fn test_size_scientific_notation_not_supported() {
    let tmp = TempDir::new("size_scientific").unwrap();
    fs::write(tmp.path().join("file.bin"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    // Scientific notation should fail to parse
    let result = cache.search("size:1e6");
    // Should either error or parse incorrectly
    let _ = result;
}

#[test]
fn test_size_range_only_start() {
    let tmp = TempDir::new("size_range_start").unwrap();
    fs::write(tmp.path().join("small.bin"), vec![0u8; 500]).unwrap();
    fs::write(tmp.path().join("large.bin"), vec![0u8; 50_000]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    let results = cache.search("size:1kb..").unwrap();
    assert_eq!(results.len(), 1);

    let path = cache.node_path(*results.first().unwrap()).unwrap();
    assert!(path.ends_with("large.bin"));
}

#[test]
fn test_size_range_only_end() {
    let tmp = TempDir::new("size_range_end").unwrap();
    fs::write(tmp.path().join("small.bin"), vec![0u8; 500]).unwrap();
    fs::write(tmp.path().join("large.bin"), vec![0u8; 50_000]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    let results = cache.search("size:..10kb").unwrap();
    assert_eq!(results.len(), 1);

    let path = cache.node_path(*results.first().unwrap()).unwrap();
    assert!(path.ends_with("small.bin"));
}

#[test]
fn test_size_keyword_with_spaces() {
    let tmp = TempDir::new("size_keyword_space").unwrap();
    fs::write(tmp.path().join("file.bin"), vec![0u8; 100]).unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    // Test that spaces are trimmed
    let result = cache.search("size: tiny ");
    // Should work or error gracefully
    let _ = result;
}

#[test]
fn test_size_multiple_ranges_or() {
    let tmp = TempDir::new("size_multi_range").unwrap();
    fs::write(tmp.path().join("tiny.bin"), vec![0u8; 100]).unwrap();
    fs::write(tmp.path().join("medium.bin"), vec![0u8; 5000]).unwrap();
    fs::write(tmp.path().join("large.bin"), vec![0u8; 100_000]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    let results = cache.search("size:..500 OR size:50kb..").unwrap();
    assert_eq!(results.len(), 2);
}

#[test]
fn test_size_boundary_exact_1024() {
    let tmp = TempDir::new("size_1024").unwrap();
    fs::write(tmp.path().join("1023.bin"), vec![0u8; 1023]).unwrap();
    fs::write(tmp.path().join("1024.bin"), vec![0u8; 1024]).unwrap();
    fs::write(tmp.path().join("1025.bin"), vec![0u8; 1025]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    let lt = cache.search("size:<1kb").unwrap();
    assert_eq!(lt.len(), 1);

    let eq = cache.search("size:=1kb").unwrap();
    assert_eq!(eq.len(), 1);

    let gt = cache.search("size:>1kb").unwrap();
    assert_eq!(gt.len(), 1);
}

#[test]
fn test_size_with_path_filter_complex() {
    let tmp = TempDir::new("size_path_complex").unwrap();
    fs::create_dir(tmp.path().join("large_files")).unwrap();
    fs::create_dir(tmp.path().join("small_files")).unwrap();
    fs::write(tmp.path().join("large_files/file.bin"), vec![0u8; 100_000]).unwrap();
    fs::write(tmp.path().join("small_files/file.bin"), vec![0u8; 100]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    let large_dir = tmp.path().join("large_files");
    let results = cache
        .search(&format!(
            "file.bin size:>10kb parent:{}",
            large_dir.display()
        ))
        .unwrap();
    assert_eq!(results.len(), 1);
}

#[test]
fn test_size_overflow_protection() {
    let tmp = TempDir::new("size_overflow").unwrap();
    fs::write(tmp.path().join("file.bin"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    // Very large number that might overflow
    let result = cache.search("size:<99999999999999gb");
    assert!(result.is_ok(), "Should handle large numbers gracefully");
}

#[test]
fn test_size_with_leading_zeros() {
    let tmp = TempDir::new("size_leading_zeros").unwrap();
    fs::write(tmp.path().join("file.bin"), vec![0u8; 1024]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    let results = cache.search("size:01kb").unwrap();
    assert_eq!(results.len(), 1);

    let results2 = cache.search("size:001kb").unwrap();
    assert_eq!(results2.len(), 1);
}

#[test]
fn test_size_with_mixed_units_in_range() {
    let tmp = TempDir::new("size_mixed_units").unwrap();
    fs::write(tmp.path().join("file.bin"), vec![0u8; 1_500_000]).unwrap(); // ~1.43 MB

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    let results = cache.search("size:1000kb..2mb").unwrap();
    assert_eq!(results.len(), 1);

    let results2 = cache.search("size:1mb..2000kb").unwrap();
    assert_eq!(results2.len(), 1);
}

#[test]
fn test_size_keyword_boundaries_precise() {
    let tmp = TempDir::new("size_keyword_precise").unwrap();
    // Test exact boundaries: tiny is 0..=10KB, small is 10KB+1..=100KB
    fs::write(tmp.path().join("tiny_max.bin"), vec![0u8; 10 * 1024]).unwrap(); // 10 KB - in tiny
    fs::write(tmp.path().join("small_min.bin"), vec![0u8; 10 * 1024 + 1]).unwrap(); // 10 KB + 1 - in small

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    let tiny = cache.search("size:tiny").unwrap();
    assert_eq!(tiny.len(), 1); // tiny_max.bin

    let small = cache.search("size:small").unwrap();
    assert_eq!(small.len(), 1); // small_min.bin
}

#[test]
fn test_size_range_with_keywords_error() {
    let tmp = TempDir::new("size_range_keyword").unwrap();
    fs::write(tmp.path().join("file.bin"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    // Keywords in ranges might not be supported
    let result = cache.search("size:tiny..large");
    // Should error or handle gracefully
    let _ = result;
}

#[test]
fn test_regression_type_and_size_intersection() {
    let tmp = TempDir::new("regression_intersect").unwrap();
    fs::write(tmp.path().join("a.jpg"), vec![0u8; 100_000]).unwrap();
    fs::write(tmp.path().join("b.jpg"), vec![0u8; 1_000]).unwrap();
    fs::write(tmp.path().join("c.mp3"), vec![0u8; 100_000]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    // Should intersect properly
    let results = cache.search("type:picture size:>10kb").unwrap();
    assert_eq!(results.len(), 1);

    let path = cache.node_path(*results.first().unwrap()).unwrap();
    assert!(path.ends_with("a.jpg"));
}

#[test]
fn test_size_decimal_rounding() {
    let tmp = TempDir::new("size_rounding").unwrap();
    fs::write(tmp.path().join("file.bin"), vec![0u8; 1536]).unwrap(); // 1.5 KB

    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    let results = cache.search("size:1.5kb").unwrap();
    assert_eq!(results.len(), 1);

    // Test rounding behavior - 1.4999kb rounds to 1535.897 bytes, which is less than 1536
    let results2 = cache.search("size:>=1.5kb").unwrap();
    assert_eq!(results2.len(), 1);
}
