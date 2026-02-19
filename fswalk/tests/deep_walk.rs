use fswalk::{NodeFileType, WalkData, walk_it};
use std::{
    fs,
    path::{Component, Path},
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

fn node_for_path<'a>(node: &'a fswalk::Node, path: &Path) -> &'a fswalk::Node {
    let mut current = node;
    for component in path.components() {
        match component {
            Component::RootDir => {
                assert_eq!(&*current.name, "/");
            }
            Component::Normal(name) => {
                let name = name.to_string_lossy();
                current = current
                    .children
                    .iter()
                    .find(|child| *child.name == name)
                    .unwrap_or_else(|| panic!("missing path segment: {name}"));
            }
            _ => {}
        }
    }
    current
}

#[test]
fn ignores_directories_and_collects_metadata() {
    let tmp = TempDir::new("fswalk_deep").unwrap();
    build_deep_fixture(tmp.path());
    let ignore = vec![tmp.path().join("skip_dir")];
    let walk_data = WalkData::new(tmp.path(), &ignore, true, || false);
    let tree = walk_it(&walk_data).expect("root node");
    let tree = node_for_path(&tree, tmp.path());

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
    assert_meta(tree);
}

#[test]
fn cancellation_stops_traversal_early() {
    let tmp = TempDir::new("fswalk_cancel").unwrap();
    // Build many subdirectories so traversal would take longer
    for i in 0..30 {
        fs::create_dir(tmp.path().join(format!("dir_{i}"))).unwrap();
    }
    let cancel = AtomicBool::new(false);
    let walk_data = WalkData::new(tmp.path(), &[], false, || cancel.load(Ordering::Relaxed));
    cancel.store(true, Ordering::Relaxed); // cancel immediately
    let node = walk_it(&walk_data);
    assert!(
        node.is_none(),
        "expected immediate cancellation to abort traversal"
    );
}
