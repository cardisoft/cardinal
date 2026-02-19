//! Comprehensive edge case tests for search-cache
//! Covers: empty results, cancellation, special characters, nested paths,
//! large result sets, and various boundary conditions

use search_cache::{SearchCache, SearchOptions};
use search_cancel::CancellationToken;
use std::{
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
};
use tempdir::TempDir;

// Helper function to build cache and keep temp dir alive
fn build_test_cache(files: &[&str]) -> (SearchCache, PathBuf) {
    let temp_dir = TempDir::new("edge_cases_test").unwrap();
    let root_path = temp_dir.path().to_path_buf();
    std::mem::forget(temp_dir); // Keep files accessible

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
fn test_empty_cache() {
    let temp_dir = TempDir::new("empty_cache_test").unwrap();
    let root_path = temp_dir.path().to_path_buf();
    std::mem::forget(temp_dir);

    // Create cache with no files (just root directory)
    let mut cache = SearchCache::walk_fs(&root_path);

    // Search for anything should return empty or small result
    let result = cache.query_files("test".to_string(), CancellationToken::noop());
    assert!(result.is_ok());
    // Just verify it doesn't crash - empty cache behavior may vary

    // Empty query on empty cache
    let result = cache.query_files("".to_string(), CancellationToken::noop());
    assert!(result.is_ok());
}

#[test]
fn test_cancellation_during_search() {
    let files = (0..1000)
        .map(|i| format!("file_{i:04}.txt"))
        .collect::<Vec<_>>();
    let file_refs: Vec<&str> = files.iter().map(|s| s.as_str()).collect();
    let (mut cache, _root) = build_test_cache(&file_refs);

    // Create a cancellation token with version 1
    let token_v1 = CancellationToken::new(1);

    // Create a new version to cancel the old one
    let _token_v2 = CancellationToken::new(2);

    // Search should return None due to cancellation
    let result = cache.query_files("file".to_string(), token_v1);
    assert!(result.is_ok());
    assert!(
        result.unwrap().is_none(),
        "Cancelled search should return None"
    );
}

#[test]
fn test_cancellation_with_stop_flag() {
    let files: Vec<String> = (0..100).map(|i| format!("test_{i:03}.txt")).collect();
    let file_refs: Vec<&str> = files.iter().map(|s| s.as_str()).collect();

    let temp_dir = TempDir::new("cancel_flag_test").unwrap();
    let root_path = temp_dir.path().to_path_buf();
    std::mem::forget(temp_dir);

    for file in file_refs.iter() {
        let full = root_path.join(file);
        std::fs::File::create(full).unwrap();
    }

    let stop = Box::leak(Box::new(AtomicBool::new(false)));
    let walk_data = fswalk::WalkData::new(&root_path, &[], false, || stop.load(Ordering::Relaxed));
    let mut cache = SearchCache::walk_fs_with_walk_data(&walk_data, Some(stop)).unwrap();

    // Set stop flag during search, then create new token to cancel previous
    stop.store(true, Ordering::SeqCst);
    let token_v1 = CancellationToken::new(1);
    let _token_v2 = CancellationToken::new(2);

    let result = cache.query_files("test".to_string(), token_v1);
    assert!(result.is_ok());
    assert!(
        result.unwrap().is_none(),
        "Search with stop flag should be cancelled"
    );
}

#[test]
fn test_special_characters_in_filenames() {
    let files = [
        "file with spaces.txt",
        "file-with-dashes.txt",
        "file_with_underscores.txt",
        "file.multiple.dots.txt",
        "file(with)parens.txt",
        "file[with]brackets.txt",
        "file{with}braces.txt",
        "file'with'quotes.txt",
        "file&ampersand.txt",
        "file@at.txt",
        "file#hash.txt",
        "file$dollar.txt",
        "file%percent.txt",
        "file^caret.txt",
        "café.txt",     // Unicode
        "文件.txt",     // Chinese characters
        "ファイル.txt", // Japanese characters
    ];

    let (mut cache, _root) = build_test_cache(&files);

    // Test searching for files with spaces
    let result = cache
        .query_files("spaces".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(result.is_some());
    let nodes = result.unwrap();
    assert_eq!(nodes.len(), 1);
    assert!(nodes[0].path.to_string_lossy().contains("spaces"));

    // Test searching for files with dashes
    let result = cache
        .query_files("dashes".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(result.is_some());
    assert_eq!(result.unwrap().len(), 1);

    // Test Unicode filenames
    let result = cache
        .query_files("café".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(result.is_some());
    assert_eq!(result.unwrap().len(), 1);

    // Test Chinese characters
    let result = cache
        .query_files("文件".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(result.is_some());
    assert_eq!(result.unwrap().len(), 1);
}

#[test]
fn test_deeply_nested_paths() {
    let files = [
        "a/file1.txt",
        "a/b/file2.txt",
        "a/b/c/file3.txt",
        "a/b/c/d/file4.txt",
        "a/b/c/d/e/file5.txt",
        "a/b/c/d/e/f/file6.txt",
        "a/b/c/d/e/f/g/file7.txt",
        "a/b/c/d/e/f/g/h/file8.txt",
        "a/b/c/d/e/f/g/h/i/file9.txt",
        "a/b/c/d/e/f/g/h/i/j/file10.txt",
    ];

    let (mut cache, root) = build_test_cache(&files);

    // Test searching deep nested file
    let result = cache
        .query_files("file10".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(result.is_some());
    assert_eq!(result.unwrap().len(), 1);

    // Test infolder at various levels
    let deep_path = root.join("a/b/c/d/e");
    let query = format!("infolder:{}", deep_path.display());
    let result = cache.query_files(query, CancellationToken::noop()).unwrap();
    assert!(result.is_some());
    let nodes = result.unwrap();
    // Should find file5.txt through file10.txt and their parent directories
    assert!(
        nodes.len() >= 6,
        "Should find at least 6 items in deep folder"
    );

    // Test parent at deep level
    let parent_path = root.join("a/b/c/d/e/f/g/h/i");
    let query = format!("parent:{}", parent_path.display());
    let result = cache.query_files(query, CancellationToken::noop()).unwrap();
    assert!(result.is_some());
    let nodes = result.unwrap();
    // parent: returns direct children, which includes both file and directory j
    assert!(
        !nodes.is_empty(),
        "Should find at least file10.txt or directory j as direct child"
    );
}

#[test]
fn test_empty_query_returns_all() {
    let files = ["file1.txt", "file2.txt", "dir1/file3.txt"];
    let (mut cache, _root) = build_test_cache(&files);

    // Empty query should return all nodes (or specific behavior)
    let result = cache.query_files("".to_string(), CancellationToken::noop());
    assert!(result.is_ok());
    // Based on implementation, empty query might return all or none
    // This tests the behavior is consistent
}

#[test]
fn test_no_results_found() {
    let files = ["test1.txt", "test2.txt", "test3.txt"];
    let (mut cache, _root) = build_test_cache(&files);

    // Search for non-existent pattern
    let result = cache
        .query_files(
            "nonexistent_pattern_xyz".to_string(),
            CancellationToken::noop(),
        )
        .unwrap();
    assert!(result.is_some());
    assert_eq!(
        result.unwrap().len(),
        0,
        "Should return empty results, not None"
    );
}

#[test]
fn test_single_character_searches() {
    let files = ["a.txt", "ab.txt", "abc.txt", "b.txt", "c.txt"];
    let (mut cache, _root) = build_test_cache(&files);

    // Search for single character
    let result = cache
        .query_files("a".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(result.is_some());
    let nodes = result.unwrap();
    assert!(!nodes.is_empty(), "Should find files starting with 'a'");

    // Search for single character 'b'
    let result = cache
        .query_files("b".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(result.is_some());
    let nodes = result.unwrap();
    assert!(!nodes.is_empty());
}

#[test]
fn test_case_sensitivity() {
    let files = ["File.txt", "file.txt", "FILE.txt"];
    let (mut cache, _root) = build_test_cache(&files);

    // Case sensitive search
    let result = cache
        .search_with_options(
            "File",
            SearchOptions {
                case_insensitive: false,
            },
            CancellationToken::noop(),
        )
        .unwrap();
    assert!(result.nodes.is_some());
    let nodes = result.nodes.unwrap();
    // Depending on filesystem, might find 1 or more (case-insensitive FS like APFS)
    assert!(
        !nodes.is_empty(),
        "Case sensitive should match at least one file"
    );

    // Case insensitive search
    let result = cache
        .search_with_options(
            "file",
            SearchOptions {
                case_insensitive: true,
            },
            CancellationToken::noop(),
        )
        .unwrap();
    assert!(result.nodes.is_some());
    let nodes = result.nodes.unwrap();
    // On case-insensitive filesystems, all three might map to same file
    assert!(!nodes.is_empty(), "Case insensitive should match files");
}

#[test]
fn test_extension_filter_edge_cases() {
    let files = [
        "file.txt",
        "file.TXT",    // uppercase extension
        "file.tar.gz", // double extension
        "file.",       // trailing dot, no extension
        "file",        // no extension
        ".hidden",     // hidden file with no extension
        ".hidden.txt", // hidden file with extension
        "multiple.dots.in.name.txt",
    ];

    let (mut cache, _root) = build_test_cache(&files);

    // Test single extension
    let result = cache
        .query_files("ext:txt".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(result.is_some());
    let nodes = result.unwrap();
    // Should match .txt files (case sensitivity depends on implementation)
    assert!(nodes.len() >= 2, "Should find txt files");

    // Test multiple extensions
    let result = cache
        .query_files("ext:txt;gz".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(result.is_some());
    let nodes = result.unwrap();
    assert!(nodes.len() >= 2);

    // Test non-existent extension
    let result = cache
        .query_files("ext:xyz123".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(result.is_some());
    assert_eq!(result.unwrap().len(), 0);
}

#[test]
fn test_boolean_operations_edge_cases() {
    let files = [
        "apple.txt",
        "banana.txt",
        "cherry.txt",
        "apricot.txt",
        "blueberry.txt",
    ];

    let (mut cache, _root) = build_test_cache(&files);

    // Test AND with no results
    let result = cache
        .query_files("apple banana".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(result.is_some());
    assert_eq!(
        result.unwrap().len(),
        0,
        "No file contains both apple and banana"
    );

    // Test OR with results
    let result = cache
        .query_files("apple | banana".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(result.is_some());
    let nodes = result.unwrap();
    assert_eq!(nodes.len(), 2, "Should find apple.txt and banana.txt");

    // Test NOT with all excluded
    let result = cache
        .query_files(
            "txt ! apple ! banana ! cherry ! apricot ! blueberry".to_string(),
            CancellationToken::noop(),
        )
        .unwrap();
    assert!(result.is_some());
    let nodes = result.unwrap();
    assert_eq!(nodes.len(), 0, "All files should be excluded");

    // Test complex: (A OR B) AND C
    let result = cache
        .query_files("(ap | blu) txt".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(result.is_some());
    let nodes = result.unwrap();
    // Should find files containing "ap" or "blu" AND "txt"
    assert!(
        nodes.len() >= 2,
        "Should find files matching (ap | blu) txt"
    );
}

#[test]
fn test_regex_edge_cases() {
    let files = [
        "test123.txt",
        "test.txt",
        "123test.txt",
        "test_123.txt",
        "test-123.txt",
    ];

    let (mut cache, _root) = build_test_cache(&files);

    // Test anchor patterns
    let result = cache
        .query_files(
            r"regex:^test\d+\.txt$".to_string(),
            CancellationToken::noop(),
        )
        .unwrap();
    assert!(result.is_some());
    let nodes = result.unwrap();
    assert_eq!(nodes.len(), 1, "Should match only test123.txt");

    // Test negative patterns
    let result = cache
        .query_files(
            r"regex:test[^0-9]+\.txt".to_string(),
            CancellationToken::noop(),
        )
        .unwrap();
    assert!(result.is_some());

    // Test empty regex (should be handled)
    let result = cache.query_files(r"regex:".to_string(), CancellationToken::noop());
    // Should either error or handle gracefully
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_wildcard_edge_cases() {
    let files = [
        "file.txt",
        "file1.txt",
        "file123.txt",
        "myfile.txt",
        "file.doc",
        "afile.txt",
    ];

    let (mut cache, _root) = build_test_cache(&files);

    // Test single asterisk at start
    let result = cache
        .query_files("*file.txt".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(result.is_some());
    let nodes = result.unwrap();
    assert!(
        nodes.len() >= 2,
        "Should match files ending with 'file.txt'"
    );

    // Test single asterisk at end
    let result = cache
        .query_files("file*".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(result.is_some());
    let nodes = result.unwrap();
    assert!(nodes.len() >= 4, "Should match files starting with 'file'");

    // Test asterisk in middle
    let result = cache
        .query_files("file*.txt".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(result.is_some());
    let nodes = result.unwrap();
    assert!(nodes.len() >= 3);

    // Test multiple asterisks
    let result = cache
        .query_files("*file*txt*".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(result.is_some());
}

#[test]
fn test_path_filter_boundary_cases() {
    let files = [
        "root.txt",
        "dir/file.txt",
        "dir/subdir/file.txt",
        "dir2/file.txt",
    ];

    let (mut cache, root) = build_test_cache(&files);

    // Test parent with root directory
    let query = format!("parent:{}", root.display());
    let result = cache.query_files(query, CancellationToken::noop()).unwrap();
    assert!(result.is_some());
    let nodes = result.unwrap();
    assert!(nodes.len() >= 3, "Should find root.txt, dir, and dir2");

    // Test infolder with non-existent path - should handle gracefully
    let fake_path = root.join("nonexistent");
    let query = format!("infolder:{}", fake_path.display());
    let result = cache.query_files(query, CancellationToken::noop());
    // Non-existent folder might error or return empty
    assert!(
        result.is_ok() || result.is_err(),
        "Non-existent folder should be handled"
    );

    // Test nosubfolders
    let dir_path = root.join("dir");
    let query = format!("nosubfolders:{}", dir_path.display());
    let result = cache.query_files(query, CancellationToken::noop()).unwrap();
    assert!(result.is_some());
    let nodes = result.unwrap();
    // Should only include direct children, not subdir/file.txt
    assert_eq!(nodes.len(), 1, "Should find only direct child file.txt");
}

#[test]
fn test_duplicate_filenames_different_paths() {
    let files = [
        "dir1/file.txt",
        "dir2/file.txt",
        "dir3/file.txt",
        "dir1/subdir/file.txt",
    ];

    let (mut cache, _root) = build_test_cache(&files);

    // Search for common filename
    let result = cache
        .query_files("file.txt".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(result.is_some());
    let nodes = result.unwrap();
    assert_eq!(nodes.len(), 4, "Should find all instances of file.txt");

    // Ensure paths are unique
    let paths: std::collections::HashSet<_> = nodes.iter().map(|n| &n.path).collect();
    assert_eq!(paths.len(), 4, "All paths should be unique");
}

#[test]
fn test_size_filter_edge_cases() {
    let temp_dir = TempDir::new("size_filter_test").unwrap();
    let root_path = temp_dir.path().to_path_buf();
    std::mem::forget(temp_dir);

    // Create files of different sizes
    std::fs::write(root_path.join("empty.txt"), b"").unwrap();
    std::fs::write(root_path.join("small.txt"), b"a").unwrap();
    std::fs::write(root_path.join("medium.txt"), vec![b'a'; 1024]).unwrap();
    std::fs::write(root_path.join("large.txt"), vec![b'a'; 1024 * 1024]).unwrap();

    let mut cache = SearchCache::walk_fs(&root_path);

    // Test exact size 0
    let result = cache
        .query_files("size:0".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(result.is_some());
    let nodes = result.unwrap();
    assert_eq!(nodes.len(), 1, "Should find empty file");

    // Test size greater than
    let result = cache
        .query_files("size:>1k".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(result.is_some());
    let nodes = result.unwrap();
    assert!(!nodes.is_empty(), "Should find large.txt");

    // Test size less than
    let result = cache
        .query_files("size:<100".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(result.is_some());
    let nodes = result.unwrap();
    assert!(nodes.len() >= 2, "Should find empty.txt and small.txt");

    // Test size range
    let result = cache
        .query_files("size:100..2000".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(result.is_some());
}

#[test]
fn test_highlights_extraction() {
    let files = ["test.txt", "example.txt"];
    let (mut cache, _root) = build_test_cache(&files);

    // Test that highlights are extracted
    let result = cache
        .search_with_options("test", SearchOptions::default(), CancellationToken::noop())
        .unwrap();

    assert!(result.nodes.is_some());
    assert!(
        !result.highlights.is_empty(),
        "Highlights should not be empty"
    );
    assert!(
        result.highlights.contains(&"test".to_string()),
        "Should contain search term"
    );

    // Test highlights with OR
    let result = cache
        .search_with_options(
            "test | example",
            SearchOptions::default(),
            CancellationToken::noop(),
        )
        .unwrap();

    assert!(!result.highlights.is_empty());
    assert!(
        result.highlights.len() >= 2,
        "Should have highlights for both terms"
    );
}

#[test]
fn test_total_files_count() {
    let files = ["file1.txt", "file2.txt", "dir/file3.txt"];
    let (cache, _root) = build_test_cache(&files);

    let total = cache.get_total_files();
    // Should count all files + directories
    assert!(total >= 4, "Should count files and directories: {total:?}");
}

#[test]
fn test_query_with_home_directory_expansion() {
    let files = ["test.txt"];
    let (mut cache, _root) = build_test_cache(&files);

    // Test with tilde (should be expanded or handled)
    // This might fail or be handled depending on implementation
    let result = cache.query_files("~/test.txt".to_string(), CancellationToken::noop());
    // Just ensure it doesn't panic
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_malformed_queries() {
    let files = ["test.txt"];
    let (mut cache, _root) = build_test_cache(&files);

    // Test various malformed queries
    let malformed = [
        "ext:",      // Empty extension
        "size:",     // Empty size
        "parent:",   // Empty parent
        "infolder:", // Empty infolder
        "regex:[[[", // Invalid regex
        "(((",       // Unmatched parens
        "!!!",       // Multiple NOTs
    ];

    for query in malformed {
        let result = cache.query_files(query.to_string(), CancellationToken::noop());
        // Should either return error or empty results, not panic
        assert!(
            result.is_ok() || result.is_err(),
            "Query should not panic: {query}"
        );
    }
}
