use rayon::{iter::ParallelBridge, prelude::ParallelIterator};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::{
    fs::{self, Metadata},
    io::{Error, ErrorKind},
    num::NonZeroU64,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
    time::UNIX_EPOCH,
};

#[derive(Serialize, Debug)]
pub struct Node {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<Node>,
    pub name: Box<str>,
    pub metadata: Option<NodeMetadata>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct NodeMetadata {
    pub r#type: NodeFileType,
    pub size: u64,
    pub ctime: Option<NonZeroU64>,
    pub mtime: Option<NonZeroU64>,
}

impl From<Metadata> for NodeMetadata {
    fn from(metadata: Metadata) -> Self {
        Self::new(&metadata)
    }
}

impl NodeMetadata {
    fn new(metadata: &Metadata) -> Self {
        let r#type = metadata.file_type().into();
        let size = metadata.size();
        let ctime = metadata
            .created()
            .ok()
            .and_then(|x| x.duration_since(UNIX_EPOCH).ok())
            .and_then(|x| NonZeroU64::new(x.as_secs()));
        let mtime = metadata
            .modified()
            .ok()
            .and_then(|x| x.duration_since(UNIX_EPOCH).ok())
            .and_then(|x| NonZeroU64::new(x.as_secs()));
        Self {
            r#type,
            size,
            ctime,
            mtime,
        }
    }
}

#[derive(Debug, Serialize_repr, Deserialize_repr, Clone, Copy, enumn::N, PartialEq, Eq)]
#[repr(u8)]
pub enum NodeFileType {
    // File occurs a lot, assign it to 0 for better compression ratio(I guess... maybe useful).
    File = 0,
    Dir = 1,
    Symlink = 2,
    Unknown = 3,
}

impl From<fs::FileType> for NodeFileType {
    fn from(file_type: fs::FileType) -> Self {
        if file_type.is_file() {
            NodeFileType::File
        } else if file_type.is_dir() {
            NodeFileType::Dir
        } else if file_type.is_symlink() {
            NodeFileType::Symlink
        } else {
            NodeFileType::Unknown
        }
    }
}

#[derive(Debug)]
pub struct WalkData<'w> {
    pub num_files: AtomicUsize,
    pub num_dirs: AtomicUsize,
    /// Cancellation will be checked periodically.
    cancel: Option<&'w AtomicBool>,
    pub root_path: &'w Path,
    pub ignore_directories: &'w [PathBuf],
    /// If set, metadata will be collected for each file node(folder node will get free metadata).
    need_metadata: bool,
}

impl<'w> WalkData<'w> {
    pub const fn simple(root_path: &'w Path, need_metadata: bool) -> Self {
        Self {
            num_files: AtomicUsize::new(0),
            num_dirs: AtomicUsize::new(0),
            cancel: None,
            root_path,
            ignore_directories: &[],
            need_metadata,
        }
    }

    pub fn new(
        root_path: &'w Path,
        ignore_directories: &'w [PathBuf],
        need_metadata: bool,
        cancel: Option<&'w AtomicBool>,
    ) -> Self {
        Self {
            num_files: AtomicUsize::new(0),
            num_dirs: AtomicUsize::new(0),
            cancel,
            root_path,
            ignore_directories,
            need_metadata,
        }
    }

    fn should_ignore(&self, path: &Path) -> bool {
        self.ignore_directories
            .iter()
            .any(|ignore| path.starts_with(ignore))
    }
}

pub fn walk_it_without_root_chain(walk_data: &WalkData) -> Option<Node> {
    walk_shim(walk_data.root_path, walk_data)
}

pub fn walk_it(walk_data: &WalkData) -> Option<Node> {
    walk_shim(walk_data.root_path, walk_data).map(|node_tree| {
        if let Some(parent) = walk_data.root_path.parent() {
            let mut path = PathBuf::from(parent);
            let mut node = Node {
                children: vec![node_tree],
                name: path
                    .iter()
                    .next_back()
                    .expect("at least one parent segment in root path")
                    .to_string_lossy()
                    .into_owned()
                    .into_boxed_str(),
                metadata: metadata_of_path(&path).map(NodeMetadata::from),
            };
            while path.pop() {
                node = Node {
                    children: vec![node],
                    name: path
                        .iter()
                        .next_back()
                        .expect("at least one parent segment in root path")
                        .to_string_lossy()
                        .into_owned()
                        .into_boxed_str(),
                    metadata: metadata_of_path(&path).map(NodeMetadata::from),
                };
            }
            node
        } else {
            node_tree
        }
    })
}

fn walk_shim(path: &Path, walk_data: &WalkData) -> Option<Node> {
    if walk_data.should_ignore(path) {
        return None;
    }
    walk(path, walk_data)
}

fn walk(path: &Path, walk_data: &WalkData) -> Option<Node> {
    let metadata = metadata_of_path(path);
    let children = if metadata.as_ref().map(|x| x.is_dir()).unwrap_or_default() {
        walk_data.num_dirs.fetch_add(1, Ordering::Relaxed);
        let read_dir = fs::read_dir(path);
        match read_dir {
            Ok(entries) => entries
                .into_iter()
                .par_bridge()
                .filter_map(|entry| {
                    match &entry {
                        Ok(entry) => {
                            if walk_data
                                .cancel
                                .map(|x| x.load(Ordering::Relaxed))
                                .unwrap_or_default()
                            {
                                return None;
                            }
                            let path = entry.path();
                            if walk_data.should_ignore(&path) {
                                return None;
                            }
                            // doesn't traverse symlink
                            if let Ok(data) = entry.file_type() {
                                if data.is_dir() {
                                    walk(&path, walk_data)
                                } else {
                                    walk_data.num_files.fetch_add(1, Ordering::Relaxed);
                                    let name = entry
                                        .file_name()
                                        .to_string_lossy()
                                        .into_owned()
                                        .into_boxed_str();
                                    Some(Node {
                                        children: vec![],
                                        name,
                                        metadata: walk_data
                                            .need_metadata
                                            .then_some(entry)
                                            .and_then(|entry| {
                                                // doesn't traverse symlink
                                                entry.metadata().ok().map(NodeMetadata::from)
                                            }),
                                    })
                                }
                            } else {
                                None
                            }
                        }
                        Err(_) => None,
                    }
                })
                .collect(),
            Err(_) => Vec::new(),
        }
    } else {
        walk_data.num_files.fetch_add(1, Ordering::Relaxed);
        Vec::new()
    };
    if walk_data
        .cancel
        .map(|x| x.load(Ordering::Relaxed))
        .unwrap_or_default()
    {
        return None;
    }
    let name = path
        .file_name()
        .map(|x| x.to_string_lossy().into_owned().into_boxed_str())
        .unwrap_or_default();
    let mut children = children;
    children.sort_unstable_by(|a, b| a.name.cmp(&b.name));
    Some(Node {
        children,
        name,
        metadata: metadata.map(NodeMetadata::from),
    })
}

fn handle_error_and_retry(failed: &Error) -> bool {
    failed.kind() == std::io::ErrorKind::Interrupted
}

fn metadata_of_path(path: &Path) -> Option<Metadata> {
    // doesn't traverse symlink
    match path.symlink_metadata() {
        Ok(metadata) => Some(metadata),
        // If it's not found, we definitely don't want it.
        Err(e) if e.kind() == ErrorKind::NotFound => None,
        // If it's permission denied or something, we still want to insert it into the tree.
        Err(e) => {
            if handle_error_and_retry(&e) {
                // doesn't traverse symlink
                path.symlink_metadata().ok()
            } else {
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        io::Write,
        path::{Component, Path, PathBuf},
        time::{Duration, Instant},
    };
    use tempdir::TempDir;

    fn node_for_path<'a>(node: &'a Node, path: &Path) -> &'a Node {
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
    fn test_walk_simple_tree_without_metadata() {
        let tmp = TempDir::new("fswalk_simple").unwrap();
        let root = tmp.path();
        fs::create_dir(root.join("dir_a")).unwrap();
        fs::File::create(root.join("file_a.txt")).unwrap();
        fs::File::create(root.join("dir_a/file_b.log")).unwrap();
        let walk_data = WalkData::simple(root, false);
        let node = walk_it(&walk_data).unwrap();
        let root_node = node_for_path(&node, root);
        assert_eq!(
            &*root_node.name,
            root.file_name().unwrap().to_str().unwrap()
        );
        // Root + dir + 2 files
        let mut counts = (0, 0);
        fn traverse(n: &Node, counts: &mut (usize, usize)) {
            if n.children.is_empty() {
                counts.0 += 1;
            } else {
                counts.1 += 1;
            }
            for c in &n.children {
                traverse(c, counts);
            }
        }
        traverse(root_node, &mut counts);
        assert_eq!(counts.0 + counts.1, 4);
        // Metadata for files should be None (walk_data.need_metadata = false)
        fn assert_no_file_metadata(n: &Node) {
            if n.children.is_empty() {
                assert!(
                    n.metadata.is_none(),
                    "file node metadata should be None when not requested: {:?}",
                    n.name
                );
            } else {
                // directory metadata may be Some (free metadata) but it's optional; ensure type correctness when present
                if let Some(m) = n.metadata {
                    assert!(matches!(m.r#type, NodeFileType::Dir));
                }
                for c in &n.children {
                    assert_no_file_metadata(c);
                }
            }
        }
        assert_no_file_metadata(root_node);
    }

    #[test]
    fn test_walk_sorts_children_and_preserves_global_order() {
        let tmp = TempDir::new("fswalk_sorted_children").unwrap();
        let root = tmp.path();
        fs::create_dir(root.join("dir_beta")).unwrap();
        fs::create_dir(root.join("dir_alpha")).unwrap();
        fs::File::create(root.join("file_delta.txt")).unwrap();
        fs::File::create(root.join("file_alpha.txt")).unwrap();
        fs::File::create(root.join("file_gamma.txt")).unwrap();

        let walk_data = WalkData::simple(root, false);
        let node = walk_it(&walk_data).expect("walked tree");
        let root_node = node_for_path(&node, root);

        let observed: Vec<&str> = root_node
            .children
            .iter()
            .map(|child| &*child.name)
            .collect();
        let expected = vec![
            "dir_alpha",
            "dir_beta",
            "file_alpha.txt",
            "file_delta.txt",
            "file_gamma.txt",
        ];
        assert_eq!(observed, expected);

        fn collect_paths(node: &Node, prefix: &Path, acc: &mut Vec<PathBuf>) {
            let current = if prefix.as_os_str().is_empty() {
                PathBuf::from(&*node.name)
            } else {
                prefix.join(&*node.name)
            };
            acc.push(current.clone());
            for child in &node.children {
                collect_paths(child, &current, acc);
            }
        }

        let mut preorder = Vec::new();
        collect_paths(root_node, Path::new(""), &mut preorder);
        let mut sorted = preorder.clone();
        sorted.sort();
        assert_eq!(
            preorder, sorted,
            "preorder traversal should match lexicographic path order"
        );
    }

    #[test]
    fn test_walk_with_metadata_enabled() {
        let tmp = TempDir::new("fswalk_meta").unwrap();
        let root = tmp.path();
        fs::File::create(root.join("meta_file.txt")).unwrap();
        let walk_data = WalkData::simple(root, true);
        let node = walk_it(&walk_data).unwrap();
        let root_node = node_for_path(&node, root);
        fn find<'a>(node: &'a Node, name: &str) -> Option<&'a Node> {
            if &*node.name == name {
                return Some(node);
            }
            for c in &node.children {
                if let Some(n) = find(c, name) {
                    return Some(n);
                }
            }
            None
        }
        let file_node = find(root_node, "meta_file.txt").unwrap();
        assert!(matches!(
            file_node.metadata.map(|m| m.r#type),
            Some(NodeFileType::File)
        ));
    }

    #[test]
    fn test_symlink_not_traversed() {
        let tmp = TempDir::new("fswalk_symlink").unwrap();
        let root = tmp.path();
        fs::create_dir(root.join("real_dir")).unwrap();
        fs::File::create(root.join("real_dir/file.txt")).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(root.join("real_dir"), root.join("link_dir")).unwrap();
        let walk_data = WalkData::simple(root, true);
        let node = walk_it(&walk_data).unwrap();
        let root_node = node_for_path(&node, root);
        // Ensure link_dir exists as a file system entry but not traversed (should be a file node with no children)
        fn get_child<'a>(n: &'a Node, name: &str) -> Option<&'a Node> {
            n.children.iter().find(|c| &*c.name == name)
        }
        let link = get_child(root_node, "link_dir").unwrap();
        assert!(
            link.children.is_empty(),
            "symlink directory should not be traversed"
        );
    }

    // ── should_ignore tests (prefix-based matching) ──────────────────────

    #[test]
    fn should_ignore_exact_match() {
        let ignore = vec![PathBuf::from("/a/b/c")];
        let wd = WalkData::new(Path::new("/"), &ignore, false, None);
        assert!(wd.should_ignore(Path::new("/a/b/c")));
    }

    #[test]
    fn should_ignore_child_of_ignored_dir() {
        let ignore = vec![PathBuf::from("/a/b")];
        let wd = WalkData::new(Path::new("/"), &ignore, false, None);
        // direct child
        assert!(wd.should_ignore(Path::new("/a/b/c")));
        // deeply nested
        assert!(wd.should_ignore(Path::new("/a/b/c/d/e")));
    }

    #[test]
    fn should_ignore_does_not_match_sibling_with_shared_prefix_string() {
        // "/tmp/abc" is NOT a child of "/tmp/ab" — Path::starts_with is
        // component-aware, not string-prefix.
        let ignore = vec![PathBuf::from("/tmp/ab")];
        let wd = WalkData::new(Path::new("/"), &ignore, false, None);
        assert!(!wd.should_ignore(Path::new("/tmp/abc")));
        assert!(!wd.should_ignore(Path::new("/tmp/abc/d")));
    }

    #[test]
    fn should_ignore_unrelated_path() {
        let ignore = vec![PathBuf::from("/a/b")];
        let wd = WalkData::new(Path::new("/"), &ignore, false, None);
        assert!(!wd.should_ignore(Path::new("/x/y")));
        assert!(!wd.should_ignore(Path::new("/a")));
    }

    #[test]
    fn should_ignore_empty_ignore_list() {
        let ignore: Vec<PathBuf> = vec![];
        let wd = WalkData::new(Path::new("/"), &ignore, false, None);
        assert!(!wd.should_ignore(Path::new("/anything")));
    }

    #[test]
    fn should_ignore_multiple_ignore_dirs() {
        let ignore = vec![PathBuf::from("/a"), PathBuf::from("/x/y")];
        let wd = WalkData::new(Path::new("/"), &ignore, false, None);
        assert!(wd.should_ignore(Path::new("/a")));
        assert!(wd.should_ignore(Path::new("/a/b/c")));
        assert!(wd.should_ignore(Path::new("/x/y")));
        assert!(wd.should_ignore(Path::new("/x/y/z")));
        assert!(!wd.should_ignore(Path::new("/x")));
        assert!(!wd.should_ignore(Path::new("/b")));
    }

    #[test]
    fn should_ignore_parent_of_ignored_dir_is_not_ignored() {
        let ignore = vec![PathBuf::from("/a/b/c")];
        let wd = WalkData::new(Path::new("/"), &ignore, false, None);
        assert!(!wd.should_ignore(Path::new("/a")));
        assert!(!wd.should_ignore(Path::new("/a/b")));
    }

    // ── other unit tests ─────────────────────────────────────────────────

    #[test]
    fn test_handle_error_and_retry_only_interrupted() {
        let interrupted = Error::from(ErrorKind::Interrupted);
        assert!(handle_error_and_retry(&interrupted));
        let not_found = Error::from(ErrorKind::NotFound);
        assert!(!handle_error_and_retry(&not_found));
    }

    #[test]
    fn test_large_number_of_files_counts() {
        let tmp = TempDir::new("fswalk_many").unwrap();
        let root = tmp.path();
        for i in 0..50u32 {
            let mut f = fs::File::create(root.join(format!("f{i}.txt"))).unwrap();
            writeln!(f, "hello {i}").unwrap();
        }
        let walk_data = WalkData::simple(root, false);
        let node = walk_it(&walk_data).unwrap();
        let root_node = node_for_path(&node, root);
        // Expect 1 (root) + 50 file children
        assert_eq!(
            root_node.children.len(),
            50,
            "expected 50 files directly under root"
        );
    }

    #[test]
    #[ignore]
    fn test_search_root() {
        let done = AtomicBool::new(false);
        let path = [PathBuf::from("/System/Volumes/Data")];
        let walk_data = WalkData::new(Path::new("/"), &path, false, None);
        std::thread::scope(|s| {
            s.spawn(|| {
                let node = walk_it(&walk_data).unwrap();
                println!("root has {} children", node.children.len());
                done.store(true, Ordering::Relaxed);
            });
            s.spawn(|| {
                while !done.load(Ordering::Relaxed) {
                    let files = walk_data.num_files.load(Ordering::Relaxed);
                    let dirs = walk_data.num_dirs.load(Ordering::Relaxed);
                    println!("so far: {files} files, {dirs} dirs");
                    std::thread::sleep(std::time::Duration::from_secs(1));
                }
            });
        });
    }

    #[test]
    #[ignore]
    fn test_search_simulator() {
        let done = AtomicBool::new(false);
        let ignore = vec![PathBuf::from("/System/Volumes/Data")];
        let walk_data = WalkData::new(
            Path::new("/Library/Developer/CoreSimulator/Volumes/iOS_23A343"),
            &ignore,
            true,
            None,
        );
        std::thread::scope(|s| {
            s.spawn(|| {
                let node = walk_it(&walk_data).unwrap();
                println!("sim has {} children", node.children.len());
                done.store(true, Ordering::Relaxed);
            });
            s.spawn(|| {
                while !done.load(Ordering::Relaxed) {
                    let files = walk_data.num_files.load(Ordering::Relaxed);
                    let dirs = walk_data.num_dirs.load(Ordering::Relaxed);
                    println!("so far: {files} files, {dirs} dirs");
                    std::thread::sleep(std::time::Duration::from_secs(1));
                }
            });
        });
    }

    #[test]
    fn test_search_cancel() {
        let cancel = AtomicBool::new(false);
        let done = AtomicBool::new(false);
        let ignore = vec![PathBuf::from("/System/Volumes/Data")];
        let walk_data = WalkData::new(Path::new("/"), &ignore, false, Some(&cancel));
        std::thread::scope(|s| {
            s.spawn(|| {
                let node = walk_it(&walk_data);
                done.store(true, Ordering::Relaxed);
                assert!(node.is_none(), "expected walk to be cancelled");
            });
            s.spawn(|| {
                std::thread::sleep(std::time::Duration::from_millis(100));
                let time = Instant::now();
                cancel.store(true, Ordering::Relaxed);
                while !done.load(Ordering::Relaxed) {
                    std::thread::yield_now();
                }
                // Ensure cancellation happened quickly
                dbg!(time.elapsed());
                assert!(time.elapsed() < Duration::from_secs(1));
            });
        });
    }
}
