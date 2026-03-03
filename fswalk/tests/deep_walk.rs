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
    let ignore = vec![std::path::PathBuf::from("/skip_dir")];
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

#[test]
fn glob_patterns_ignore_nested_directories() {
    let tmp = TempDir::new("fswalk_glob_ignore").unwrap();
    let root = tmp.path();

    fs::create_dir_all(root.join("packages/app/node_modules/pkg")).unwrap();
    fs::create_dir_all(root.join("src/components")).unwrap();
    fs::write(
        root.join("packages/app/node_modules/pkg/ignored.js"),
        b"console.log('ignored');",
    )
    .unwrap();
    fs::write(
        root.join("src/components/kept.tsx"),
        b"export const kept = true;",
    )
    .unwrap();

    let ignore = vec![std::path::PathBuf::from("**/node_modules/**")];
    let walk_data = WalkData::new(root, &ignore, true, || false);
    let tree = walk_it(&walk_data).expect("root node");
    let tree = node_for_path(&tree, root);

    let packages = tree
        .children
        .iter()
        .find(|child| &*child.name == "packages")
        .expect("packages directory");
    let app = packages
        .children
        .iter()
        .find(|child| &*child.name == "app")
        .expect("app directory");
    let node_modules = app
        .children
        .iter()
        .find(|child| &*child.name == "node_modules")
        .expect("node_modules directory should still exist");
    assert!(
        node_modules.children.is_empty(),
        "glob ignore should remove descendants under node_modules"
    );

    let src = tree
        .children
        .iter()
        .find(|child| &*child.name == "src")
        .expect("src directory");
    let components = src
        .children
        .iter()
        .find(|child| &*child.name == "components")
        .expect("components directory");
    assert!(
        components
            .children
            .iter()
            .any(|child| &*child.name == "kept.tsx"),
        "non-matching files should remain indexed"
    );
}

#[test]
fn globstar_descendant_pattern_does_not_ignore_parent_directory() {
    let tmp = TempDir::new("fswalk_globstar_parent_semantics").unwrap();
    let root = tmp.path();

    fs::create_dir_all(root.join("Xcode.app/Contents")).unwrap();
    fs::write(root.join("Xcode.app/Contents/data.bin"), b"data").unwrap();

    let ignore = vec![std::path::PathBuf::from("**/Xcode.app/**")];
    let walk_data = WalkData::new(root, &ignore, true, || false);
    let tree = walk_it(&walk_data).expect("root node");
    let tree = node_for_path(&tree, root);

    let xcode_app = tree
        .children
        .iter()
        .find(|child| &*child.name == "Xcode.app")
        .expect("Xcode.app should not be ignored by **/Xcode.app/** itself");
    assert!(
        xcode_app.children.is_empty(),
        "descendants under Xcode.app should be ignored by **/Xcode.app/**"
    );
}

#[test]
fn gitignore_negation_reincludes_file_when_parent_is_not_pruned() {
    let tmp = TempDir::new("fswalk_gitignore_negation_reinclude").unwrap();
    let root = tmp.path();

    fs::create_dir_all(root.join("workspace/node_modules")).unwrap();
    fs::write(root.join("workspace/node_modules/included.js"), b"ok").unwrap();
    fs::write(root.join("workspace/node_modules/ignored.js"), b"no").unwrap();

    let ignore = vec![
        std::path::PathBuf::from("**/node_modules/**"),
        std::path::PathBuf::from("!**/node_modules/**/included.js"),
    ];
    let walk_data = WalkData::new(root, &ignore, true, || false);
    let tree = walk_it(&walk_data).expect("root node");
    let tree = node_for_path(&tree, root);

    let workspace = tree
        .children
        .iter()
        .find(|child| &*child.name == "workspace")
        .expect("workspace directory");
    let node_modules = workspace
        .children
        .iter()
        .find(|child| &*child.name == "node_modules")
        .expect("node_modules directory");

    assert!(
        node_modules
            .children
            .iter()
            .any(|child| &*child.name == "included.js"),
        "negation should re-include explicit file when parent directory is not pruned"
    );
    assert!(
        node_modules
            .children
            .iter()
            .all(|child| &*child.name != "ignored.js"),
        "non-negated files should remain ignored"
    );
}

#[test]
fn gitignore_negation_cannot_reinclude_when_parent_directory_is_pruned() {
    let tmp = TempDir::new("fswalk_gitignore_negation_pruned_parent").unwrap();
    let root = tmp.path();

    fs::create_dir_all(root.join("workspace/node_modules")).unwrap();
    fs::write(root.join("workspace/node_modules/included.js"), b"ok").unwrap();

    let ignore = vec![
        std::path::PathBuf::from("**/node_modules/"),
        std::path::PathBuf::from("!**/node_modules/**/included.js"),
    ];
    let walk_data = WalkData::new(root, &ignore, true, || false);
    let tree = walk_it(&walk_data).expect("root node");
    let tree = node_for_path(&tree, root);

    let workspace = tree
        .children
        .iter()
        .find(|child| &*child.name == "workspace")
        .expect("workspace directory");
    assert!(
        workspace
            .children
            .iter()
            .all(|child| &*child.name != "node_modules"),
        "when a parent directory is ignored, its subtree is pruned and negation cannot re-include descendants"
    );
}
