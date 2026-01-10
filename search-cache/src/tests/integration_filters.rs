use super::prelude::*;

#[test]
fn test_combined_filters_all_match() {
    let tmp = TempDir::new("combined_all_match").unwrap();
    fs::write(tmp.path().join("report_large.pdf"), vec![0u8; 100_000]).unwrap();
    fs::write(tmp.path().join("report_small.pdf"), vec![0u8; 1_000]).unwrap();
    fs::write(tmp.path().join("data.csv"), vec![0u8; 100_000]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    let results = cache.search("report type:pdf size:>10kb").unwrap();
    assert_eq!(results.len(), 1);

    let path = cache.node_path(*results.first().unwrap()).unwrap();
    assert!(path.ends_with("report_large.pdf"));
}

#[test]
fn test_complex_boolean_with_filters() {
    let tmp = TempDir::new("complex_boolean").unwrap();
    fs::write(tmp.path().join("report.pdf"), vec![0u8; 100_000]).unwrap();
    fs::write(tmp.path().join("report.txt"), vec![0u8; 1_000]).unwrap();
    fs::write(tmp.path().join("image.jpg"), vec![0u8; 100_000]).unwrap();
    fs::write(tmp.path().join("small_image.jpg"), vec![0u8; 1_000]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    let results = cache
        .search("(report OR type:picture) size:>10kb !txt")
        .unwrap();
    assert_eq!(results.len(), 2);

    let paths: Vec<_> = results
        .iter()
        .map(|i| cache.node_path(*i).unwrap())
        .collect();
    assert!(paths.iter().any(|p| p.ends_with("report.pdf")));
    assert!(paths.iter().any(|p| p.ends_with("image.jpg")));
}

#[test]
fn test_combined_filters_no_matches() {
    let tmp = TempDir::new("combined_no_match").unwrap();
    fs::write(tmp.path().join("small.jpg"), vec![0u8; 100]).unwrap();
    fs::write(tmp.path().join("large.txt"), vec![0u8; 100_000]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    // Picture that is large (but our picture is small)
    let results = cache.search("type:picture size:>10kb").unwrap();
    assert_eq!(results.len(), 0);
}

#[test]
fn test_complex_query_with_precedence() {
    let tmp = TempDir::new("complex_precedence").unwrap();
    fs::write(tmp.path().join("report_a.pdf"), vec![0u8; 100_000]).unwrap();
    fs::write(tmp.path().join("report_b.txt"), vec![0u8; 100_000]).unwrap();
    fs::write(tmp.path().join("photo_a.jpg"), vec![0u8; 100_000]).unwrap();
    fs::write(tmp.path().join("photo_b.jpg"), vec![0u8; 1_000]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    // Test: (report OR photo_a) AND type:picture
    let results = cache.search("report OR photo_a type:picture").unwrap();
    assert_eq!(results.len(), 1);
    let path = cache.node_path(*results.first().unwrap()).unwrap();
    assert!(path.ends_with("photo_a.jpg"));
}

#[test]
fn test_multiple_filters_intersection_complex() {
    let tmp = TempDir::new("multi_filter_complex").unwrap();
    fs::create_dir(tmp.path().join("photos")).unwrap();
    fs::write(tmp.path().join("photos/vacation.jpg"), vec![0u8; 100_000]).unwrap();
    fs::write(tmp.path().join("photos/small.jpg"), vec![0u8; 1_000]).unwrap();
    fs::write(tmp.path().join("document.pdf"), vec![0u8; 100_000]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    let photos_dir = tmp.path().join("photos");
    let results = cache
        .search(&format!(
            "type:picture size:>10kb parent:{}",
            photos_dir.display()
        ))
        .unwrap();
    assert_eq!(results.len(), 1);
}

#[test]
fn test_deeply_nested_boolean_with_filters() {
    let tmp = TempDir::new("deep_boolean").unwrap();
    fs::write(tmp.path().join("a.jpg"), vec![0u8; 100_000]).unwrap();
    fs::write(tmp.path().join("b.jpg"), vec![0u8; 1_000]).unwrap();
    fs::write(tmp.path().join("c.mp3"), vec![0u8; 100_000]).unwrap();
    fs::write(tmp.path().join("d.mp3"), vec![0u8; 1_000]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    let results = cache
        .search("((type:picture OR type:audio) size:>10kb)")
        .unwrap();
    assert_eq!(results.len(), 2);
}

#[test]
fn test_complex_or_chain_with_types() {
    let tmp = TempDir::new("or_chain_types").unwrap();
    fs::write(tmp.path().join("image.jpg"), b"x").unwrap();
    fs::write(tmp.path().join("video.mp4"), b"x").unwrap();
    fs::write(tmp.path().join("audio.mp3"), b"x").unwrap();
    fs::write(tmp.path().join("doc.pdf"), b"x").unwrap();
    fs::write(tmp.path().join("archive.zip"), b"x").unwrap();
    fs::write(tmp.path().join("code.rs"), b"x").unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    let results = cache
        .search("type:picture OR type:video OR type:audio OR type:doc")
        .unwrap();
    assert_eq!(results.len(), 4);
}

#[test]
fn test_combined_filters_empty_intersection() {
    let tmp = TempDir::new("empty_intersection").unwrap();
    fs::write(tmp.path().join("photo.jpg"), vec![0u8; 100]).unwrap();
    fs::write(tmp.path().join("large.txt"), vec![0u8; 100_000]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    // Looking for large pictures, but the picture is small
    let results = cache.search("type:picture size:>10kb").unwrap();
    assert_eq!(results.len(), 0);

    // Looking for small documents, but the document is large
    let results2 = cache.search("type:doc size:<1kb").unwrap();
    assert_eq!(results2.len(), 0);
}

#[test]
fn test_complex_real_world_query() {
    let tmp = TempDir::new("real_world").unwrap();
    fs::write(
        tmp.path().join("vacation_photo_2024.jpg"),
        vec![0u8; 500_000],
    )
    .unwrap();
    fs::write(tmp.path().join("family_photo_2024.jpg"), vec![0u8; 1_000]).unwrap();
    fs::write(
        tmp.path().join("vacation_video_2024.mp4"),
        vec![0u8; 500_000],
    )
    .unwrap();
    fs::write(tmp.path().join("old_photo_2023.jpg"), vec![0u8; 500_000]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    // Find large vacation photos from 2024
    let results = cache
        .search("vacation 2024 type:picture size:>100kb")
        .unwrap();
    assert_eq!(results.len(), 1);

    let path = cache.node_path(*results.first().unwrap()).unwrap();
    assert!(path.ends_with("vacation_photo_2024.jpg"));
}

#[test]
fn test_combined_all_filter_types() {
    let tmp = TempDir::new("all_filters").unwrap();
    fs::create_dir(tmp.path().join("photos")).unwrap();
    fs::write(tmp.path().join("photos/vacation.jpg"), vec![0u8; 100_000]).unwrap();
    fs::write(tmp.path().join("photos/small.jpg"), vec![0u8; 1_000]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    let photos_dir = tmp.path().join("photos");
    let results = cache
        .search(&format!(
            "vacation type:picture size:>10kb ext:jpg parent:{}",
            photos_dir.display()
        ))
        .unwrap();
    assert_eq!(results.len(), 1);
}

#[test]
fn test_stress_many_filters_combined() {
    let tmp = TempDir::new("stress_many_filters").unwrap();
    fs::create_dir(tmp.path().join("media")).unwrap();
    fs::write(
        tmp.path().join("media/vacation_2024_photo.jpg"),
        vec![0u8; 500_000],
    )
    .unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    let media_dir = tmp.path().join("media");
    let results = cache
        .search(&format!(
            "vacation 2024 photo type:picture size:>100kb ext:jpg parent:{}",
            media_dir.display()
        ))
        .unwrap();
    assert_eq!(results.len(), 1);
}

#[test]
fn test_final_integration_comprehensive() {
    let tmp = TempDir::new("final_integration").unwrap();
    // Create a realistic file structure
    fs::create_dir_all(tmp.path().join("Documents/Reports")).unwrap();
    fs::create_dir_all(tmp.path().join("Media/Photos")).unwrap();
    fs::create_dir_all(tmp.path().join("Media/Videos")).unwrap();
    fs::create_dir(tmp.path().join("Code")).unwrap();

    fs::write(
        tmp.path().join("Documents/Reports/Q4_Report.pdf"),
        vec![0u8; 1_000_000],
    )
    .unwrap();
    fs::write(tmp.path().join("Documents/Notes.txt"), vec![0u8; 5_000]).unwrap();
    fs::write(
        tmp.path().join("Media/Photos/vacation.jpg"),
        vec![0u8; 500_000],
    )
    .unwrap();
    fs::write(
        tmp.path().join("Media/Videos/clip.mp4"),
        vec![0u8; 5_000_000],
    )
    .unwrap();
    fs::write(tmp.path().join("Code/main.rs"), vec![0u8; 10_000]).unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    // Test 1: Find large documents
    let docs = cache.search("type:doc size:>100kb").unwrap();
    assert_eq!(docs.len(), 1);

    // Test 2: Find media files
    let media = cache.search("type:picture OR type:video").unwrap();
    assert_eq!(media.len(), 2);

    // Test 3: Find code files
    let code = cache.search("type:code").unwrap();
    assert_eq!(code.len(), 1);

    // Test 4: Complex query
    let results = cache.search("vacation type:picture size:>100kb").unwrap();
    assert_eq!(results.len(), 1);
}

#[test]
fn test_nosubfolders_keeps_only_direct_files() {
    let tmp = TempDir::new("nosubfolders_direct_files").unwrap();
    let projects = tmp.path().join("Projects");
    fs::create_dir(&projects).unwrap();
    fs::write(projects.join("root.txt"), b"root").unwrap();
    fs::create_dir(projects.join("Nested")).unwrap();
    fs::write(projects.join("Nested/deep.txt"), b"deep").unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());
    let results = cache
        .search(&format!("nosubfolders:{}", projects.display()))
        .unwrap();

    let paths: Vec<_> = results
        .iter()
        .filter_map(|idx| cache.node_path(*idx))
        .collect();
    assert_eq!(paths.len(), 1);
    assert!(paths.iter().any(|path| path.ends_with("root.txt")));
    assert!(paths.iter().all(|path| !path.ends_with("Nested")));
    assert!(paths.iter().all(|path| !path.ends_with("deep.txt")));
}

#[test]
fn test_nosubfolders_only_filters_target_tree() {
    let tmp = TempDir::new("nosubfolders_intersection").unwrap();
    let projects = tmp.path().join("Projects");
    fs::create_dir(&projects).unwrap();
    fs::write(projects.join("report.txt"), b"root report").unwrap();
    fs::create_dir(projects.join("Nested")).unwrap();
    fs::write(projects.join("Nested/report.txt"), b"deep report").unwrap();
    fs::write(tmp.path().join("report.txt"), b"global report").unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());
    let query = format!("report nosubfolders:{}", projects.display());
    let paths: Vec<_> = cache
        .search(&query)
        .unwrap()
        .into_iter()
        .filter_map(|idx| cache.node_path(idx))
        .collect();

    assert_eq!(paths.len(), 1);
    assert_eq!(paths[0], projects.join("report.txt"));
}
