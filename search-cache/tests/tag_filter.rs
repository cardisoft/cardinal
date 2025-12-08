#![cfg(target_os = "macos")]

use plist::Value;
use search_cache::{SearchCache, SearchOptions, SlabIndex};
use search_cancel::CancellationToken;
use std::{fs, path::Path};
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
    let data = plist::to_bytes_binary(&Value::Array(values)).expect("serialize tags");
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
fn tag_filter_supports_lists_and_substring() {
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
        "tag:Alpha;Important",
        SearchOptions {
            case_insensitive: false,
        },
        CancellationToken::noop(),
    ));
    assert_eq!(indices.len(), 1);
    let nodes = cache.expand_file_nodes(&indices);
    assert!(nodes[0].path.ends_with("first.txt"));
}
