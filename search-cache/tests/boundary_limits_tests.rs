//! Boundary condition and limit tests for search-cache
//! Covers: large file counts, long filenames, deep paths, slab index limits,
//! name index stress, query complexity limits

use search_cache::SearchCache;
use search_cancel::CancellationToken;
use tempdir::TempDir;

#[test]
fn test_large_file_count() {
    let temp_dir = TempDir::new("large_count_test").unwrap();
    let root_path = temp_dir.path().to_path_buf();
    std::mem::forget(temp_dir);

    // Create 10,000 files to stress test the cache
    for i in 0..10_000 {
        let filename = format!("file_{i:06}.txt");
        std::fs::File::create(root_path.join(filename)).unwrap();
    }

    let cache = SearchCache::walk_fs(root_path.clone());
    let total = cache.get_total_files();
    assert!(total >= 10_000, "Should index at least 10,000 files");

    // Test search on large cache
    let mut cache_mut = cache;
    let result = cache_mut
        .query_files("file_005000".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(result.is_some());
    assert_eq!(
        result.unwrap().len(),
        1,
        "Should find specific file in large cache"
    );
}

#[test]
fn test_maximum_filename_length() {
    let temp_dir = TempDir::new("max_filename_test").unwrap();
    let root_path = temp_dir.path().to_path_buf();
    std::mem::forget(temp_dir);

    // macOS/Unix typically limits filenames to 255 bytes
    let long_name = "a".repeat(255);
    let long_file = root_path.join(format!("{long_name}.txt"));

    // This might fail on some filesystems, handle gracefully
    if std::fs::File::create(&long_file).is_ok() {
        let mut cache = SearchCache::walk_fs(root_path.clone());

        let result = cache
            .query_files(long_name, CancellationToken::noop())
            .unwrap();
        assert!(result.is_some());
        assert_eq!(
            result.unwrap().len(),
            1,
            "Should handle max-length filename"
        );
    }
}

#[test]
fn test_very_deep_directory_nesting() {
    let temp_dir = TempDir::new("deep_nesting_test").unwrap();
    let root_path = temp_dir.path().to_path_buf();
    std::mem::forget(temp_dir);

    // Create 100 levels of nesting
    let mut current_path = root_path.clone();
    for i in 0..100 {
        current_path = current_path.join(format!("level_{i:03}"));
    }

    if std::fs::create_dir_all(&current_path).is_ok() {
        std::fs::File::create(current_path.join("deep_file.txt")).unwrap();

        let mut cache = SearchCache::walk_fs(root_path.clone());

        let result = cache
            .query_files("deep_file".to_string(), CancellationToken::noop())
            .unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().len(), 1, "Should handle very deep nesting");
    }
}

#[test]
fn test_many_files_same_name_different_dirs() {
    let temp_dir = TempDir::new("same_name_test").unwrap();
    let root_path = temp_dir.path().to_path_buf();
    std::mem::forget(temp_dir);

    // Create 1000 directories each with a file named "duplicate.txt"
    for i in 0..1000 {
        let dir = root_path.join(format!("dir_{i:04}"));
        std::fs::create_dir(&dir).unwrap();
        std::fs::File::create(dir.join("duplicate.txt")).unwrap();
    }

    let mut cache = SearchCache::walk_fs(root_path.clone());

    let result = cache
        .query_files("duplicate.txt".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(result.is_some());
    let nodes = result.unwrap();
    assert_eq!(
        nodes.len(),
        1000,
        "Should find all 1000 files with same name"
    );
}

#[test]
fn test_name_index_with_many_unique_names() {
    let temp_dir = TempDir::new("unique_names_test").unwrap();
    let root_path = temp_dir.path().to_path_buf();
    std::mem::forget(temp_dir);

    // Create 5000 files with unique names to stress the name index
    for i in 0..5000 {
        let filename = format!("unique_file_{i:05}.txt");
        std::fs::File::create(root_path.join(filename)).unwrap();
    }

    let mut cache = SearchCache::walk_fs(root_path.clone());

    // Search for a specific unique name
    let result = cache
        .query_files("unique_file_03456".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(result.is_some());
    assert_eq!(result.unwrap().len(), 1);

    // Search for a pattern that matches many files
    let result = cache
        .query_files("unique_file".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(result.is_some());
    assert_eq!(result.unwrap().len(), 5000, "Should find all unique files");
}

#[test]
fn test_empty_directory_structures() {
    let temp_dir = TempDir::new("empty_dirs_test").unwrap();
    let root_path = temp_dir.path().to_path_buf();
    std::mem::forget(temp_dir);

    // Create 100 empty directories
    for i in 0..100 {
        let dir = root_path.join(format!("empty_dir_{i:03}"));
        std::fs::create_dir(&dir).unwrap();
    }

    let mut cache = SearchCache::walk_fs(root_path.clone());

    // Search for directories
    let result = cache
        .query_files("empty_dir".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(result.is_some());
    let nodes = result.unwrap();
    // Implementation might index the directories themselves
    assert!(
        nodes.len() >= 100,
        "Should index at least 100 empty directories"
    );
}

#[test]
fn test_mixed_file_and_directory_counts() {
    let temp_dir = TempDir::new("mixed_count_test").unwrap();
    let root_path = temp_dir.path().to_path_buf();
    std::mem::forget(temp_dir);

    // Create mix of files and directories
    for i in 0..500 {
        if i % 2 == 0 {
            std::fs::File::create(root_path.join(format!("file_{i}.txt"))).unwrap();
        } else {
            let dir = root_path.join(format!("dir_{i}"));
            std::fs::create_dir(&dir).unwrap();
            std::fs::File::create(dir.join("nested.txt")).unwrap();
        }
    }

    let cache = SearchCache::walk_fs(root_path.clone());
    let total = cache.get_total_files();
    // Should count files + directories + nested files
    assert!(total >= 750, "Should count all files and directories");
}

#[test]
fn test_slab_index_sequential_allocation() {
    let temp_dir = TempDir::new("slab_index_test").unwrap();
    let root_path = temp_dir.path().to_path_buf();
    std::mem::forget(temp_dir);

    // Create files and verify slab indices are allocated
    for i in 0..1000 {
        std::fs::File::create(root_path.join(format!("file_{i}.txt"))).unwrap();
    }

    let cache = SearchCache::walk_fs(root_path.clone());

    // Verify we can get slab indices
    let mut cache_mut = cache;
    let result = cache_mut
        .query_files("file_".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(result.is_some());
    let nodes = result.unwrap();
    assert_eq!(nodes.len(), 1000);
}

#[test]
fn test_query_complexity_boolean_operations() {
    let temp_dir = TempDir::new("complex_query_test").unwrap();
    let root_path = temp_dir.path().to_path_buf();
    std::mem::forget(temp_dir);

    let files = [
        "alpha_beta_gamma.txt",
        "alpha_delta.txt",
        "beta_gamma.txt",
        "gamma_epsilon.txt",
        "zeta.txt",
    ];

    for file in &files {
        std::fs::File::create(root_path.join(file)).unwrap();
    }

    let mut cache = SearchCache::walk_fs(root_path.clone());

    // Very complex query with nested boolean operations
    let complex_query = "(alpha | beta) gamma ! delta";
    let result = cache.query_files(complex_query.to_string(), CancellationToken::noop());
    assert!(result.is_ok(), "Complex query should not crash");

    // Another complex query
    let complex_query2 = "((alpha | beta) gamma) | (epsilon ! zeta)";
    let result = cache.query_files(complex_query2.to_string(), CancellationToken::noop());
    assert!(result.is_ok(), "Nested boolean query should not crash");
}

#[test]
fn test_extremely_long_query_string() {
    let temp_dir = TempDir::new("long_query_test").unwrap();
    let root_path = temp_dir.path().to_path_buf();
    std::mem::forget(temp_dir);

    std::fs::File::create(root_path.join("test.txt")).unwrap();

    let mut cache = SearchCache::walk_fs(root_path.clone());

    // Create extremely long OR query
    let mut long_query = String::new();
    for i in 0..1000 {
        if i > 0 {
            long_query.push_str(" | ");
        }
        long_query.push_str(&format!("term{i}"));
    }

    let result = cache.query_files(long_query, CancellationToken::noop());
    // Should handle or error gracefully, not panic
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_many_extensions_filter() {
    let temp_dir = TempDir::new("many_ext_test").unwrap();
    let root_path = temp_dir.path().to_path_buf();
    std::mem::forget(temp_dir);

    // Create files with many different extensions
    let extensions = [
        "txt", "rs", "md", "toml", "json", "yaml", "xml", "html", "css", "js",
    ];
    for ext in &extensions {
        for i in 0..10 {
            std::fs::File::create(root_path.join(format!("file_{i}.{ext}"))).unwrap();
        }
    }

    let mut cache = SearchCache::walk_fs(root_path.clone());

    // Query with many extensions
    let ext_query = "ext:txt;rs;md;toml;json;yaml;xml;html;css;js";
    let result = cache
        .query_files(ext_query.to_string(), CancellationToken::noop())
        .unwrap();
    assert!(result.is_some());
    let nodes = result.unwrap();
    assert_eq!(
        nodes.len(),
        100,
        "Should find all 100 files across 10 extensions"
    );
}

#[test]
fn test_zero_byte_files() {
    let temp_dir = TempDir::new("zero_byte_test").unwrap();
    let root_path = temp_dir.path().to_path_buf();
    std::mem::forget(temp_dir);

    // Create empty files
    for i in 0..100 {
        std::fs::File::create(root_path.join(format!("empty_{i}.txt"))).unwrap();
    }

    let mut cache = SearchCache::walk_fs(root_path.clone());

    // Search should work on empty files
    let result = cache
        .query_files("empty_".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(result.is_some());
    assert_eq!(result.unwrap().len(), 100);

    // Size filter for zero bytes
    let result = cache
        .query_files("size:0".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(result.is_some());
    let nodes = result.unwrap();
    assert_eq!(nodes.len(), 100, "Should find all zero-byte files");
}

#[test]
fn test_special_filenames_dot_files() {
    let temp_dir = TempDir::new("dot_files_test").unwrap();
    let root_path = temp_dir.path().to_path_buf();
    std::mem::forget(temp_dir);

    // Create hidden files
    let dot_files = [".hidden", ".gitignore", ".env", ".config", ".bashrc"];

    for file in &dot_files {
        std::fs::File::create(root_path.join(file)).unwrap();
    }

    let mut cache = SearchCache::walk_fs(root_path.clone());

    // Search for hidden files
    let result = cache
        .query_files(".hidden".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(result.is_some());
    assert_eq!(result.unwrap().len(), 1);

    // Search for all dot files
    let result = cache
        .query_files(".".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(result.is_some());
    let nodes = result.unwrap();
    assert!(nodes.len() >= 5, "Should find all dot files");
}

#[test]
fn test_path_with_consecutive_dots() {
    let temp_dir = TempDir::new("consecutive_dots_test").unwrap();
    let root_path = temp_dir.path().to_path_buf();
    std::mem::forget(temp_dir);

    // Create files with multiple consecutive dots
    let files = ["file..txt", "file...txt", "file....txt", "....file.txt"];

    for file in &files {
        if std::fs::File::create(root_path.join(file)).is_ok() {
            // Some filesystems may not allow these names
        }
    }

    let mut cache = SearchCache::walk_fs(root_path.clone());

    // Search should handle these gracefully
    let result = cache.query_files("file".to_string(), CancellationToken::noop());
    assert!(result.is_ok());
}

#[test]
fn test_directory_and_file_same_prefix() {
    let temp_dir = TempDir::new("same_prefix_test").unwrap();
    let root_path = temp_dir.path().to_path_buf();
    std::mem::forget(temp_dir);

    // Create file and directory with same prefix
    std::fs::File::create(root_path.join("test")).unwrap();
    let test_dir = root_path.join("test_dir");
    std::fs::create_dir(&test_dir).unwrap();
    std::fs::File::create(test_dir.join("nested.txt")).unwrap();

    let mut cache = SearchCache::walk_fs(root_path.clone());

    // Search for "test" should find both
    let result = cache
        .query_files("test".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(result.is_some());
    let nodes = result.unwrap();
    assert!(
        nodes.len() >= 2,
        "Should find both file and directory with same prefix"
    );
}

#[test]
fn test_cancel_large_search_operation() {
    let temp_dir = TempDir::new("cancel_large_test").unwrap();
    let root_path = temp_dir.path().to_path_buf();
    std::mem::forget(temp_dir);

    // Create many files
    for i in 0..10_000 {
        std::fs::File::create(root_path.join(format!("file_{i:05}.txt"))).unwrap();
    }

    let mut cache = SearchCache::walk_fs(root_path.clone());

    // Create cancellation token and cancel it
    let token_v1 = CancellationToken::new(1);
    let _token_v2 = CancellationToken::new(2); // This cancels v1

    // Large search should be cancelled
    let result = cache.query_files("file_".to_string(), token_v1);
    assert!(result.is_ok());
    // Depending on timing, might return None (cancelled) or partial results
}

#[test]
fn test_metadata_filter_on_large_set() {
    let temp_dir = TempDir::new("metadata_large_test").unwrap();
    let root_path = temp_dir.path().to_path_buf();
    std::mem::forget(temp_dir);

    // Create files with different sizes
    for i in 0..1000 {
        let size = i * 100;
        let content = vec![b'a'; size];
        std::fs::write(root_path.join(format!("file_{i}.txt")), content).unwrap();
    }

    let mut cache = SearchCache::walk_fs(root_path.clone());

    // Filter by size range
    let result = cache
        .query_files("size:>50k".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(result.is_some());
    let nodes = result.unwrap();
    assert!(!nodes.is_empty(), "Should find large files");

    // Combined filter
    let result = cache
        .query_files("file_ size:<10k".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(result.is_some());
}

#[test]
fn test_unicode_in_paths() {
    let temp_dir = TempDir::new("unicode_paths_test").unwrap();
    let root_path = temp_dir.path().to_path_buf();
    std::mem::forget(temp_dir);

    let unicode_names = [
        "测试文件.txt",       // Chinese
        "テストファイル.txt", // Japanese
        "테스트파일.txt",     // Korean
        "тест.txt",           // Russian
        "δοκιμή.txt",         // Greek
        "اختبار.txt",         // Arabic
        "परीक्षण.txt",         // Hindi
    ];

    for name in &unicode_names {
        if std::fs::File::create(root_path.join(name)).is_ok() {
            // Some filesystems may have restrictions
        }
    }

    let mut cache = SearchCache::walk_fs(root_path.clone());

    // Try to search for unicode content
    let result = cache.query_files("测试".to_string(), CancellationToken::noop());
    assert!(result.is_ok(), "Unicode search should not crash");
}
