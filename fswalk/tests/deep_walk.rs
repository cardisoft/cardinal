use fswalk::{NodeFileType, WalkData, walk_it};
use std::{
    fs,
    sync::atomic::{AtomicBool, Ordering},
};
use tempdir::TempDir;

fn build_deep_fixture(root: &std::path::Path) {
    // /root
    //   /skip_dir
    //      skip_a.txt
    //   /keep_dir
    //      /nested
    //         deep.txt
    //   keep_a.txt
    //   keep_b.log
    fs::create_dir(root.join("skip_dir")).unwrap();
    fs::create_dir(root.join("keep_dir")).unwrap();
    fs::create_dir(root.join("keep_dir/nested")).unwrap();
    fs::write(root.join("skip_dir/skip_a.txt"), b"s").unwrap();
    fs::write(root.join("keep_dir/nested/deep.txt"), b"d").unwrap();
    fs::write(root.join("keep_a.txt"), b"a").unwrap();
    fs::write(root.join("keep_b.log"), b"b").unwrap();
}

#[test]
fn ignores_directories_and_collects_metadata() {
    let tmp = TempDir::new("fswalk_deep").unwrap();
    build_deep_fixture(tmp.path());
    let ignore = vec![tmp.path().join("skip_dir")];
    let walk_data = WalkData::new(tmp.path(), &ignore, true, None);
    let tree = walk_it(&walk_data).expect("root node");

    // Ensure skip_dir absent
    assert!(!tree.children.iter().any(|c| &*c.name == "skip_dir"));
    // Ensure keep_dir present with nested/deep.txt
    let keep_dir = tree
        .children
        .iter()
        .find(|c| &*c.name == "keep_dir")
        .expect("keep_dir");
    let nested = keep_dir
        .children
        .iter()
        .find(|c| &*c.name == "nested")
        .expect("nested");
    assert!(nested.children.iter().any(|c| &*c.name == "deep.txt"));

    // Metadata existence for files (requested) and types correct
    fn assert_meta(node: &fswalk::Node) {
        if node.children.is_empty() {
            let m = node.metadata.expect("file metadata should be present");
            assert!(matches!(m.r#type, NodeFileType::File));
        } else {
            if let Some(m) = node.metadata {
                assert!(matches!(m.r#type, NodeFileType::Dir));
            }
            for ch in &node.children {
                assert_meta(ch);
            }
        }
    }
    assert_meta(&tree);
}

#[test]
fn cancellation_stops_traversal_early() {
    let tmp = TempDir::new("fswalk_cancel").unwrap();
    // Build many subdirectories so traversal would take longer
    for i in 0..30 {
        fs::create_dir(tmp.path().join(format!("dir_{i}"))).unwrap();
    }
    let cancel = AtomicBool::new(false);
    let walk_data = WalkData::new(tmp.path(), &[], false, Some(&cancel));
    cancel.store(true, Ordering::Relaxed); // cancel immediately
    let node = walk_it(&walk_data);
    assert!(
        node.is_none(),
        "expected immediate cancellation to abort traversal"
    );
}
