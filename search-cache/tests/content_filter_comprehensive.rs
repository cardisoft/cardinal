use search_cache::{SearchCache, SearchOptions, SlabIndex};
use search_cancel::CancellationToken;
use std::fs;
use tempdir::TempDir;

// Public constant from the implementation
const CONTENT_BUFFER_BYTES: usize = 64 * 1024;

fn guard_indices(result: Result<search_cache::SearchOutcome, anyhow::Error>) -> Vec<SlabIndex> {
    result
        .expect("search should succeed")
        .nodes
        .expect("noop cancellation token should not cancel")
}

/// Test empty needle rejection
#[test]
fn content_filter_rejects_empty_needle() {
    let temp_dir = TempDir::new("content_empty_needle").unwrap();
    let dir = temp_dir.path();
    fs::write(dir.join("file.txt"), b"content").unwrap();

    let mut cache = SearchCache::walk_fs(dir);
    let result = cache.search_with_options(
        r#"content:"""#,
        SearchOptions {
            case_insensitive: false,
        },
        CancellationToken::noop(),
    );
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("content: requires a value")
    );
}

/// Test single-byte needle - exact match (case sensitive)
#[test]
fn content_filter_single_byte_exact_match() {
    let temp_dir = TempDir::new("content_single_byte_exact").unwrap();
    let dir = temp_dir.path();
    fs::write(dir.join("lower.txt"), b"abcdefg").unwrap();
    fs::write(dir.join("upper.txt"), b"ABCDEFG").unwrap();

    let mut cache = SearchCache::walk_fs(dir);

    // Case sensitive: only lowercase 'a' should match
    let indices = guard_indices(cache.search_with_options(
        "content:a",
        SearchOptions {
            case_insensitive: false,
        },
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 1);
    let nodes = cache.expand_file_nodes(&indices);
    assert!(nodes[0].path.ends_with("lower.txt"));

    // Case sensitive: only uppercase 'A' should match
    let indices = guard_indices(cache.search_with_options(
        "content:A",
        SearchOptions {
            case_insensitive: false,
        },
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 1);
    let nodes = cache.expand_file_nodes(&indices);
    assert!(nodes[0].path.ends_with("upper.txt"));
}

/// Test single-byte needle - case insensitive
#[test]
fn content_filter_single_byte_case_insensitive() {
    let temp_dir = TempDir::new("content_single_byte_insensitive").unwrap();
    let dir = temp_dir.path();
    fs::write(dir.join("lower.txt"), b"abcdefg").unwrap();
    fs::write(dir.join("upper.txt"), b"ABCDEFG").unwrap();
    fs::write(dir.join("none.txt"), b"xyz123").unwrap();

    let mut cache = SearchCache::walk_fs(dir);

    // Case insensitive: both 'a' and 'A' should match
    let indices = guard_indices(cache.search_with_options(
        "content:a",
        SearchOptions {
            case_insensitive: true,
        },
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 2);
    let nodes = cache.expand_file_nodes(&indices);
    assert!(nodes.iter().any(|n| n.path.ends_with("lower.txt")));
    assert!(nodes.iter().any(|n| n.path.ends_with("upper.txt")));

    let indices = guard_indices(cache.search_with_options(
        "content:A",
        SearchOptions {
            case_insensitive: true,
        },
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 2);
}

/// Test multi-byte needle at exact buffer boundary
#[test]
fn content_filter_needle_spans_exact_buffer_boundary() {
    let temp_dir = TempDir::new("content_exact_boundary").unwrap();
    let dir = temp_dir.path();

    // Needle starts at last byte of buffer and continues
    let mut payload = vec![b'x'; CONTENT_BUFFER_BYTES - 1];
    payload.extend_from_slice(b"BOUNDARY");
    payload.extend(vec![b'y'; 100]);
    fs::write(dir.join("exact.bin"), &payload).unwrap();

    let mut cache = SearchCache::walk_fs(dir);
    let indices = guard_indices(cache.search_with_options(
        "content:BOUNDARY",
        SearchOptions {
            case_insensitive: false,
        },
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 1);
}

/// Test needle split across multiple buffer reads (3+ chunks)
#[test]
fn content_filter_needle_spans_three_chunks() {
    let temp_dir = TempDir::new("content_three_chunks").unwrap();
    let dir = temp_dir.path();

    let needle = "LONGNEEDLE";
    let needle_len = needle.len();

    // Place needle such that it spans across 2nd and 3rd buffer reads
    let mut payload = vec![b'a'; CONTENT_BUFFER_BYTES * 2 - needle_len / 2];
    payload.extend_from_slice(needle.as_bytes());
    payload.extend(vec![b'b'; 100]);
    fs::write(dir.join("three_chunks.bin"), &payload).unwrap();

    let mut cache = SearchCache::walk_fs(dir);
    let indices = guard_indices(cache.search_with_options(
        "content:LONGNEEDLE",
        SearchOptions {
            case_insensitive: false,
        },
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 1);
}

/// Test needle longer than buffer size
#[test]
fn content_filter_needle_exceeds_buffer_size() {
    let temp_dir = TempDir::new("content_long_needle").unwrap();
    let dir = temp_dir.path();

    let needle_len = CONTENT_BUFFER_BYTES + 1000;
    let needle: String = std::iter::repeat_n('N', needle_len).collect();

    let mut payload = vec![b'x'; 50];
    payload.extend_from_slice(needle.as_bytes());
    payload.extend(vec![b'z'; 50]);
    fs::write(dir.join("long_needle.bin"), &payload).unwrap();

    let mut cache = SearchCache::walk_fs(dir);
    let query = format!("content:{needle}");
    let indices = guard_indices(cache.search_with_options(
        &query,
        SearchOptions {
            case_insensitive: false,
        },
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 1);
}

/// Test needle that's exactly overlap size (needle.len() - 1)
#[test]
fn content_filter_needle_equals_overlap_size() {
    let temp_dir = TempDir::new("content_overlap_size").unwrap();
    let dir = temp_dir.path();

    let needle = "AB"; // overlap will be 1
    let mut payload = vec![b'x'; CONTENT_BUFFER_BYTES - 1];
    payload.extend_from_slice(needle.as_bytes());
    payload.extend(vec![b'y'; 10]);
    fs::write(dir.join("overlap.bin"), &payload).unwrap();

    let mut cache = SearchCache::walk_fs(dir);
    let indices = guard_indices(cache.search_with_options(
        "content:AB",
        SearchOptions {
            case_insensitive: false,
        },
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 1);
}

/// Test file smaller than buffer size
#[test]
fn content_filter_file_smaller_than_buffer() {
    let temp_dir = TempDir::new("content_small_file").unwrap();
    let dir = temp_dir.path();

    fs::write(dir.join("tiny.txt"), b"small content here").unwrap();

    let mut cache = SearchCache::walk_fs(dir);
    let indices = guard_indices(cache.search_with_options(
        "content:content",
        SearchOptions {
            case_insensitive: false,
        },
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 1);
}

/// Test empty file
#[test]
fn content_filter_empty_file_returns_no_match() {
    let temp_dir = TempDir::new("content_empty_file").unwrap();
    let dir = temp_dir.path();

    fs::write(dir.join("empty.txt"), b"").unwrap();

    let mut cache = SearchCache::walk_fs(dir);
    let indices = guard_indices(cache.search_with_options(
        "content:anything",
        SearchOptions {
            case_insensitive: false,
        },
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 0);
}

/// Test needle at the very start of file
#[test]
fn content_filter_needle_at_file_start() {
    let temp_dir = TempDir::new("content_file_start").unwrap();
    let dir = temp_dir.path();

    fs::write(dir.join("start.txt"), b"STARTrest of content").unwrap();

    let mut cache = SearchCache::walk_fs(dir);
    let indices = guard_indices(cache.search_with_options(
        "content:START",
        SearchOptions {
            case_insensitive: false,
        },
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 1);
}

/// Test needle at the very end of file
#[test]
fn content_filter_needle_at_file_end() {
    let temp_dir = TempDir::new("content_file_end").unwrap();
    let dir = temp_dir.path();

    fs::write(dir.join("end.txt"), b"content before END").unwrap();

    let mut cache = SearchCache::walk_fs(dir);
    let indices = guard_indices(cache.search_with_options(
        "content:END",
        SearchOptions {
            case_insensitive: false,
        },
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 1);
}

/// Test needle appearing multiple times in same file
#[test]
fn content_filter_multiple_occurrences_in_file() {
    let temp_dir = TempDir::new("content_multiple").unwrap();
    let dir = temp_dir.path();

    fs::write(dir.join("multi.txt"), b"foo bar foo baz foo").unwrap();

    let mut cache = SearchCache::walk_fs(dir);
    let indices = guard_indices(cache.search_with_options(
        "content:foo",
        SearchOptions {
            case_insensitive: false,
        },
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 1); // Still only one file matches
}

/// Test binary content with null bytes
#[test]
fn content_filter_binary_with_null_bytes() {
    let temp_dir = TempDir::new("content_binary").unwrap();
    let dir = temp_dir.path();

    let mut binary = vec![0u8, 1u8, 2u8, 0u8];
    binary.extend_from_slice(b"TARGET");
    binary.extend_from_slice(&[0u8, 255u8, 128u8]);
    fs::write(dir.join("binary.bin"), &binary).unwrap();

    let mut cache = SearchCache::walk_fs(dir);
    let indices = guard_indices(cache.search_with_options(
        "content:TARGET",
        SearchOptions {
            case_insensitive: false,
        },
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 1);
}

/// Test UTF-8 multi-byte characters
#[test]
fn content_filter_utf8_multibyte_characters() {
    let temp_dir = TempDir::new("content_utf8").unwrap();
    let dir = temp_dir.path();

    fs::write(dir.join("utf8.txt"), "Hello 世界 Rust 🦀".as_bytes()).unwrap();

    let mut cache = SearchCache::walk_fs(dir);

    // Search for Chinese characters
    let indices = guard_indices(cache.search_with_options(
        "content:世界",
        SearchOptions {
            case_insensitive: false,
        },
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 1);

    // Search for emoji
    let indices = guard_indices(cache.search_with_options(
        "content:🦀",
        SearchOptions {
            case_insensitive: false,
        },
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 1);
}

/// Test UTF-8 character split across buffer boundary
#[test]
fn content_filter_utf8_split_across_boundary() {
    let temp_dir = TempDir::new("content_utf8_boundary").unwrap();
    let dir = temp_dir.path();

    // Create a file where a multi-byte UTF-8 character might span buffer boundary
    let mut payload = vec![b'a'; CONTENT_BUFFER_BYTES - 2];
    payload.extend_from_slice("世界".as_bytes()); // Chinese chars are 3 bytes each
    payload.extend(vec![b'b'; 100]);
    fs::write(dir.join("utf8_boundary.txt"), &payload).unwrap();

    let mut cache = SearchCache::walk_fs(dir);
    let indices = guard_indices(cache.search_with_options(
        "content:世界",
        SearchOptions {
            case_insensitive: false,
        },
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 1);
}

/// Test special characters and escape sequences
#[test]
fn content_filter_special_characters() {
    let temp_dir = TempDir::new("content_special_chars").unwrap();
    let dir = temp_dir.path();

    fs::write(
        dir.join("special.txt"),
        b"Line1\nLine2\tTabbed\r\nWindows\nSymbols: !@#$%^&*()",
    )
    .unwrap();

    let mut cache = SearchCache::walk_fs(dir);

    // Special symbols (use quotes to preserve the content)
    let indices = guard_indices(cache.search_with_options(
        r#"content:"!@#$%""#,
        SearchOptions {
            case_insensitive: false,
        },
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 1);

    // Test parentheses
    let indices = guard_indices(cache.search_with_options(
        r#"content:"&*()""#,
        SearchOptions {
            case_insensitive: false,
        },
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 1);
}

/// Test directories are filtered out (only files scanned)
#[test]
fn content_filter_ignores_directories() {
    let temp_dir = TempDir::new("content_dirs").unwrap();
    let dir = temp_dir.path();

    fs::create_dir(dir.join("subdir")).unwrap();
    fs::write(dir.join("subdir/file.txt"), b"content").unwrap();

    let mut cache = SearchCache::walk_fs(dir);
    let indices = guard_indices(cache.search_with_options(
        "content:content",
        SearchOptions {
            case_insensitive: false,
        },
        CancellationToken::noop(),
    ));

    // Should only match the file, not the directory
    let nodes = cache.expand_file_nodes(&indices);
    assert_eq!(nodes.len(), 1);
    assert!(nodes[0].path.ends_with("file.txt"));
}

/// Test combining content filter with other filters
#[test]
fn content_filter_combined_with_extension() {
    let temp_dir = TempDir::new("content_combined").unwrap();
    let dir = temp_dir.path();

    fs::write(dir.join("match.txt"), b"Bearer token").unwrap();
    fs::write(dir.join("match.md"), b"Bearer token").unwrap();
    fs::write(dir.join("nomatch.txt"), b"no token").unwrap();

    let mut cache = SearchCache::walk_fs(dir);

    // Search for .txt files containing "Bearer"
    let indices = guard_indices(cache.search_with_options(
        "*.txt content:Bearer",
        SearchOptions {
            case_insensitive: false,
        },
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 1);
    let nodes = cache.expand_file_nodes(&indices);
    assert!(nodes[0].path.ends_with("match.txt"));

    // Search for .md files containing "Bearer"
    let indices = guard_indices(cache.search_with_options(
        "*.md content:Bearer",
        SearchOptions {
            case_insensitive: false,
        },
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 1);
    let nodes = cache.expand_file_nodes(&indices);
    assert!(nodes[0].path.ends_with("match.md"));
}

/// Test content filter with infolder
#[test]
fn content_filter_combined_with_infolder() {
    let temp_dir = TempDir::new("content_infolder").unwrap();
    let dir = temp_dir.path();

    fs::create_dir(dir.join("target")).unwrap();
    fs::create_dir(dir.join("other")).unwrap();
    fs::write(dir.join("target/match.txt"), b"secret key").unwrap();
    fs::write(dir.join("other/nomatch.txt"), b"secret key").unwrap();

    let mut cache = SearchCache::walk_fs(dir);

    let target_path = dir.join("target").to_str().unwrap().to_string();
    let query = format!("infolder:{target_path} content:\"secret key\"");
    let indices = guard_indices(cache.search_with_options(
        &query,
        SearchOptions {
            case_insensitive: false,
        },
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 1);
    let nodes = cache.expand_file_nodes(&indices);
    assert!(nodes[0].path.to_str().unwrap().contains("target"));
}

/// Test content filter with size filter
#[test]
fn content_filter_combined_with_size() {
    let temp_dir = TempDir::new("content_size").unwrap();
    let dir = temp_dir.path();

    fs::write(dir.join("small.txt"), b"test").unwrap(); // 4 bytes
    fs::write(dir.join("large.txt"), vec![b't'; 2000]).unwrap(); // 2000 bytes

    let mut cache = SearchCache::walk_fs(dir);

    // Search for files > 1KB containing 't'
    let indices = guard_indices(cache.search_with_options(
        "size:>1kb content:t",
        SearchOptions {
            case_insensitive: false,
        },
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 1);
    let nodes = cache.expand_file_nodes(&indices);
    assert!(nodes[0].path.ends_with("large.txt"));
}

/// Test content filter with NOT operator
#[test]
fn content_filter_with_not_operator() {
    let temp_dir = TempDir::new("content_not").unwrap();
    let dir = temp_dir.path();

    fs::write(dir.join("has_secret.txt"), b"secret key").unwrap();
    fs::write(dir.join("no_secret.txt"), b"public data").unwrap();

    let mut cache = SearchCache::walk_fs(dir);

    // Search for .txt files that don't contain "secret"
    let indices = guard_indices(cache.search_with_options(
        "*.txt !content:secret",
        SearchOptions {
            case_insensitive: false,
        },
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 1);
    let nodes = cache.expand_file_nodes(&indices);
    assert!(nodes[0].path.ends_with("no_secret.txt"));
}

/// Test content filter with OR operator
#[test]
fn content_filter_with_or_operator() {
    let temp_dir = TempDir::new("content_or").unwrap();
    let dir = temp_dir.path();

    fs::write(dir.join("todo.txt"), b"TODO: fix bug").unwrap();
    fs::write(dir.join("fixme.txt"), b"FIXME: urgent").unwrap();
    fs::write(dir.join("clean.txt"), b"all good").unwrap();

    let mut cache = SearchCache::walk_fs(dir);

    // Search for files containing either TODO or FIXME
    let indices = guard_indices(cache.search_with_options(
        "content:TODO | content:FIXME",
        SearchOptions {
            case_insensitive: false,
        },
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 2);
    let nodes = cache.expand_file_nodes(&indices);
    assert!(nodes.iter().any(|n| n.path.ends_with("todo.txt")));
    assert!(nodes.iter().any(|n| n.path.ends_with("fixme.txt")));
}

/// Test cancellation during content scanning
#[test]
fn content_filter_respects_cancellation() {
    let temp_dir = TempDir::new("content_cancel").unwrap();
    let dir = temp_dir.path();

    // Create multiple large files
    for i in 0..10 {
        let content = vec![b'x'; CONTENT_BUFFER_BYTES * 2];
        fs::write(dir.join(format!("large{i}.bin")), content).unwrap();
    }

    let mut cache = SearchCache::walk_fs(dir);

    // Create a cancelled token
    let token = CancellationToken::new(999);
    let _ = CancellationToken::new(1000); // This cancels token 999

    let result = cache.search_with_options(
        "content:needle",
        SearchOptions {
            case_insensitive: false,
        },
        token,
    );

    // Should return cancelled outcome
    assert!(matches!(
        result,
        Ok(search_cache::SearchOutcome { nodes: None, .. })
    ));
}

/// Test unreadable file (permission denied scenario simulation)
#[test]
#[cfg(unix)]
fn content_filter_handles_unreadable_file() {
    use std::os::unix::fs::PermissionsExt;

    let temp_dir = TempDir::new("content_unreadable").unwrap();
    let dir = temp_dir.path();

    let unreadable = dir.join("unreadable.txt");
    fs::write(&unreadable, b"secret").unwrap();
    fs::write(dir.join("readable.txt"), b"secret").unwrap();

    // Remove read permissions
    let mut perms = fs::metadata(&unreadable).unwrap().permissions();
    perms.set_mode(0o000);
    fs::set_permissions(&unreadable, perms).unwrap();

    let mut cache = SearchCache::walk_fs(dir);
    let indices = guard_indices(cache.search_with_options(
        "content:secret",
        SearchOptions {
            case_insensitive: false,
        },
        CancellationToken::noop(),
    ));

    // Should only match readable file
    assert_eq!(indices.len(), 1);
    let nodes = cache.expand_file_nodes(&indices);
    assert!(nodes[0].path.ends_with("readable.txt"));

    // Restore permissions for cleanup
    let mut perms = fs::metadata(&unreadable).unwrap().permissions();
    perms.set_mode(0o644);
    fs::set_permissions(&unreadable, perms).unwrap();
}

/// Test needle that looks like regex special characters
#[test]
fn content_filter_regex_special_chars_treated_literally() {
    let temp_dir = TempDir::new("content_regex_chars").unwrap();
    let dir = temp_dir.path();

    fs::write(dir.join("regex.txt"), b"file.* [test]+ (group)? ^start$").unwrap();

    let mut cache = SearchCache::walk_fs(dir);

    // These should be treated as literal strings, not regex
    let indices = guard_indices(cache.search_with_options(
        "content:.*",
        SearchOptions {
            case_insensitive: false,
        },
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 1);

    let indices = guard_indices(cache.search_with_options(
        "content:[test]+",
        SearchOptions {
            case_insensitive: false,
        },
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 1);

    let indices = guard_indices(cache.search_with_options(
        "content:(group)?",
        SearchOptions {
            case_insensitive: false,
        },
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 1);
}

/// Test very long line without newlines
#[test]
fn content_filter_handles_long_lines() {
    let temp_dir = TempDir::new("content_long_line").unwrap();
    let dir = temp_dir.path();

    // Create a file with one very long line
    let mut content = vec![b'a'; CONTENT_BUFFER_BYTES * 3];
    content.extend_from_slice(b"NEEDLE");
    content.extend(vec![b'b'; 1000]);
    fs::write(dir.join("long_line.txt"), content).unwrap();

    let mut cache = SearchCache::walk_fs(dir);
    let indices = guard_indices(cache.search_with_options(
        "content:NEEDLE",
        SearchOptions {
            case_insensitive: false,
        },
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 1);
}

/// Test whitespace handling - spaces, tabs, newlines in needle
#[test]
fn content_filter_whitespace_in_needle() {
    let temp_dir = TempDir::new("content_whitespace").unwrap();
    let dir = temp_dir.path();

    fs::write(dir.join("whitespace.txt"), b"line one word three   spaced").unwrap();

    let mut cache = SearchCache::walk_fs(dir);

    // Search with space (use quotes to preserve the space)
    let indices = guard_indices(cache.search_with_options(
        r#"content:"word three""#,
        SearchOptions {
            case_insensitive: false,
        },
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 1);

    // Search with multiple spaces
    let indices = guard_indices(cache.search_with_options(
        r#"content:"three   spaced""#,
        SearchOptions {
            case_insensitive: false,
        },
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 1);
}

/// Test case insensitive matching with mixed case
#[test]
fn content_filter_case_insensitive_mixed_case() {
    let temp_dir = TempDir::new("content_mixed_case").unwrap();
    let dir = temp_dir.path();

    fs::write(dir.join("mixed.txt"), b"ThIsIsMiXeDCaSe").unwrap();

    let mut cache = SearchCache::walk_fs(dir);

    // All variations should match case-insensitively (using quoted strings)
    for needle in ["thisismixedcase", "THISISMIXEDCASE", "ThIsIsMiXeDCaSe"] {
        let indices = guard_indices(cache.search_with_options(
            &format!(r#"content:"{needle}""#),
            SearchOptions {
                case_insensitive: true,
            },
            CancellationToken::noop(),
        ));
        assert_eq!(indices.len(), 1, "Failed for needle: {needle}");
    }
}

/// Test exact buffer size file
#[test]
fn content_filter_file_exact_buffer_size() {
    let temp_dir = TempDir::new("content_exact_buffer").unwrap();
    let dir = temp_dir.path();

    let mut content = vec![b'x'; CONTENT_BUFFER_BYTES - 10];
    content.extend_from_slice(b"TARGET");
    content.extend(vec![b'y'; 4]); // Total = CONTENT_BUFFER_BYTES
    fs::write(dir.join("exact.bin"), &content).unwrap();

    let mut cache = SearchCache::walk_fs(dir);
    let indices = guard_indices(cache.search_with_options(
        "content:TARGET",
        SearchOptions {
            case_insensitive: false,
        },
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 1);
}

/// Test partial match at end of file (needle extends beyond file)
#[test]
fn content_filter_no_false_positive_at_eof() {
    let temp_dir = TempDir::new("content_eof").unwrap();
    let dir = temp_dir.path();

    // File ends with partial match of needle
    fs::write(dir.join("partial.txt"), b"data ends with TARG").unwrap();

    let mut cache = SearchCache::walk_fs(dir);
    let indices = guard_indices(cache.search_with_options(
        "content:TARGET",
        SearchOptions {
            case_insensitive: false,
        },
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 0); // Should not match partial
}

/// Test repeated pattern in needle
#[test]
fn content_filter_repeated_pattern_in_needle() {
    let temp_dir = TempDir::new("content_repeated").unwrap();
    let dir = temp_dir.path();

    fs::write(dir.join("repeat.txt"), b"aaaaaaBBBBBBaaaaaa").unwrap();

    let mut cache = SearchCache::walk_fs(dir);
    let indices = guard_indices(cache.search_with_options(
        "content:aaaaaa",
        SearchOptions {
            case_insensitive: false,
        },
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 1);

    let indices = guard_indices(cache.search_with_options(
        "content:BBBBBB",
        SearchOptions {
            case_insensitive: false,
        },
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 1);
}

/// Test all ASCII characters
#[test]
fn content_filter_all_ascii_printable() {
    let temp_dir = TempDir::new("content_ascii").unwrap();
    let dir = temp_dir.path();

    // All printable ASCII characters
    let ascii: Vec<u8> = (32..127).collect();
    fs::write(dir.join("ascii.txt"), &ascii).unwrap();

    let mut cache = SearchCache::walk_fs(dir);

    // Test finding a substring
    let indices = guard_indices(cache.search_with_options(
        "content:@ABCDEF",
        SearchOptions {
            case_insensitive: false,
        },
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 1);
}

/// Test high bytes (non-ASCII)
#[test]
fn content_filter_high_bytes() {
    let temp_dir = TempDir::new("content_high_bytes").unwrap();
    let dir = temp_dir.path();

    let content: Vec<u8> = vec![0x80, 0x90, 0xFF, b'A', b'B', 0xFE, 0xFD];
    fs::write(dir.join("high.bin"), &content).unwrap();

    let mut cache = SearchCache::walk_fs(dir);
    let indices = guard_indices(cache.search_with_options(
        "content:AB",
        SearchOptions {
            case_insensitive: false,
        },
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 1);
}
