#![cfg(target_os = "macos")]

use plist::{Value, to_writer_binary};
use search_cache::{SearchCache, SearchOptions, SlabIndex};
use search_cancel::CancellationToken;
use std::{
    fs,
    path::{Path, PathBuf},
};
use tempdir::TempDir;
use xattr::set;

const USER_TAG_XATTR: &str = "com.apple.metadata:_kMDItemUserTags";

fn guard_indices(result: Result<search_cache::SearchOutcome, anyhow::Error>) -> Vec<SlabIndex> {
    result
        .expect("search should succeed")
        .nodes
        .expect("noop token should not cancel")
}

fn write_tags(path: &Path, tags: &[&str]) {
    let values: Vec<Value> = tags
        .iter()
        .map(|tag| Value::String(format!("{tag}\n0")))
        .collect();
    let mut data = Vec::new();
    to_writer_binary(&mut data, &Value::Array(values)).expect("serialize tags");
    set(path, USER_TAG_XATTR, &data).expect("write tag xattr");
}

#[test]
fn tag_filter_requires_value() {
    let temp_dir = TempDir::new("tag_filter_empty").unwrap();
    let dir = temp_dir.path();
    fs::write(dir.join("file.txt"), b"dummy").unwrap();
    write_tags(&dir.join("file.txt"), &["Project"]);

    let mut cache = SearchCache::walk_fs(dir.to_path_buf());
    let result = cache.search_with_options(
        r#"tag:"""#,
        SearchOptions::default(),
        CancellationToken::noop(),
    );
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("tag: requires a value")
    );
}

#[test]
fn tag_filter_matches_case_insensitive() {
    let temp_dir = TempDir::new("tag_filter_basic").unwrap();
    let dir = temp_dir.path();

    let first = dir.join("first.txt");
    fs::write(&first, b"dummy").unwrap();
    write_tags(&first, &["Project", "Important"]);

    let second = dir.join("second.txt");
    fs::write(&second, b"dummy").unwrap();
    write_tags(&second, &["Archive"]);

    let mut cache = SearchCache::walk_fs(dir.to_path_buf());
    let indices = guard_indices(cache.search_with_options(
        "tag:project",
        SearchOptions {
            case_insensitive: true,
        },
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 1);
    let nodes = cache.expand_file_nodes(&indices);
    assert!(nodes[0].path.ends_with("first.txt"));
}

#[test]
fn tag_filter_matches_substring() {
    let temp_dir = TempDir::new("tag_filter_list").unwrap();
    let dir = temp_dir.path();

    let first = dir.join("first.txt");
    fs::write(&first, b"dummy").unwrap();
    write_tags(&first, &["Project-Alpha"]);

    let second = dir.join("second.txt");
    fs::write(&second, b"dummy").unwrap();
    write_tags(&second, &["Archive"]);

    let mut cache = SearchCache::walk_fs(dir.to_path_buf());
    let indices = guard_indices(cache.search_with_options(
        "tag:Alpha",
        SearchOptions {
            case_insensitive: false,
        },
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 1);
    let nodes = cache.expand_file_nodes(&indices);
    assert!(nodes[0].path.ends_with("first.txt"));
}

#[test]
fn tag_filter_rejects_semicolon_list() {
    let temp_dir = TempDir::new("tag_filter_list_error").unwrap();
    let dir = temp_dir.path();

    let first = dir.join("first.txt");
    fs::write(&first, b"dummy").unwrap();
    write_tags(&first, &["Project-Alpha"]);

    let mut cache = SearchCache::walk_fs(dir.to_path_buf());
    let result = cache.search_with_options(
        "tag:Alpha;Important",
        SearchOptions::default(),
        CancellationToken::noop(),
    );
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("tag: accepts a single value")
    );
}

#[test]
fn tag_filter_case_sensitive_exact_match() {
    let temp_dir = TempDir::new("tag_case_sensitive").unwrap();
    let dir = temp_dir.path();

    let first = dir.join("first.txt");
    fs::write(&first, b"dummy").unwrap();
    write_tags(&first, &["Project"]);

    let second = dir.join("second.txt");
    fs::write(&second, b"dummy").unwrap();
    write_tags(&second, &["project"]);

    let mut cache = SearchCache::walk_fs(dir.to_path_buf());
    let indices = guard_indices(cache.search_with_options(
        "tag:Project",
        SearchOptions {
            case_insensitive: false,
        },
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 1);
    let nodes = cache.expand_file_nodes(&indices);
    assert!(nodes[0].path.ends_with("first.txt"));
}

#[test]
fn tag_filter_case_insensitive_matches_both() {
    let temp_dir = TempDir::new("tag_case_insensitive").unwrap();
    let dir = temp_dir.path();

    let first = dir.join("first.txt");
    fs::write(&first, b"dummy").unwrap();
    write_tags(&first, &["Project"]);

    let second = dir.join("second.txt");
    fs::write(&second, b"dummy").unwrap();
    write_tags(&second, &["project"]);

    let third = dir.join("third.txt");
    fs::write(&third, b"dummy").unwrap();
    write_tags(&third, &["PROJECT"]);

    let mut cache = SearchCache::walk_fs(dir.to_path_buf());
    let indices = guard_indices(cache.search_with_options(
        "tag:project",
        SearchOptions {
            case_insensitive: true,
        },
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 3);
}

#[test]
fn tag_filter_substring_at_start() {
    let temp_dir = TempDir::new("tag_substring_start").unwrap();
    let dir = temp_dir.path();

    let first = dir.join("first.txt");
    fs::write(&first, b"dummy").unwrap();
    write_tags(&first, &["Important-Task"]);

    let mut cache = SearchCache::walk_fs(dir.to_path_buf());
    let indices = guard_indices(cache.search_with_options(
        "tag:Import",
        SearchOptions::default(),
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 1);
}

#[test]
fn tag_filter_substring_at_end() {
    let temp_dir = TempDir::new("tag_substring_end").unwrap();
    let dir = temp_dir.path();

    let first = dir.join("first.txt");
    fs::write(&first, b"dummy").unwrap();
    write_tags(&first, &["Important-Task"]);

    let mut cache = SearchCache::walk_fs(dir.to_path_buf());
    let indices = guard_indices(cache.search_with_options(
        "tag:Task",
        SearchOptions::default(),
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 1);
}

#[test]
fn tag_filter_substring_in_middle() {
    let temp_dir = TempDir::new("tag_substring_middle").unwrap();
    let dir = temp_dir.path();

    let first = dir.join("first.txt");
    fs::write(&first, b"dummy").unwrap();
    write_tags(&first, &["Project-Alpha-2024"]);

    let mut cache = SearchCache::walk_fs(dir.to_path_buf());
    let indices = guard_indices(cache.search_with_options(
        "tag:Alpha",
        SearchOptions::default(),
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 1);
}

#[test]
fn tag_filter_multiple_tags_and_logic() {
    let temp_dir = TempDir::new("tag_multiple_and").unwrap();
    let dir = temp_dir.path();

    let first = dir.join("first.txt");
    fs::write(&first, b"dummy").unwrap();
    write_tags(&first, &["Project", "Important"]);

    let second = dir.join("second.txt");
    fs::write(&second, b"dummy").unwrap();
    write_tags(&second, &["Project"]);

    let third = dir.join("third.txt");
    fs::write(&third, b"dummy").unwrap();
    write_tags(&third, &["Important"]);

    let mut cache = SearchCache::walk_fs(dir.to_path_buf());
    let indices = guard_indices(cache.search_with_options(
        "tag:Project tag:Important",
        SearchOptions::default(),
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 1);
    let nodes = cache.expand_file_nodes(&indices);
    assert!(nodes[0].path.ends_with("first.txt"));
}

#[test]
fn tag_filter_three_tags_and_logic() {
    let temp_dir = TempDir::new("tag_three_and").unwrap();
    let dir = temp_dir.path();

    let first = dir.join("first.txt");
    fs::write(&first, b"dummy").unwrap();
    write_tags(&first, &["Project", "Important", "Urgent"]);

    let second = dir.join("second.txt");
    fs::write(&second, b"dummy").unwrap();
    write_tags(&second, &["Project", "Important"]);

    let mut cache = SearchCache::walk_fs(dir.to_path_buf());
    let indices = guard_indices(cache.search_with_options(
        "tag:Project tag:Important tag:Urgent",
        SearchOptions::default(),
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 1);
    let nodes = cache.expand_file_nodes(&indices);
    assert!(nodes[0].path.ends_with("first.txt"));
}

#[test]
fn tag_filter_or_logic() {
    let temp_dir = TempDir::new("tag_or_logic").unwrap();
    let dir = temp_dir.path();

    let first = dir.join("first.txt");
    fs::write(&first, b"dummy").unwrap();
    write_tags(&first, &["Project"]);

    let second = dir.join("second.txt");
    fs::write(&second, b"dummy").unwrap();
    write_tags(&second, &["Important"]);

    let third = dir.join("third.txt");
    fs::write(&third, b"dummy").unwrap();
    write_tags(&third, &["Archive"]);

    let mut cache = SearchCache::walk_fs(dir.to_path_buf());
    let indices = guard_indices(cache.search_with_options(
        "tag:Project | tag:Important",
        SearchOptions::default(),
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 2);
}

#[test]
fn tag_filter_with_not_operator() {
    let temp_dir = TempDir::new("tag_not_operator").unwrap();
    let dir = temp_dir.path();

    let first = dir.join("first.txt");
    fs::write(&first, b"dummy").unwrap();
    write_tags(&first, &["Project"]);

    let second = dir.join("second.txt");
    fs::write(&second, b"dummy").unwrap();
    write_tags(&second, &["Important"]);

    let third = dir.join("third.txt");
    fs::write(&third, b"dummy").unwrap();
    write_tags(&third, &["Archive"]);

    let fourth = dir.join("fourth.txt");
    fs::write(&fourth, b"dummy").unwrap();

    let mut cache = SearchCache::walk_fs(dir.to_path_buf());
    let indices = guard_indices(cache.search_with_options(
        "!tag:Archive",
        SearchOptions::default(),
        CancellationToken::noop(),
    ));
    // Should match: first.txt (Project), second.txt (Important), fourth.txt (no tags), and the temp dir itself
    assert_eq!(indices.len(), 4);
}

#[test]
fn tag_filter_empty_tag_argument() {
    let temp_dir = TempDir::new("tag_empty_arg").unwrap();
    let dir = temp_dir.path();

    let first = dir.join("first.txt");
    fs::write(&first, b"dummy").unwrap();
    write_tags(&first, &["Project"]);

    let mut cache = SearchCache::walk_fs(dir.to_path_buf());
    let result =
        cache.search_with_options("tag:", SearchOptions::default(), CancellationToken::noop());
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("tag: requires a value")
    );
}

#[test]
fn tag_filter_whitespace_only_argument() {
    let temp_dir = TempDir::new("tag_whitespace_only").unwrap();
    let dir = temp_dir.path();

    let first = dir.join("first.txt");
    fs::write(&first, b"dummy").unwrap();
    write_tags(&first, &["Project"]);

    let mut cache = SearchCache::walk_fs(dir.to_path_buf());
    let result = cache.search_with_options(
        "tag:   ",
        SearchOptions::default(),
        CancellationToken::noop(),
    );
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("tag: requires a value")
    );
}

#[test]
fn tag_filter_quoted_with_whitespace() {
    let temp_dir = TempDir::new("tag_quoted_whitespace").unwrap();
    let dir = temp_dir.path();

    let first = dir.join("first.txt");
    fs::write(&first, b"dummy").unwrap();
    write_tags(&first, &["Project Alpha"]);

    let mut cache = SearchCache::walk_fs(dir.to_path_buf());
    let indices = guard_indices(cache.search_with_options(
        r#"tag:"Project Alpha""#,
        SearchOptions::default(),
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 1);
}

#[test]
fn tag_filter_no_match() {
    let temp_dir = TempDir::new("tag_no_match").unwrap();
    let dir = temp_dir.path();

    let first = dir.join("first.txt");
    fs::write(&first, b"dummy").unwrap();
    write_tags(&first, &["Project"]);

    let mut cache = SearchCache::walk_fs(dir.to_path_buf());
    let indices = guard_indices(cache.search_with_options(
        "tag:Nonexistent",
        SearchOptions::default(),
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 0);
}

#[test]
fn tag_filter_file_with_no_tags() {
    let temp_dir = TempDir::new("tag_no_tags").unwrap();
    let dir = temp_dir.path();

    let first = dir.join("first.txt");
    fs::write(&first, b"dummy").unwrap();

    let mut cache = SearchCache::walk_fs(dir.to_path_buf());
    let indices = guard_indices(cache.search_with_options(
        "tag:Project",
        SearchOptions::default(),
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 0);
}

#[test]
fn tag_filter_file_with_multiple_tags() {
    let temp_dir = TempDir::new("tag_multiple_tags").unwrap();
    let dir = temp_dir.path();

    let first = dir.join("first.txt");
    fs::write(&first, b"dummy").unwrap();
    write_tags(&first, &["Project", "Important", "Urgent", "Q4"]);

    let mut cache = SearchCache::walk_fs(dir.to_path_buf());
    let indices = guard_indices(cache.search_with_options(
        "tag:Important",
        SearchOptions::default(),
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 1);
}

#[test]
fn tag_filter_unicode_tag_name() {
    let temp_dir = TempDir::new("tag_unicode").unwrap();
    let dir = temp_dir.path();

    let first = dir.join("first.txt");
    fs::write(&first, b"dummy").unwrap();
    write_tags(&first, &["项目", "重要"]);

    let mut cache = SearchCache::walk_fs(dir.to_path_buf());
    let indices = guard_indices(cache.search_with_options(
        "tag:项目",
        SearchOptions::default(),
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 1);
}

#[test]
fn tag_filter_emoji_in_tag() {
    let temp_dir = TempDir::new("tag_emoji").unwrap();
    let dir = temp_dir.path();

    let first = dir.join("first.txt");
    fs::write(&first, b"dummy").unwrap();
    write_tags(&first, &["🔴Important", "⭐Project"]);

    let mut cache = SearchCache::walk_fs(dir.to_path_buf());
    let indices = guard_indices(cache.search_with_options(
        "tag:🔴",
        SearchOptions::default(),
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 1);
}

#[test]
fn tag_filter_special_characters() {
    let temp_dir = TempDir::new("tag_special_chars").unwrap();
    let dir = temp_dir.path();

    let first = dir.join("first.txt");
    fs::write(&first, b"dummy").unwrap();
    write_tags(&first, &["Project-2024", "To-Do", "Work/Personal"]);

    let mut cache = SearchCache::walk_fs(dir.to_path_buf());
    let indices = guard_indices(cache.search_with_options(
        "tag:Project-2024",
        SearchOptions::default(),
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 1);
}

#[test]
fn tag_filter_with_hyphen() {
    let temp_dir = TempDir::new("tag_hyphen").unwrap();
    let dir = temp_dir.path();

    let first = dir.join("first.txt");
    fs::write(&first, b"dummy").unwrap();
    write_tags(&first, &["Work-In-Progress"]);

    let mut cache = SearchCache::walk_fs(dir.to_path_buf());
    let indices = guard_indices(cache.search_with_options(
        "tag:Work-In-Progress",
        SearchOptions::default(),
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 1);
}

#[test]
fn tag_filter_with_numbers() {
    let temp_dir = TempDir::new("tag_numbers").unwrap();
    let dir = temp_dir.path();

    let first = dir.join("first.txt");
    fs::write(&first, b"dummy").unwrap();
    write_tags(&first, &["Q4-2024", "Priority1"]);

    let mut cache = SearchCache::walk_fs(dir.to_path_buf());
    let indices = guard_indices(cache.search_with_options(
        "tag:2024",
        SearchOptions::default(),
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 1);
}

#[test]
fn tag_filter_combined_with_word_search() {
    let temp_dir = TempDir::new("tag_with_word").unwrap();
    let dir = temp_dir.path();

    let first = dir.join("report.txt");
    fs::write(&first, b"dummy").unwrap();
    write_tags(&first, &["Project"]);

    let second = dir.join("notes.txt");
    fs::write(&second, b"dummy").unwrap();
    write_tags(&second, &["Project"]);

    let mut cache = SearchCache::walk_fs(dir.to_path_buf());
    let indices = guard_indices(cache.search_with_options(
        "tag:Project report",
        SearchOptions::default(),
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 1);
    let nodes = cache.expand_file_nodes(&indices);
    assert!(nodes[0].path.ends_with("report.txt"));
}

#[test]
fn tag_filter_combined_with_ext_filter() {
    let temp_dir = TempDir::new("tag_with_ext").unwrap();
    let dir = temp_dir.path();

    let first = dir.join("file.txt");
    fs::write(&first, b"dummy").unwrap();
    write_tags(&first, &["Project"]);

    let second = dir.join("file.md");
    fs::write(&second, b"dummy").unwrap();
    write_tags(&second, &["Project"]);

    let mut cache = SearchCache::walk_fs(dir.to_path_buf());
    let indices = guard_indices(cache.search_with_options(
        "tag:Project ext:txt",
        SearchOptions::default(),
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 1);
    let nodes = cache.expand_file_nodes(&indices);
    assert!(nodes[0].path.ends_with("file.txt"));
}

#[test]
fn tag_filter_combined_with_type_filter() {
    let temp_dir = TempDir::new("tag_with_type").unwrap();
    let dir = temp_dir.path();

    let first = dir.join("file.txt");
    fs::write(&first, b"dummy").unwrap();
    write_tags(&first, &["Project"]);

    let second = dir.join("subdir");
    fs::create_dir(&second).unwrap();
    write_tags(&second, &["Project"]);

    let mut cache = SearchCache::walk_fs(dir.to_path_buf());
    let indices = guard_indices(cache.search_with_options(
        "tag:Project type:file",
        SearchOptions::default(),
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 1);
    let nodes = cache.expand_file_nodes(&indices);
    assert!(nodes[0].path.ends_with("file.txt"));
}

#[test]
fn tag_filter_nested_boolean_logic() {
    let temp_dir = TempDir::new("tag_nested_boolean").unwrap();
    let dir = temp_dir.path();

    let first = dir.join("first.txt");
    fs::write(&first, b"dummy").unwrap();
    write_tags(&first, &["Project", "Important"]);

    let second = dir.join("second.txt");
    fs::write(&second, b"dummy").unwrap();
    write_tags(&second, &["Archive", "Important"]);

    let third = dir.join("third.txt");
    fs::write(&third, b"dummy").unwrap();
    write_tags(&third, &["Project"]);

    let mut cache = SearchCache::walk_fs(dir.to_path_buf());
    let indices = guard_indices(cache.search_with_options(
        "(tag:Project | tag:Archive) tag:Important",
        SearchOptions::default(),
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 2);
}

#[test]
fn tag_filter_quoted_tag_name() {
    let temp_dir = TempDir::new("tag_quoted").unwrap();
    let dir = temp_dir.path();

    let first = dir.join("first.txt");
    fs::write(&first, b"dummy").unwrap();
    write_tags(&first, &["Q4 Report"]);

    let mut cache = SearchCache::walk_fs(dir.to_path_buf());
    let indices = guard_indices(cache.search_with_options(
        r#"tag:"Q4 Report""#,
        SearchOptions::default(),
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 1);
}

#[test]
fn tag_filter_partial_quoted_match() {
    let temp_dir = TempDir::new("tag_quoted_partial").unwrap();
    let dir = temp_dir.path();

    let first = dir.join("first.txt");
    fs::write(&first, b"dummy").unwrap();
    write_tags(&first, &["Q4 Report 2024"]);

    let mut cache = SearchCache::walk_fs(dir.to_path_buf());
    let indices = guard_indices(cache.search_with_options(
        r#"tag:"Q4 Report""#,
        SearchOptions::default(),
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 1);
}

#[test]
fn tag_filter_tags_on_directory() {
    let temp_dir = TempDir::new("tag_on_directory").unwrap();
    let dir = temp_dir.path();

    let subdir = dir.join("project");
    fs::create_dir(&subdir).unwrap();
    write_tags(&subdir, &["Important"]);

    let file = subdir.join("file.txt");
    fs::write(&file, b"dummy").unwrap();

    let mut cache = SearchCache::walk_fs(dir.to_path_buf());
    let indices = guard_indices(cache.search_with_options(
        "tag:Important",
        SearchOptions::default(),
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 1);
    let nodes = cache.expand_file_nodes(&indices);
    assert!(nodes[0].path.ends_with("project"));
}

#[test]
fn tag_filter_mixed_files_and_directories() {
    let temp_dir = TempDir::new("tag_mixed_types").unwrap();
    let dir = temp_dir.path();

    let file1 = dir.join("file1.txt");
    fs::write(&file1, b"dummy").unwrap();
    write_tags(&file1, &["Project"]);

    let subdir = dir.join("project_dir");
    fs::create_dir(&subdir).unwrap();
    write_tags(&subdir, &["Project"]);

    let file2 = dir.join("file2.txt");
    fs::write(&file2, b"dummy").unwrap();
    write_tags(&file2, &["Project"]);

    let mut cache = SearchCache::walk_fs(dir.to_path_buf());
    let indices = guard_indices(cache.search_with_options(
        "tag:Project",
        SearchOptions::default(),
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 3);
}

#[test]
fn tag_filter_with_wildcard_in_filename() {
    let temp_dir = TempDir::new("tag_wildcard_filename").unwrap();
    let dir = temp_dir.path();

    let first = dir.join("report-2024.txt");
    fs::write(&first, b"dummy").unwrap();
    write_tags(&first, &["Project"]);

    let second = dir.join("report-2023.txt");
    fs::write(&second, b"dummy").unwrap();
    write_tags(&second, &["Archive"]);

    let mut cache = SearchCache::walk_fs(dir.to_path_buf());
    let indices = guard_indices(cache.search_with_options(
        "tag:Project report-*.txt",
        SearchOptions::default(),
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 1);
    let nodes = cache.expand_file_nodes(&indices);
    assert!(nodes[0].path.ends_with("report-2024.txt"));
}

#[test]
fn tag_filter_single_character_tag() {
    let temp_dir = TempDir::new("tag_single_char").unwrap();
    let dir = temp_dir.path();

    let first = dir.join("first.txt");
    fs::write(&first, b"dummy").unwrap();
    write_tags(&first, &["A", "B"]);

    let mut cache = SearchCache::walk_fs(dir.to_path_buf());
    let indices = guard_indices(cache.search_with_options(
        "tag:A",
        SearchOptions::default(),
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 1);
}

#[test]
#[ignore = "This test is slow and should be run manually when needed"]
fn tag_mdfind_speed() {
    let mut cache = SearchCache::walk_fs(PathBuf::from("/"));
    let now = std::time::Instant::now();
    guard_indices(cache.search_with_options(
        "tag:A",
        SearchOptions::default(),
        CancellationToken::noop(),
    ));
    let elapsed = now.elapsed();
    println!("mdfind search took: {:?}", elapsed);
    assert!(elapsed.as_secs() < 1, "Search using mdfind took too long");
}

#[test]
fn tag_filter_very_long_tag_name() {
    let temp_dir = TempDir::new("tag_long_name").unwrap();
    let dir = temp_dir.path();

    let long_tag =
        "VeryLongTagNameThatExceedsNormalExpectationsForTagLength2024ProjectImportantUrgent";
    let first = dir.join("first.txt");
    fs::write(&first, b"dummy").unwrap();
    write_tags(&first, &[long_tag]);

    let mut cache = SearchCache::walk_fs(dir.to_path_buf());
    let indices = guard_indices(cache.search_with_options(
        &format!("tag:{}", long_tag),
        SearchOptions::default(),
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 1);
}

#[test]
fn tag_filter_case_sensitive_substring() {
    let temp_dir = TempDir::new("tag_case_sensitive_substring").unwrap();
    let dir = temp_dir.path();

    let first = dir.join("first.txt");
    fs::write(&first, b"dummy").unwrap();
    write_tags(&first, &["ProjectAlpha"]);

    let second = dir.join("second.txt");
    fs::write(&second, b"dummy").unwrap();
    write_tags(&second, &["projectalpha"]);

    let mut cache = SearchCache::walk_fs(dir.to_path_buf());
    let indices = guard_indices(cache.search_with_options(
        "tag:Alpha",
        SearchOptions {
            case_insensitive: false,
        },
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 1);
    let nodes = cache.expand_file_nodes(&indices);
    assert!(nodes[0].path.ends_with("first.txt"));
}

#[test]
fn tag_filter_duplicate_tag_filters_and() {
    let temp_dir = TempDir::new("tag_duplicate_and").unwrap();
    let dir = temp_dir.path();

    let first = dir.join("first.txt");
    fs::write(&first, b"dummy").unwrap();
    write_tags(&first, &["Project"]);

    let mut cache = SearchCache::walk_fs(dir.to_path_buf());
    let indices = guard_indices(cache.search_with_options(
        "tag:Project tag:Project",
        SearchOptions::default(),
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 1);
}

#[test]
fn tag_filter_mixed_case_in_query() {
    let temp_dir = TempDir::new("tag_mixed_case_query").unwrap();
    let dir = temp_dir.path();

    let first = dir.join("first.txt");
    fs::write(&first, b"dummy").unwrap();
    write_tags(&first, &["project"]);

    let mut cache = SearchCache::walk_fs(dir.to_path_buf());
    let indices = guard_indices(cache.search_with_options(
        "tag:PrOjEcT",
        SearchOptions {
            case_insensitive: true,
        },
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 1);
}

#[test]
fn tag_filter_performance_many_files() {
    let temp_dir = TempDir::new("tag_performance").unwrap();
    let dir = temp_dir.path();

    for i in 0..100 {
        let file = dir.join(format!("file{}.txt", i));
        fs::write(&file, b"dummy").unwrap();
        if i % 3 == 0 {
            write_tags(&file, &["Project"]);
        } else if i % 3 == 1 {
            write_tags(&file, &["Important"]);
        }
    }

    let mut cache = SearchCache::walk_fs(dir.to_path_buf());
    let indices = guard_indices(cache.search_with_options(
        "tag:Project",
        SearchOptions::default(),
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 34);
}
