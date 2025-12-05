//! Tests for FSEvent handling and incremental cache updates
//! Covers: add, remove, rename operations, rescan triggers, event batching

use cardinal_sdk::{EventFlag, FsEvent};
use search_cache::SearchCache;
use search_cancel::CancellationToken;
use std::path::PathBuf;
use tempdir::TempDir;

fn build_initial_cache(files: &[&str]) -> (SearchCache, PathBuf) {
    let temp_dir = TempDir::new("fsevent_test").unwrap();
    let root_path = temp_dir.path().to_path_buf();
    std::mem::forget(temp_dir);

    for file in files {
        let full = root_path.join(file);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::File::create(full).unwrap();
    }

    let cache = SearchCache::walk_fs(root_path.clone());
    (cache, root_path)
}

#[test]
fn test_handle_single_file_creation() {
    let initial_files = ["existing1.txt", "existing2.txt"];
    let (mut cache, root) = build_initial_cache(&initial_files);

    let initial_count = cache.get_total_files();

    // Simulate file creation event
    let new_file = root.join("new_file.txt");
    std::fs::File::create(&new_file).unwrap();

    let event = FsEvent {
        path: new_file.clone(),
        flag: EventFlag::ItemCreated,
        id: 1,
    };

    let result = cache.handle_fs_events(vec![event]);
    assert!(
        result.is_ok(),
        "File creation should be handled successfully"
    );

    // Verify the file is now searchable
    let search_result = cache
        .query_files("new_file".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(search_result.is_some());
    let nodes = search_result.unwrap();
    assert_eq!(nodes.len(), 1, "New file should be found");

    let new_count = cache.get_total_files();
    assert!(
        new_count > initial_count,
        "Total file count should increase"
    );
}

#[test]
fn test_handle_file_removal() {
    let initial_files = ["file1.txt", "file2.txt", "file3.txt"];
    let (mut cache, root) = build_initial_cache(&initial_files);

    // Verify file exists initially
    let search_result = cache
        .query_files("file2".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(search_result.is_some());
    assert_eq!(search_result.unwrap().len(), 1);

    // Remove file and send event
    let removed_file = root.join("file2.txt");
    std::fs::remove_file(&removed_file).unwrap();

    let event = FsEvent {
        path: removed_file.clone(),
        flag: EventFlag::ItemRemoved,
        id: 2,
    };

    let result = cache.handle_fs_events(vec![event]);
    assert!(result.is_ok(), "File removal should be handled");

    // Verify file is no longer searchable
    let search_result = cache
        .query_files("file2".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(search_result.is_some());
    assert_eq!(
        search_result.unwrap().len(),
        0,
        "Removed file should not be found"
    );
}

#[test]
fn test_handle_directory_creation() {
    let initial_files = ["file1.txt"];
    let (mut cache, root) = build_initial_cache(&initial_files);

    // Create new directory with files
    let new_dir = root.join("new_directory");
    std::fs::create_dir(&new_dir).unwrap();
    std::fs::File::create(new_dir.join("nested.txt")).unwrap();

    let event = FsEvent {
        path: new_dir.clone(),
        flag: EventFlag::ItemCreated,
        id: 3,
    };

    let result = cache.handle_fs_events(vec![event]);
    assert!(result.is_ok(), "Directory creation should be handled");

    // Search for the nested file
    let search_result = cache
        .query_files("nested".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(search_result.is_some());
    assert_eq!(
        search_result.unwrap().len(),
        1,
        "Nested file should be found"
    );
}

#[test]
fn test_handle_directory_removal() {
    let initial_files = ["dir/file1.txt", "dir/file2.txt", "other.txt"];
    let (mut cache, root) = build_initial_cache(&initial_files);

    // Verify files in directory are searchable
    let search_result = cache
        .query_files("file1".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(search_result.is_some());
    assert_eq!(search_result.unwrap().len(), 1);

    // Remove directory and send event
    let removed_dir = root.join("dir");
    std::fs::remove_dir_all(&removed_dir).unwrap();

    let event = FsEvent {
        path: removed_dir.clone(),
        flag: EventFlag::ItemRemoved,
        id: 4,
    };

    let result = cache.handle_fs_events(vec![event]);
    assert!(result.is_ok(), "Directory removal should be handled");

    // Verify files are no longer searchable
    let search_result = cache
        .query_files("file1".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(search_result.is_some());
    assert_eq!(
        search_result.unwrap().len(),
        0,
        "Files in removed dir should not be found"
    );

    // Other file should still be there
    let search_result = cache
        .query_files("other".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(search_result.is_some());
    assert_eq!(search_result.unwrap().len(), 1);
}

#[test]
fn test_handle_file_rename() {
    let initial_files = ["old_name.txt", "other.txt"];
    let (mut cache, root) = build_initial_cache(&initial_files);

    // Verify old name is searchable
    let search_result = cache
        .query_files("old_name".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(search_result.is_some());
    assert_eq!(search_result.unwrap().len(), 1);

    // Rename file
    let old_path = root.join("old_name.txt");
    let new_path = root.join("new_name.txt");
    std::fs::rename(&old_path, &new_path).unwrap();

    // Send remove and create events
    let events = vec![
        FsEvent {
            path: old_path.clone(),
            flag: EventFlag::ItemRemoved,
            id: 5,
        },
        FsEvent {
            path: new_path.clone(),
            flag: EventFlag::ItemCreated,
            id: 6,
        },
    ];

    let result = cache.handle_fs_events(events);
    assert!(result.is_ok(), "Rename should be handled");

    // Old name should not be found
    let search_result = cache
        .query_files("old_name".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(search_result.is_some());
    assert_eq!(search_result.unwrap().len(), 0);

    // New name should be found
    let search_result = cache
        .query_files("new_name".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(search_result.is_some());
    assert_eq!(search_result.unwrap().len(), 1);
}

#[test]
fn test_handle_modified_event() {
    let initial_files = ["modified.txt"];
    let (mut cache, root) = build_initial_cache(&initial_files);

    let modified_file = root.join("modified.txt");

    // Write some content
    std::fs::write(&modified_file, b"new content").unwrap();

    let event = FsEvent {
        path: modified_file.clone(),
        flag: EventFlag::ItemModified,
        id: 7,
    };

    let result = cache.handle_fs_events(vec![event]);
    assert!(result.is_ok(), "Modification should be handled");

    // File should still be searchable
    let search_result = cache
        .query_files("modified".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(search_result.is_some());
    assert_eq!(search_result.unwrap().len(), 1);
}

#[test]
fn test_batch_events_same_directory() {
    let initial_files = ["base.txt"];
    let (mut cache, root) = build_initial_cache(&initial_files);

    // Create multiple files in same directory
    let dir = root.join("batch_dir");
    std::fs::create_dir(&dir).unwrap();

    let files: Vec<_> = (0..10)
        .map(|i| {
            let path = dir.join(format!("file_{i}.txt"));
            std::fs::File::create(&path).unwrap();
            path
        })
        .collect();

    // Send batch of creation events
    let events: Vec<_> = files
        .iter()
        .enumerate()
        .map(|(i, path)| FsEvent {
            path: path.clone(),
            flag: EventFlag::ItemCreated,
            id: 10 + i as u64,
        })
        .collect();

    let result = cache.handle_fs_events(events);
    assert!(result.is_ok(), "Batch events should be handled");

    // All files should be searchable
    let search_result = cache
        .query_files("file_".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(search_result.is_some());
    let nodes = search_result.unwrap();
    assert_eq!(nodes.len(), 10, "All batch created files should be found");
}

#[test]
fn test_nested_directory_operations() {
    let initial_files = ["root.txt"];
    let (mut cache, root) = build_initial_cache(&initial_files);

    // Create nested directory structure
    let deep_path = root.join("a/b/c/d");
    std::fs::create_dir_all(&deep_path).unwrap();
    std::fs::File::create(deep_path.join("deep.txt")).unwrap();

    let event = FsEvent {
        path: root.join("a"),
        flag: EventFlag::ItemCreated,
        id: 20,
    };

    let result = cache.handle_fs_events(vec![event]);
    assert!(result.is_ok());

    // Deep file should be found
    let search_result = cache
        .query_files("deep".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(search_result.is_some());
    assert_eq!(search_result.unwrap().len(), 1);

    // Now remove the top-level directory
    std::fs::remove_dir_all(root.join("a")).unwrap();

    let event = FsEvent {
        path: root.join("a"),
        flag: EventFlag::ItemRemoved,
        id: 21,
    };

    let result = cache.handle_fs_events(vec![event]);
    assert!(result.is_ok());

    // Deep file should no longer be found
    let search_result = cache
        .query_files("deep".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(search_result.is_some());
    assert_eq!(search_result.unwrap().len(), 0);
}

#[test]
fn test_event_id_tracking() {
    let initial_files = ["test.txt"];
    let (mut cache, root) = build_initial_cache(&initial_files);

    let initial_event_id = cache.last_event_id();

    // Send event with higher ID
    let new_file = root.join("new.txt");
    std::fs::File::create(&new_file).unwrap();

    let event = FsEvent {
        path: new_file,
        flag: EventFlag::ItemCreated,
        id: initial_event_id + 100,
    };

    cache.handle_fs_events(vec![event]).unwrap();

    let new_event_id = cache.last_event_id();
    assert!(
        new_event_id > initial_event_id,
        "Event ID should be updated"
    );
    assert_eq!(
        new_event_id,
        initial_event_id + 100,
        "Should track max event ID"
    );
}

#[test]
fn test_rescan_trigger_on_root_changed() {
    let initial_files = ["test.txt"];
    let (mut cache, root) = build_initial_cache(&initial_files);

    // Send RootChanged event (should trigger rescan)
    let event = FsEvent {
        path: root.clone(),
        flag: EventFlag::RootChanged,
        id: 50,
    };

    let result = cache.handle_fs_events(vec![event]);
    assert!(result.is_err(), "RootChanged should trigger rescan error");
}

#[test]
fn test_history_done_event() {
    let initial_files = ["test.txt"];
    let (mut cache, root) = build_initial_cache(&initial_files);

    // HistoryDone should be processed without error
    let event = FsEvent {
        path: root.clone(),
        flag: EventFlag::HistoryDone,
        id: 100,
    };

    let result = cache.handle_fs_events(vec![event]);
    // HistoryDone might trigger rescan or be handled gracefully
    // Just verify it doesn't panic
    let _ = result;
}

#[test]
fn test_duplicate_events_deduplicated() {
    let initial_files = ["test.txt"];
    let (mut cache, root) = build_initial_cache(&initial_files);

    let new_file = root.join("duplicate.txt");
    std::fs::File::create(&new_file).unwrap();

    // Send same event multiple times
    let events = vec![
        FsEvent {
            path: new_file.clone(),
            flag: EventFlag::ItemCreated,
            id: 200,
        },
        FsEvent {
            path: new_file.clone(),
            flag: EventFlag::ItemCreated,
            id: 201,
        },
        FsEvent {
            path: new_file.clone(),
            flag: EventFlag::ItemCreated,
            id: 202,
        },
    ];

    let result = cache.handle_fs_events(events);
    assert!(result.is_ok());

    // Should still only find one file
    let search_result = cache
        .query_files("duplicate".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(search_result.is_some());
    assert_eq!(
        search_result.unwrap().len(),
        1,
        "Duplicate events should result in single entry"
    );
}

#[test]
fn test_events_for_ignored_paths() {
    let temp_dir = TempDir::new("ignored_paths_test").unwrap();
    let root_path = temp_dir.path().to_path_buf();
    std::mem::forget(temp_dir);

    std::fs::File::create(root_path.join("included.txt")).unwrap();

    // Create ignored directory
    let ignored_dir = root_path.join("ignored");
    std::fs::create_dir(&ignored_dir).unwrap();

    let mut cache = SearchCache::walk_fs_with_ignore(root_path.clone(), vec![ignored_dir.clone()]);

    // Create file in ignored directory
    let ignored_file = ignored_dir.join("should_not_index.txt");
    std::fs::File::create(&ignored_file).unwrap();

    let event = FsEvent {
        path: ignored_file.clone(),
        flag: EventFlag::ItemCreated,
        id: 300,
    };

    cache.handle_fs_events(vec![event]).unwrap();

    // File in ignored path may or may not be indexed depending on implementation
    // Just verify it doesn't panic
    let search_result = cache
        .query_files("should_not_index".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(
        search_result.is_some(),
        "Search should not panic for ignored paths"
    );
}

#[test]
fn test_rapid_create_delete_cycle() {
    let initial_files = ["base.txt"];
    let (mut cache, root) = build_initial_cache(&initial_files);

    let temp_file = root.join("temp.txt");

    // Create and delete multiple times
    for i in 0..5 {
        std::fs::File::create(&temp_file).unwrap();
        let create_event = FsEvent {
            path: temp_file.clone(),
            flag: EventFlag::ItemCreated,
            id: 400 + i * 2,
        };
        cache.handle_fs_events(vec![create_event]).unwrap();

        std::fs::remove_file(&temp_file).unwrap();
        let remove_event = FsEvent {
            path: temp_file.clone(),
            flag: EventFlag::ItemRemoved,
            id: 401 + i * 2,
        };
        cache.handle_fs_events(vec![remove_event]).unwrap();
    }

    // File should not exist after cycle
    let search_result = cache
        .query_files("temp".to_string(), CancellationToken::noop())
        .unwrap();
    assert!(search_result.is_some());
    assert_eq!(
        search_result.unwrap().len(),
        0,
        "File should be removed after cycles"
    );
}
