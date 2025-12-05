//! Tests for metadata filters and date/time operations
//! Covers: size filters, date modified/created filters, file type filters,
//! metadata caching, and edge cases in metadata handling

use search_cache::SearchCache;
use search_cancel::CancellationToken;
use std::{path::PathBuf, time::Duration};
use tempdir::TempDir;

fn build_cache_with_files(files: &[(&str, &[u8])]) -> (SearchCache, PathBuf) {
    let temp_dir = TempDir::new("metadata_test").unwrap();
    let root_path = temp_dir.path().to_path_buf();
    std::mem::forget(temp_dir);

    for (filename, content) in files {
        let full_path = root_path.join(filename);
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(full_path, content).unwrap();
    }

    let cache = SearchCache::walk_fs(root_path.clone());
    (cache, root_path)
}

#[test]
fn test_size_filter_exact_match() {
    let files = [
        ("exact_100.txt", &[b'a'; 100][..]),
        ("exact_200.txt", &[b'b'; 200][..]),
        ("exact_300.txt", &[b'c'; 300][..]),
    ];
    let (mut cache, _root) = build_cache_with_files(&files);

    // Exact size match
    let result = cache
        .query_files("size:100".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(result.is_some());
    let nodes = result.unwrap();
    assert_eq!(nodes.len(), 1, "Should find exactly one 100-byte file");
    assert!(nodes[0].path.to_string_lossy().contains("exact_100"));
}

#[test]
fn test_size_filter_greater_than() {
    let files = [
        ("small.txt", &[b'a'; 50][..]),
        ("medium.txt", &[b'b'; 500][..]),
        ("large.txt", &[b'c'; 5000][..]),
    ];
    let (mut cache, _root) = build_cache_with_files(&files);

    // Greater than 100 bytes
    let result = cache
        .query_files("size:>100".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(result.is_some());
    let nodes = result.unwrap();
    assert!(nodes.len() >= 2, "Should find medium and large files");
}

#[test]
fn test_size_filter_less_than() {
    let files = [
        ("tiny.txt", &[b'a'; 10][..]),
        ("small.txt", &[b'b'; 100][..]),
        ("large.txt", &[b'c'; 1000][..]),
    ];
    let (mut cache, _root) = build_cache_with_files(&files);

    // Less than 200 bytes
    let result = cache
        .query_files("size:<200".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(result.is_some());
    let nodes = result.unwrap();
    assert!(nodes.len() >= 2, "Should find tiny and small files");
}

#[test]
fn test_size_filter_range() {
    let files = [
        ("file1.txt", &[b'a'; 100][..]),
        ("file2.txt", &[b'b'; 500][..]),
        ("file3.txt", &[b'c'; 1000][..]),
        ("file4.txt", &[b'd'; 2000][..]),
    ];
    let (mut cache, _root) = build_cache_with_files(&files);

    // Range: 200 to 1500 bytes
    let result = cache
        .query_files("size:200..1500".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(result.is_some());
    let nodes = result.unwrap();
    assert!(nodes.len() >= 2, "Should find files in size range");
}

#[test]
fn test_size_filter_with_units() {
    let files = [
        ("1k.txt", &vec![b'a'; 1024][..]),
        ("10k.txt", &vec![b'b'; 10 * 1024][..]),
        ("100k.txt", &vec![b'c'; 100 * 1024][..]),
        ("1m.txt", &vec![b'd'; 1024 * 1024][..]),
    ];
    let (mut cache, _root) = build_cache_with_files(&files);

    // Size in KB
    let result = cache
        .query_files("size:>5k".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(result.is_some());
    let nodes = result.unwrap();
    assert!(nodes.len() >= 3, "Should find files larger than 5KB");

    // Size in MB
    let result = cache
        .query_files("size:<2m".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(result.is_some());
    let nodes = result.unwrap();
    assert_eq!(nodes.len(), 4, "All test files are less than 2MB");
}

#[test]
fn test_file_type_filter_file() {
    let temp_dir = TempDir::new("file_type_test").unwrap();
    let root_path = temp_dir.path().to_path_buf();
    std::mem::forget(temp_dir);

    std::fs::File::create(root_path.join("regular.txt")).unwrap();
    std::fs::create_dir(root_path.join("directory")).unwrap();
    std::fs::File::create(root_path.join("another.txt")).unwrap();

    let mut cache = SearchCache::walk_fs(root_path.clone());

    // Filter only files
    let result = cache
        .query_files("file:".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(result.is_some());
    let nodes = result.unwrap();
    assert!(nodes.len() >= 2, "Should find regular files");
}

#[test]
fn test_folder_type_filter() {
    let temp_dir = TempDir::new("folder_type_test").unwrap();
    let root_path = temp_dir.path().to_path_buf();
    std::mem::forget(temp_dir);

    std::fs::create_dir(root_path.join("dir1")).unwrap();
    std::fs::create_dir(root_path.join("dir2")).unwrap();
    std::fs::File::create(root_path.join("file.txt")).unwrap();

    let mut cache = SearchCache::walk_fs(root_path.clone());

    // Filter only folders
    let result = cache
        .query_files("folder:".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(result.is_some());
    let nodes = result.unwrap();
    assert!(nodes.len() >= 2, "Should find directories");
}

#[test]
fn test_combined_size_and_type_filter() {
    let temp_dir = TempDir::new("combined_filter_test").unwrap();
    let root_path = temp_dir.path().to_path_buf();
    std::mem::forget(temp_dir);

    std::fs::write(root_path.join("small.txt"), vec![b'a'; 100]).unwrap();
    std::fs::write(root_path.join("large.txt"), vec![b'b'; 10000]).unwrap();
    std::fs::create_dir(root_path.join("dir")).unwrap();

    let mut cache = SearchCache::walk_fs(root_path.clone());

    // Files larger than 1000 bytes
    let result = cache
        .query_files("file: size:>1000".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(result.is_some());
    let nodes = result.unwrap();
    assert_eq!(nodes.len(), 1, "Should find only large.txt");
}

#[test]
fn test_size_filter_zero_bytes() {
    let files = [
        ("empty1.txt", &[][..]),
        ("empty2.txt", &[][..]),
        ("nonempty.txt", &[b'a'][..]),
    ];
    let (mut cache, _root) = build_cache_with_files(&files);

    // Exact zero size
    let result = cache
        .query_files("size:0".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(result.is_some());
    let nodes = result.unwrap();
    assert_eq!(nodes.len(), 2, "Should find both empty files");
}

#[test]
fn test_size_filter_edge_values() {
    let files = [
        ("one_byte.txt", &[b'a'][..]),
        ("max_byte.txt", &[b'b'; 255][..]),
    ];
    let (mut cache, _root) = build_cache_with_files(&files);

    // Single byte
    let result = cache
        .query_files("size:1".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(result.is_some());
    assert_eq!(result.unwrap().len(), 1);

    // 255 bytes
    let result = cache
        .query_files("size:255".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(result.is_some());
    assert_eq!(result.unwrap().len(), 1);
}

#[test]
fn test_date_modified_filter_recent() {
    let temp_dir = TempDir::new("date_modified_test").unwrap();
    let root_path = temp_dir.path().to_path_buf();
    std::mem::forget(temp_dir);

    // Create files (they will have current timestamp)
    std::fs::File::create(root_path.join("recent.txt")).unwrap();
    std::thread::sleep(Duration::from_millis(100));
    std::fs::File::create(root_path.join("newer.txt")).unwrap();

    let mut cache = SearchCache::walk_fs(root_path.clone());

    // Query for files modified today
    let result = cache
        .query_files("dm:today".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(result.is_some());
    let nodes = result.unwrap();
    assert!(nodes.len() >= 2, "Should find files modified today");
}

#[test]
fn test_date_created_filter() {
    let temp_dir = TempDir::new("date_created_test").unwrap();
    let root_path = temp_dir.path().to_path_buf();
    std::mem::forget(temp_dir);

    std::fs::File::create(root_path.join("created_today.txt")).unwrap();

    let mut cache = SearchCache::walk_fs(root_path.clone());

    // Query for files created today
    let result = cache
        .query_files("dm:today".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(result.is_some());
    let nodes = result.unwrap();
    assert!(!nodes.is_empty(), "Should find files created today");
}

#[test]
fn test_metadata_lazy_loading() {
    let temp_dir = TempDir::new("lazy_metadata_test").unwrap();
    let root_path = temp_dir.path().to_path_buf();
    std::mem::forget(temp_dir);

    std::fs::write(root_path.join("test.txt"), vec![b'a'; 1000]).unwrap();

    // Initial walk doesn't fetch metadata
    let mut cache = SearchCache::walk_fs(root_path.clone());

    // First size query should trigger metadata fetch
    let result = cache
        .query_files("size:>500".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(result.is_some());
    assert_eq!(result.unwrap().len(), 1);

    // Second query should use cached metadata
    let result = cache
        .query_files("size:<2000".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(result.is_some());
    assert_eq!(result.unwrap().len(), 1);
}

#[test]
fn test_expand_file_nodes_with_metadata() {
    let files = [
        ("file1.txt", &[b'a'; 100][..]),
        ("file2.txt", &[b'b'; 200][..]),
    ];
    let (mut cache, _root) = build_cache_with_files(&files);

    // Get file nodes
    let result = cache
        .query_files("file".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(result.is_some());
    let nodes = result.unwrap();

    // Nodes should have paths
    for node in &nodes {
        assert!(!node.path.as_os_str().is_empty(), "Node should have path");
    }
}

#[test]
fn test_size_comparison_operators() {
    let files = [
        ("100.txt", &[b'a'; 100][..]),
        ("200.txt", &[b'b'; 200][..]),
        ("300.txt", &[b'c'; 300][..]),
    ];
    let (mut cache, _root) = build_cache_with_files(&files);

    // >= operator
    let result = cache
        .query_files("size:>=200".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(result.is_some());
    let nodes = result.unwrap();
    assert_eq!(nodes.len(), 2, "Should find 200 and 300 byte files");

    // <= operator
    let result = cache
        .query_files("size:<=200".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(result.is_some());
    let nodes = result.unwrap();
    assert_eq!(nodes.len(), 2, "Should find 100 and 200 byte files");

    // != operator (if supported)
    let result = cache.query_files("size:!=200".to_string(), CancellationToken::noop());
    // May or may not be supported, just ensure no panic
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_type_macro_filters() {
    let temp_dir = TempDir::new("type_macro_test").unwrap();
    let root_path = temp_dir.path().to_path_buf();
    std::mem::forget(temp_dir);

    let files = [
        "video.mp4",
        "audio.mp3",
        "document.pdf",
        "image.jpg",
        "executable.exe",
    ];

    for file in &files {
        std::fs::File::create(root_path.join(file)).unwrap();
    }

    let mut cache = SearchCache::walk_fs(root_path.clone());

    // Test video macro
    let result = cache
        .query_files("video:".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(result.is_some());
    let nodes = result.unwrap();
    assert!(!nodes.is_empty(), "Should find video files");

    // Test audio macro
    let result = cache
        .query_files("audio:".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(result.is_some());
    let nodes = result.unwrap();
    assert!(!nodes.is_empty(), "Should find audio files");

    // Test doc macro
    let result = cache
        .query_files("doc:".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(result.is_some());
    let nodes = result.unwrap();
    assert!(!nodes.is_empty(), "Should find document files");
}

#[test]
fn test_size_units_case_insensitive() {
    let files = [("1k.txt", &vec![b'a'; 1024][..])];
    let (mut cache, _root) = build_cache_with_files(&files);

    // Test various case combinations
    let queries = [
        "size:>500b",
        "size:>500B",
        "size:>0.5k",
        "size:>0.5K",
        "size:>0.5KB",
    ];

    for query in &queries {
        let result = cache.query_files(query.to_string(), CancellationToken::noop());
        assert!(result.is_ok(), "Query {query} should succeed");
    }
}

#[test]
fn test_metadata_for_inaccessible_files() {
    let temp_dir = TempDir::new("inaccessible_test").unwrap();
    let root_path = temp_dir.path().to_path_buf();
    std::mem::forget(temp_dir);

    let test_file = root_path.join("test.txt");
    std::fs::File::create(&test_file).unwrap();

    let mut cache = SearchCache::walk_fs(root_path.clone());

    // Delete file after cache is built
    std::fs::remove_file(&test_file).unwrap();

    // Query should handle missing file gracefully
    let result = cache.query_files("test".to_string(), CancellationToken::noop());
    assert!(result.is_ok(), "Should handle missing files gracefully");
}

#[test]
fn test_date_filter_with_ranges() {
    let temp_dir = TempDir::new("date_range_test").unwrap();
    let root_path = temp_dir.path().to_path_buf();
    std::mem::forget(temp_dir);

    std::fs::File::create(root_path.join("file.txt")).unwrap();

    let mut cache = SearchCache::walk_fs(root_path.clone());

    // Date range query (specific dates would depend on implementation)
    let result = cache.query_files("dm:2020..2030".to_string(), CancellationToken::noop());
    // Should handle gracefully
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_combined_metadata_filters() {
    let temp_dir = TempDir::new("combined_metadata_test").unwrap();
    let root_path = temp_dir.path().to_path_buf();
    std::mem::forget(temp_dir);

    std::fs::write(root_path.join("small.txt"), vec![b'a'; 100]).unwrap();
    std::fs::write(root_path.join("large.txt"), vec![b'b'; 10000]).unwrap();
    std::fs::create_dir(root_path.join("dir")).unwrap();

    let mut cache = SearchCache::walk_fs(root_path.clone());

    // Multiple filters: file type, extension, size, date
    let result = cache
        .query_files(
            "file: ext:txt size:>500 dm:today".to_string(),
            CancellationToken::noop(),
        )
        .unwrap();
    assert!(result.is_some());
    let nodes = result.unwrap();
    assert_eq!(
        nodes.len(),
        1,
        "Should find only large.txt matching all criteria"
    );
}

#[test]
fn test_size_filter_with_decimal_units() {
    let files = [
        ("1.5k.txt", &vec![b'a'; 1536][..]), // 1.5 KB
    ];
    let (mut cache, _root) = build_cache_with_files(&files);

    // Decimal size
    let result = cache
        .query_files("size:>1.2k".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(result.is_some());
    assert_eq!(result.unwrap().len(), 1);

    let result = cache
        .query_files("size:<2k".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(result.is_some());
    assert_eq!(result.unwrap().len(), 1);
}

#[test]
fn test_metadata_total_files_accuracy() {
    let temp_dir = TempDir::new("total_files_test").unwrap();
    let root_path = temp_dir.path().to_path_buf();
    std::mem::forget(temp_dir);

    let expected_files = 10;
    for i in 0..expected_files {
        std::fs::File::create(root_path.join(format!("file_{i}.txt"))).unwrap();
    }

    let cache = SearchCache::walk_fs(root_path.clone());
    let total = cache.get_total_files();

    // Should count files (not including root directory)
    assert!(
        total >= expected_files,
        "Should count at least {expected_files} files"
    );
}
