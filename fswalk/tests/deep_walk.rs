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

// ── should_ignore prefix-matching integration tests ─────────────────────

/// Ignoring a directory also excludes all of its nested descendants.
#[test]
fn ignore_prefix_excludes_nested_children() {
    let tmp = TempDir::new("fswalk_prefix").unwrap();
    let root = tmp.path();

    // /root/skip_dir/sub/deep/file.txt
    // /root/keep.txt
    fs::create_dir_all(root.join("skip_dir/sub/deep")).unwrap();
    fs::write(root.join("skip_dir/sub/deep/file.txt"), b"x").unwrap();
    fs::write(root.join("skip_dir/top.txt"), b"y").unwrap();
    fs::write(root.join("keep.txt"), b"k").unwrap();

    let ignore = vec![root.join("skip_dir")];
    let walk_data = WalkData::new(root, &ignore, false, || false);
    let tree = walk_it(&walk_data).expect("root node");
    let tree = node_for_path(&tree, root);

    // skip_dir should be completely absent
    assert!(
        !tree.children.iter().any(|c| &*c.name == "skip_dir"),
        "skip_dir and all descendants should be excluded"
    );
    // keep.txt should be present
    assert!(tree.children.iter().any(|c| &*c.name == "keep.txt"));
}

/// A sibling directory whose name starts with the same characters as the
/// ignored directory must NOT be ignored (component-aware prefix, not string).
#[test]
fn ignore_does_not_affect_sibling_with_similar_name() {
    let tmp = TempDir::new("fswalk_similar_name").unwrap();
    let root = tmp.path();

    // /root/node_modules/          <- to be ignored
    // /root/node_modules_backup/   <- should NOT be ignored
    fs::create_dir(root.join("node_modules")).unwrap();
    fs::write(root.join("node_modules/pkg.json"), b"{}").unwrap();
    fs::create_dir(root.join("node_modules_backup")).unwrap();
    fs::write(root.join("node_modules_backup/pkg.json"), b"{}").unwrap();

    let ignore = vec![root.join("node_modules")];
    let walk_data = WalkData::new(root, &ignore, false, || false);
    let tree = walk_it(&walk_data).expect("root node");
    let tree = node_for_path(&tree, root);

    assert!(
        !tree.children.iter().any(|c| &*c.name == "node_modules"),
        "node_modules should be ignored"
    );
    let backup = tree
        .children
        .iter()
        .find(|c| &*c.name == "node_modules_backup")
        .expect("node_modules_backup must survive");
    assert!(
        backup.children.iter().any(|c| &*c.name == "pkg.json"),
        "sibling with shared prefix string should keep its children"
    );
}

/// Ignoring an intermediate directory preserves ancestors and other siblings.
#[test]
fn ignore_intermediate_dir_preserves_siblings() {
    let tmp = TempDir::new("fswalk_mid").unwrap();
    let root = tmp.path();

    // /root/parent/ignore_me/file.txt
    // /root/parent/keep_me/file.txt
    fs::create_dir_all(root.join("parent/ignore_me")).unwrap();
    fs::create_dir_all(root.join("parent/keep_me")).unwrap();
    fs::write(root.join("parent/ignore_me/file.txt"), b"i").unwrap();
    fs::write(root.join("parent/keep_me/file.txt"), b"k").unwrap();

    let ignore = vec![root.join("parent/ignore_me")];
    let walk_data = WalkData::new(root, &ignore, false, || false);
    let tree = walk_it(&walk_data).expect("root node");
    let tree = node_for_path(&tree, root);

    let parent = tree
        .children
        .iter()
        .find(|c| &*c.name == "parent")
        .expect("parent must survive");
    assert!(
        !parent.children.iter().any(|c| &*c.name == "ignore_me"),
        "ignore_me should be excluded"
    );
    let keep = parent
        .children
        .iter()
        .find(|c| &*c.name == "keep_me")
        .expect("keep_me should survive");
    assert!(keep.children.iter().any(|c| &*c.name == "file.txt"));
}

/// Multiple ignore paths with prefix semantics work together.
#[test]
fn multiple_ignores_with_prefix() {
    let tmp = TempDir::new("fswalk_multi_ig").unwrap();
    let root = tmp.path();

    fs::create_dir_all(root.join("a/deep/child")).unwrap();
    fs::create_dir_all(root.join("b/deep/child")).unwrap();
    fs::create_dir(root.join("c")).unwrap();
    fs::write(root.join("a/deep/child/f.txt"), b"").unwrap();
    fs::write(root.join("b/deep/child/f.txt"), b"").unwrap();
    fs::write(root.join("c/f.txt"), b"").unwrap();

    let ignore = vec![root.join("a"), root.join("b")];
    let walk_data = WalkData::new(root, &ignore, false, || false);
    let tree = walk_it(&walk_data).expect("root node");
    let tree = node_for_path(&tree, root);

    let names: Vec<&str> = tree.children.iter().map(|c| &*c.name).collect();
    assert!(!names.contains(&"a"), "a should be ignored");
    assert!(!names.contains(&"b"), "b should be ignored");
    assert!(names.contains(&"c"), "c should remain");
}

/// File counts reflect the prefix-based ignore (descendants not counted).
#[test]
fn file_counts_exclude_ignored_subtree() {
    let tmp = TempDir::new("fswalk_counts").unwrap();
    let root = tmp.path();

    // 3 files under ignored dir, 2 under kept dir
    fs::create_dir_all(root.join("ignored/sub")).unwrap();
    fs::write(root.join("ignored/a.txt"), b"").unwrap();
    fs::write(root.join("ignored/b.txt"), b"").unwrap();
    fs::write(root.join("ignored/sub/c.txt"), b"").unwrap();

    fs::create_dir(root.join("kept")).unwrap();
    fs::write(root.join("kept/d.txt"), b"").unwrap();
    fs::write(root.join("kept/e.txt"), b"").unwrap();

    let ignore = vec![root.join("ignored")];
    let walk_data = WalkData::new(root, &ignore, false, || false);
    let _tree = walk_it(&walk_data).expect("root node");

    let num_files = walk_data.num_files.load(Ordering::Relaxed);
    assert_eq!(
        num_files, 2,
        "only the 2 files under kept/ should be counted, got {num_files}"
    );
}

/// `walk_it_without_root_chain` returns `None` when the cancel closure is
/// already true before traversal starts.
#[test]
fn cancellation_stops_walk_it_without_root_chain() {
    use fswalk::walk_it_without_root_chain;

    let tmp = TempDir::new("fswalk_cancel_noroot").unwrap();
    for i in 0..20 {
        fs::create_dir(tmp.path().join(format!("dir_{i}"))).unwrap();
        fs::write(tmp.path().join(format!("dir_{i}/f.txt")), b"").unwrap();
    }

    let cancel = AtomicBool::new(true); // already cancelled
    let walk_data = WalkData::new(tmp.path(), &[], false, || cancel.load(Ordering::Relaxed));
    let result = walk_it_without_root_chain(&walk_data);
    assert!(
        result.is_none(),
        "walk_it_without_root_chain must return None when cancel closure returns true"
    );
}

/// `walk_it_without_root_chain` also respects prefix-based ignore.
#[test]
fn walk_without_root_chain_respects_prefix_ignore() {
    use fswalk::walk_it_without_root_chain;

    let tmp = TempDir::new("fswalk_noroot").unwrap();
    let root = tmp.path();

    fs::create_dir_all(root.join("skip/nested")).unwrap();
    fs::write(root.join("skip/nested/f.txt"), b"").unwrap();
    fs::write(root.join("stay.txt"), b"").unwrap();

    let ignore = vec![root.join("skip")];
    let walk_data = WalkData::new(root, &ignore, false, || false);
    let tree = walk_it_without_root_chain(&walk_data).expect("root node");

    assert!(
        !tree.children.iter().any(|c| &*c.name == "skip"),
        "skip and descendants should be excluded"
    );
    assert!(tree.children.iter().any(|c| &*c.name == "stay.txt"));
}
