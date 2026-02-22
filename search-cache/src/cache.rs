use crate::{
    FileNodes, NameIndex, SearchOptions, SearchResultNode, SlabIndex, SlabNode,
    SlabNodeMetadataCompact, State, ThinSlab,
    highlight::derive_highlight_terms,
    persistent::{PersistentStorage, read_cache_from_file, write_cache_to_file},
    query_preprocessor::{expand_query_home_dirs, strip_query_quotes},
};
use anyhow::{Context, Result, anyhow};
use cardinal_sdk::{EventFlag, FsEvent, ScanType, current_event_id};
use cardinal_syntax::{optimize_query, parse_query};
use fswalk::{
    Node, NodeMetadata, WalkData, should_ignore_path, walk_it, walk_it_without_root_chain,
};
use hashbrown::HashSet;
use namepool::NamePool;
use search_cancel::CancellationToken;
use std::{
    ffi::OsStr,
    io::ErrorKind,
    path::{Path, PathBuf},
    sync::{
        LazyLock,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};
use thin_vec::ThinVec;
use tracing::{debug, info};
use typed_num::Num;

pub struct SearchCache {
    pub(crate) file_nodes: FileNodes,
    last_event_id: u64,
    rescan_count: u64,
    pub(crate) name_index: NameIndex,
    stop: Option<&'static AtomicBool>,
}

#[derive(Debug, Clone)]
pub struct SearchOutcome {
    pub nodes: Option<Vec<SlabIndex>>,
    pub highlights: Vec<String>,
}

impl SearchOutcome {
    fn new(nodes: Option<Vec<SlabIndex>>, highlights: Vec<String>) -> Self {
        Self { nodes, highlights }
    }
}

impl std::fmt::Debug for SearchCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SearchCache")
            .field("path", &self.file_nodes.path())
            .field("last_event_id", &self.last_event_id)
            .field("rescan_count", &self.rescan_count)
            .field("slab_root", &self.file_nodes.root())
            .field("slab.len()", &self.file_nodes.len())
            .field("name_index.len()", &self.name_index.len())
            .finish()
    }
}

impl SearchCache {
    pub fn ignore_paths(&self) -> &Vec<PathBuf> {
        self.file_nodes.ignore_paths()
    }

    /// The `path` is the root path of the constructed cache and fsevent watch path.
    pub fn try_read_persistent_cache(
        path: &Path,
        cache_path: &Path,
        current_ignore_paths: &Vec<PathBuf>,
        cancel: Option<&'static AtomicBool>,
    ) -> Result<Self> {
        read_cache_from_file(cache_path)
            .and_then(|x| {
                (x.path == path)
                    .then_some(())
                    .ok_or_else(|| {
                        anyhow!(
                            "Inconsistent root path: expected: {:?}, actual: {:?}",
                            path,
                            &x.path
                        )
                    })
                    .map(|()| x)
            })
            .and_then(|x| {
                (&x.ignore_paths == current_ignore_paths)
                    .then_some(())
                    .ok_or_else(|| {
                        anyhow!(
                            "Inconsistent ignore paths: expected: {:?}, actual: {:?}",
                            &current_ignore_paths,
                            &x.ignore_paths
                        )
                    })
                    .map(|()| x)
            })
            .map(
                |PersistentStorage {
                     version: _,
                     path,
                     ignore_paths,
                     slab_root,
                     slab,
                     name_index,
                     last_event_id,
                     rescan_count,
                 }| {
                    // name pool construction speed is fast enough that caching it doesn't worth it.
                    let name_index = NameIndex::construct_name_pool(name_index);
                    let slab = FileNodes::new(path, ignore_paths, slab, slab_root);
                    Self::new(slab, last_event_id, rescan_count, name_index, cancel)
                },
            )
    }

    /// Get the total number of files and directories in the cache.
    pub fn get_total_files(&self) -> usize {
        self.file_nodes.len()
    }

    pub fn walk_fs_with_ignore(path: &Path, ignore_paths: &[PathBuf]) -> Self {
        Self::walk_fs_with_walk_data(&WalkData::new(path, ignore_paths, false, || false), None)
            .unwrap()
    }

    pub fn walk_fs(path: &Path) -> Self {
        Self::walk_fs_with_walk_data(&WalkData::new(path, &[], false, || false), None).unwrap()
    }

    /// This function is expected to be called with WalkData which metadata is not fetched.
    /// If cancelled during walking, None is returned.
    pub fn walk_fs_with_walk_data<F>(
        walk_data: &WalkData<'_, F>,
        cancel: Option<&'static AtomicBool>,
    ) -> Option<Self>
    where
        F: Fn() -> bool + Send + Sync,
    {
        // Return None if cancelled
        fn walkfs_to_slab<F>(
            walk_data: &WalkData<'_, F>,
        ) -> Option<(SlabIndex, ThinSlab<SlabNode>, NameIndex)>
        where
            F: Fn() -> bool + Send + Sync,
        {
            // Build the tree of file names in parallel first (we cannot construct the slab directly
            // because slab nodes reference each other and we prefer to avoid locking).
            let visit_time = Instant::now();
            let Some(node) = walk_it(walk_data) else {
                info!("walk filesystem cancelled during walk_it.");
                return None;
            };
            info!(
                "Walk data: {:?}, time: {:?}",
                walk_data,
                visit_time.elapsed()
            );

            // Then create the slab.
            let slab_time = Instant::now();
            let mut slab = ThinSlab::new();
            let mut name_index = NameIndex::default();
            let slab_root = construct_node_slab_name_index(None, &node, &mut slab, &mut name_index);
            info!(
                "Slab & NameIndex construction time: {:?}, slab root: {:?}, slab len: {:?}",
                slab_time.elapsed(),
                slab_root,
                slab.len()
            );

            Some((slab_root, slab, name_index))
        }

        let last_event_id = current_event_id();
        let (slab_root, slab, name_index) = walkfs_to_slab(walk_data)?;
        let slab = FileNodes::new(
            walk_data.root_path.to_path_buf(),
            walk_data.ignore_directories.to_vec(),
            slab,
            slab_root,
        );
        // metadata cache inits later
        Some(Self::new(slab, last_event_id, 0, name_index, cancel))
    }

    fn new(
        slab: FileNodes,
        last_event_id: u64,
        rescan_count: u64,
        name_index: NameIndex,
        cancel: Option<&'static AtomicBool>,
    ) -> Self {
        Self {
            file_nodes: slab,
            last_event_id,
            rescan_count,
            name_index,
            stop: cancel,
        }
    }

    pub fn search_empty(&self, cancellation_token: CancellationToken) -> Option<Vec<SlabIndex>> {
        self.name_index.all_indices(cancellation_token)
    }

    #[cfg(test)]
    pub fn search(&mut self, line: &str) -> Result<Vec<SlabIndex>> {
        self.search_with_options(line, SearchOptions::default(), CancellationToken::noop())
            .map(|outcome| outcome.nodes.unwrap_or_default())
    }

    pub fn search_with_options(
        &mut self,
        line: &str,
        options: SearchOptions,
        cancellation_token: CancellationToken,
    ) -> Result<SearchOutcome> {
        let parsed = parse_query(line).map_err(|err| anyhow!("Failed to parse query: {err}"))?;
        let expanded = expand_query_home_dirs(parsed);
        let unquoted = strip_query_quotes(expanded);
        let highlights = derive_highlight_terms(&unquoted.expr);
        let optimized = optimize_query(unquoted);
        let search_time = Instant::now();
        let result = self.evaluate_expr(&optimized.expr, options, cancellation_token);
        info!("Search time: {:?}", search_time.elapsed());
        result.map(|nodes| SearchOutcome::new(nodes, highlights))
    }

    /// Get the path of the node in the slab.
    pub fn node_path(&self, index: SlabIndex) -> Option<PathBuf> {
        self.file_nodes.node_path(index)
    }

    /// Locate the slab index for an absolute path when it belongs to the watch root.
    pub fn node_index_for_path(&self, path: &Path) -> Option<SlabIndex> {
        let Ok(path) = path.strip_prefix("/") else {
            return None;
        };
        let mut current = self.file_nodes.root();
        for segment in path {
            let next = self.file_nodes[current]
                .children
                .iter()
                .find_map(|&child| {
                    let name = self.file_nodes[child].name();
                    if OsStr::new(name) == segment {
                        Some(child)
                    } else {
                        None
                    }
                })?;
            current = next;
        }
        Some(current)
    }

    /// Get all subnode indices of a given node index
    pub fn all_subnodes(
        &self,
        index: SlabIndex,
        cancel: CancellationToken,
    ) -> Option<Vec<SlabIndex>> {
        let mut result = Vec::new();
        let mut i = 0;
        self.all_subnodes_recursive(index, &mut result, &mut i, cancel)?;
        Some(result)
    }

    fn all_subnodes_recursive(
        &self,
        index: SlabIndex,
        out: &mut Vec<SlabIndex>,
        i: &mut usize,
        cancel: CancellationToken,
    ) -> Option<()> {
        for &child in &self.file_nodes[index].children {
            cancel.is_cancelled_sparse(*i)?;
            *i += 1;
            out.push(child);
            self.all_subnodes_recursive(child, out, i, cancel)?;
        }
        Some(())
    }

    fn push_node(&mut self, node: SlabNode) -> SlabIndex {
        let name = node.name();
        let index = self.file_nodes.insert(node);
        self.name_index.add_index(name, index, &self.file_nodes);
        index
    }

    /// Removes a node by path and its children recursively.
    fn remove_node_path(&mut self, path: &Path) -> Option<SlabIndex> {
        let Ok(path) = path.strip_prefix("/") else {
            return None;
        };
        let mut current = self.file_nodes.root();
        for name in path {
            if let Some(&index) = self.file_nodes[current]
                .children
                .iter()
                .find(|&&x| self.file_nodes[x].name() == name)
            {
                current = index;
            } else {
                return None;
            }
        }
        self.remove_node(current);
        Some(current)
    }

    // Create node chain of specific path
    fn create_node_chain(&mut self, path: &Path) -> SlabIndex {
        let path = path
            .strip_prefix("/")
            .expect("create_node_chain only accepts absolute path");
        let mut current = self.file_nodes.root();
        let mut current_path = PathBuf::from("/");
        for name in path {
            current_path.push(name);
            current = if let Some(&index) = self.file_nodes[current]
                .children
                .iter()
                .find(|&&x| self.file_nodes[x].name() == name)
            {
                index
            } else {
                // TODO(ldm0): optimize: slab node children is empty, we can create a node chain directly.
                let metadata = std::fs::symlink_metadata(&current_path)
                    .map(NodeMetadata::from)
                    .ok();
                let name = NAME_POOL.push(name.to_string_lossy().as_ref());
                let node = SlabNode::new(
                    Some(current),
                    name,
                    match metadata {
                        Some(metadata) => SlabNodeMetadataCompact::some(metadata),
                        None => SlabNodeMetadataCompact::unaccessible(),
                    },
                );
                let index = self.push_node(node);
                self.file_nodes[current].add_children(index);
                index
            };
        }
        current
    }

    fn should_ignore(&self, path: &Path) -> bool {
        should_ignore_path(path, self.file_nodes.ignore_paths())
    }

    // `Self::scan_path_recursive`function returns index of the constructed node(with metadata provided).
    // - If path is not under the watch root, None is returned.
    // - Procedure contains metadata fetching, if metadata fetching failed, None is returned.
    fn scan_path_recursive(&mut self, path: &Path) -> Option<SlabIndex> {
        // Ensure path is under the watch root
        if path.symlink_metadata().err().map(|e| e.kind()) == Some(ErrorKind::NotFound) {
            self.remove_node_path(path);
            return None;
        };
        if self.should_ignore(path) {
            return None;
        }
        let parent = path.parent().expect(
            "scan_path_recursive doesn't expected to scan root(should be filtered outside)",
        );
        // Ensure node of the path parent is existed
        let parent = self.create_node_chain(parent);
        // Remove node(if exists) and do a full rescan
        if let Some(&old_node) = self.file_nodes[parent]
            .children
            .iter()
            .find(|&&x| path.file_name() == Some(OsStr::new(self.file_nodes[x].name())))
        {
            self.remove_node(old_node);
        }
        // For incremental data, we need metadata
        let walk_data = WalkData::new(path, self.file_nodes.ignore_paths(), true, || {
            self.stop
                .map(|x| x.load(Ordering::Relaxed))
                .unwrap_or_default()
        });
        walk_it_without_root_chain(&walk_data).map(|node| {
            let node = self.create_node_slab_update_name_index_and_name_pool(Some(parent), &node);
            // Push the newly created node to the parent's children
            self.file_nodes[parent].add_children(node);
            node
        })
    }

    // `Self::scan_path_nonrecursive`function returns index of the constructed node.
    // - If path is not under the watch root, None is returned.
    // - Procedure contains metadata fetching, if metadata fetching failed, None is returned.
    #[allow(dead_code)]
    fn scan_path_nonrecursive(&mut self, path: &Path) -> Option<SlabIndex> {
        // Ensure path is under the watch root
        if path.symlink_metadata().err().map(|e| e.kind()) == Some(ErrorKind::NotFound) {
            self.remove_node_path(path);
            return None;
        };
        Some(self.create_node_chain(path))
    }

    pub fn walk_data<'p>(
        &self,
        phantom1: &'p mut PathBuf,
        phantom2: &'p mut Vec<PathBuf>,
        scan_cancellation_token: CancellationToken,
    ) -> WalkData<'p, impl Fn() -> bool + Send + Sync + Copy + 'static> {
        *phantom1 = self.file_nodes.path().to_path_buf();
        *phantom2 = self.file_nodes.ignore_paths().clone();
        let stop = self.stop;
        WalkData::new(phantom1, phantom2, false, move || {
            stop.map(|x| x.load(Ordering::Relaxed)).unwrap_or_default()
                || scan_cancellation_token.is_cancelled().is_none()
        })
    }

    pub fn rescan_with_walk_data<F>(&mut self, walk_data: &WalkData<'_, F>) -> Option<()>
    where
        F: Fn() -> bool + Send + Sync,
    {
        let Some(new_cache) = Self::walk_fs_with_walk_data(walk_data, self.stop) else {
            info!("Rescan cancelled.");
            return None;
        };
        *self = new_cache;
        Some(())
    }

    pub fn rescan(&mut self) {
        // Remove all memory consuming cache early for memory consumption in Self::walk_fs_new.
        let Some(new_cache) = Self::walk_fs_with_walk_data(
            &WalkData::new(
                self.file_nodes.path(),
                self.file_nodes.ignore_paths(),
                false,
                || {
                    self.stop
                        .map(|x| x.load(Ordering::Relaxed))
                        .unwrap_or_default()
                },
            ),
            self.stop,
        ) else {
            info!("Rescan cancelled.");
            return;
        };
        *self = new_cache;
    }

    /// Removes a node and its children recursively by index.
    fn remove_node(&mut self, index: SlabIndex) {
        fn remove_single_node(cache: &mut SearchCache, index: SlabIndex) {
            if let Some(node) = cache.file_nodes.try_remove(index) {
                let removed = cache.name_index.remove_index(node.name(), index);
                assert!(removed, "inconsistent name index and node");
            }
        }

        // Remove parent reference, make whole subtree unreachable.
        if let Some(parent) = self.file_nodes[index].parent() {
            self.file_nodes[parent].children.retain(|&x| x != index);
        }
        let mut stack = vec![index];
        while let Some(current) = stack.pop() {
            stack.extend_from_slice(&self.file_nodes[current].children);
            remove_single_node(self, current);
        }
    }

    pub fn flush_snapshot_to_file(&mut self, cache_path: &Path) -> Result<()> {
        let name_index = self.name_index.as_persistent();
        let slab = self.file_nodes.take_slab();

        let storage = PersistentStorage {
            version: Num,
            last_event_id: self.last_event_id,
            rescan_count: self.rescan_count,
            path: self.file_nodes.path().to_path_buf(),
            ignore_paths: self.file_nodes.ignore_paths().clone(),
            slab_root: self.file_nodes.root(),
            name_index,
            slab,
        };

        let flush_result =
            write_cache_to_file(cache_path, &storage).context("Write cache to file failed.");

        let PersistentStorage { slab, .. } = storage;
        self.file_nodes.put_slab(slab);

        flush_result
    }

    pub fn flush_to_file(self, cache_path: &Path) -> Result<()> {
        let Self {
            file_nodes,
            last_event_id,
            rescan_count,
            name_index,
            stop: _,
        } = self;
        let (path, ignore_paths, slab_root, slab) = file_nodes.into_parts();
        let name_index = name_index.into_persistent();
        write_cache_to_file(
            cache_path,
            &PersistentStorage {
                version: Num,
                path,
                ignore_paths,
                slab_root,
                slab,
                name_index,
                last_event_id,
                rescan_count,
            },
        )
        .context("Write cache to file failed.")
    }

    fn update_last_event_id(&mut self, event_id: u64) {
        if event_id <= self.last_event_id {
            debug!("last_event_id {} |< {event_id}", self.last_event_id);
        } else {
            debug!("last_event_id {} => {event_id}", self.last_event_id);
            self.last_event_id = event_id;
        }
    }

    pub fn last_event_id(&mut self) -> u64 {
        self.last_event_id
    }

    pub fn rescan_count(&self) -> u64 {
        self.rescan_count
    }

    /// Note that this function doesn't fetch metadata(even if it's not cahced) for the nodes.
    pub fn query_files(
        &mut self,
        query: String,
        cancellation_token: CancellationToken,
    ) -> Result<Option<Vec<SearchResultNode>>> {
        self.query_files_with_options(query, SearchOptions::default(), cancellation_token)
    }

    pub fn query_files_with_options(
        &mut self,
        query: String,
        options: SearchOptions,
        cancellation_token: CancellationToken,
    ) -> Result<Option<Vec<SearchResultNode>>> {
        self.search_with_options(&query, options, cancellation_token)
            .map(|outcome| {
                outcome
                    .nodes
                    .map(|nodes| self.expand_file_nodes_inner::<false>(&nodes))
            })
    }

    /// Returns a node info vector with the same length as the input nodes.
    /// If the given node is not found, an empty SearchResultNode is returned.
    pub fn expand_file_nodes(&mut self, nodes: &[SlabIndex]) -> Vec<SearchResultNode> {
        self.expand_file_nodes_inner::<true>(nodes)
    }

    fn expand_file_nodes_inner<const FETCH_META: bool>(
        &mut self,
        nodes: &[SlabIndex],
    ) -> Vec<SearchResultNode> {
        nodes
            .iter()
            .copied()
            .map(|node_index| {
                let path = self.node_path(node_index);
                let metadata = self
                    .file_nodes
                    .get_mut(node_index)
                    .map(|node| {
                        match (node.state(), &path) {
                            (State::None, Some(path)) if FETCH_META => {
                                // try fetching metadata if it's not cached and cache them
                                let metadata = match std::fs::symlink_metadata(path) {
                                    Ok(metadata) => SlabNodeMetadataCompact::some(metadata.into()),
                                    Err(_) => SlabNodeMetadataCompact::unaccessible(),
                                };
                                node.metadata = metadata;
                                metadata
                            }
                            _ => node.metadata,
                        }
                    })
                    .unwrap_or_else(SlabNodeMetadataCompact::unaccessible);
                SearchResultNode {
                    path: path.unwrap_or_default(),
                    metadata,
                }
            })
            .collect()
    }

    pub fn handle_fs_events(&mut self, events: Vec<FsEvent>) -> Result<(), HandleFSEError> {
        let max_event_id = events.iter().map(|e| e.id).max();
        // If rescan needed, early exit.
        if events.iter().any(|event| {
            if event.flag.contains(EventFlag::HistoryDone) {
                info!("History processing done: {:?}", event);
            }
            if event.should_rescan(self.file_nodes.path()) {
                info!("Event rescan: {:?}", event);
                true
            } else {
                false
            }
        }) {
            self.rescan_count = self.rescan_count.saturating_add(1);
            return Err(HandleFSEError::Rescan);
        }
        for scan_path in scan_paths(events) {
            info!("Scanning path: {scan_path:?}");
            let folder = self.scan_path_recursive(&scan_path);
            if folder.is_some() {
                info!("Node changed: {folder:?}");
            }
        }
        if let Some(max_event_id) = max_event_id {
            self.update_last_event_id(max_event_id);
        }
        Ok(())
    }
}

/// Compute the minimal set of paths that must be rescanned for a batch of FsEvents.
///
/// Goals:
/// 1. Filter out events that do not require incremental rescans (e.g. `ReScan` / `Nop` variants
///    such as RootChanged or HistoryDone). Higher-level logic either rebuilds the cache or simply
///    updates the event id for those.
/// 2. Keep only `ScanType::SingleNode` and `ScanType::Folder` paths.
/// 3. Deduplicate ancestors and descendants:
///    - Skip a path if it is already covered by an ancestor (`path.starts_with(ancestor)`).
///    - When inserting an ancestor, remove all of its descendants that were previously added.
///    - Keep only a single entry for identical paths; later duplicates are considered covered.
/// 4. Return the minimal cover—the smallest set of paths whose rescans still cover every change.
///
/// Usage:
/// - `SearchCache::handle_fs_events` iterates over the returned paths and calls
///   `scan_path_recursive` on each of them to avoid redundant rescans of descendants or duplicates.
/// - High-frequency FSEvents often bubble many changes from the same subtree; merging them here
///   significantly reduces IO and metadata fetch work downstream.
///
/// Complexity:
/// - Approximately O(n log n + m * depth): sort by depth first, then scan linearly while checking
///   ancestors.
/// - If we ever need additional speed we can explore trie/prefix-tree structures.
///
/// Corner cases:
/// - Empty input → returns an empty `Vec`.
/// - Duplicate identical paths → only one is kept (later duplicates are skipped via `starts_with`).
/// - Child path seen before its ancestor → the ancestor replaces all children, so only the ancestor remains.
/// - Ancestor seen before its child → the child is skipped.
/// - Sibling paths never interfere with each other and are all kept.
/// - Paths that merely share prefixes (e.g. `/foo/bar` vs `/foo/barista`) are both retained because
///   `Path::starts_with` compares path components.
/// - Folder and `SingleNode` events participate together; we only look at the hierarchy.
/// - `PathBuf` values are compared as-is without normalisation, so symlinks are left untouched—the
///   caller must provide consistent inputs.
///
/// Result:
/// - Local benchmarks skipped rescans for 173,034 events out of 415,449.
fn scan_paths(events: Vec<FsEvent>) -> Vec<PathBuf> {
    let mut candidates: Vec<(PathBuf, usize)> = events
        .into_iter()
        .filter(|event| {
            // Sometimes there are ridiculous events assuming dir as file, so we always scan them as folder
            matches!(
                event.flag.scan_type(),
                ScanType::SingleNode | ScanType::Folder
            )
        })
        .map(|event| {
            let path = event.path;
            let depth = path_depth(&path);
            (path, depth)
        })
        .collect();

    candidates.sort_unstable_by(|(path_a, depth_a), (path_b, depth_b)| {
        depth_a.cmp(depth_b).then_with(|| path_a.cmp(path_b))
    });
    candidates.dedup_by(|(path_a, _), (path_b, _)| path_a == path_b);

    let mut selected = Vec::with_capacity(candidates.len());
    let mut selected_set = HashSet::with_capacity(candidates.len());
    for (path, _) in candidates {
        if has_selected_ancestor(&path, &selected_set) {
            continue;
        }
        selected_set.insert(path.clone());
        selected.push(path);
    }
    selected
}

fn path_depth(path: &Path) -> usize {
    path.components().count()
}

fn has_selected_ancestor(path: &Path, selected: &HashSet<PathBuf>) -> bool {
    if selected.is_empty() {
        return false;
    }
    if selected.contains(path) {
        return true;
    }
    let mut ancestor = path.to_path_buf();
    while ancestor.pop() {
        if selected.contains(&ancestor) {
            return true;
        }
    }
    false
}

/// Error type for `SearchCache::handle_fs_event`.
#[derive(Debug)]
pub enum HandleFSEError {
    /// Full rescan is required.
    Rescan,
}

/// Note: This function is expected to be called with WalkData which metadata is not fetched.
fn construct_node_slab_name_index(
    parent: Option<SlabIndex>,
    node: &Node,
    slab: &mut ThinSlab<SlabNode>,
    name_index: &mut NameIndex,
) -> SlabIndex {
    let metadata = match node.metadata {
        Some(metadata) => SlabNodeMetadataCompact::some(metadata),
        None => SlabNodeMetadataCompact::none(),
    };
    let name = NAME_POOL.push(&node.name);
    let slab_node = SlabNode::new(parent, name, metadata);
    let index = slab.insert(slab_node);
    unsafe {
        // SAFETY: fswalk sorts each directory's children by name before we recurse,
        // so this preorder traversal visits nodes in lexicographic path order.
        name_index.add_index_ordered(name, index);
    }
    slab[index].children = node
        .children
        .iter()
        .map(|node| construct_node_slab_name_index(Some(index), node, slab, name_index))
        .collect();
    index
}

impl SearchCache {
    /// ATTENTION: This function doesn't remove existing node, you should remove it
    /// before creating the new subtree, or the old subtree nodes will be dangling.
    ///
    /// ATTENTION1: This function should only called with Node fetched with metadata.
    fn create_node_slab_update_name_index_and_name_pool(
        &mut self,
        parent: Option<SlabIndex>,
        node: &Node,
    ) -> SlabIndex {
        let metadata = match node.metadata {
            Some(metadata) => SlabNodeMetadataCompact::some(metadata),
            // This function should only be called with Node fetched with metadata
            None => SlabNodeMetadataCompact::unaccessible(),
        };
        let name = NAME_POOL.push(&node.name);
        let slab_node = SlabNode::new(parent, name, metadata);
        let index = self.push_node(slab_node);
        self.file_nodes[index].children = node
            .children
            .iter()
            .map(|node| self.create_node_slab_update_name_index_and_name_pool(Some(index), node))
            .collect::<ThinVec<_>>();
        index
    }
}

pub static NAME_POOL: LazyLock<NamePool> = LazyLock::new(NamePool::new);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::CONTENT_BUFFER_BYTES;
    use fswalk::NodeFileType;
    use std::{
        fs,
        path::{Component, Path, PathBuf},
    };
    use tempdir::TempDir;

    fn depth(path: &Path) -> usize {
        path.components()
            .filter(|c| matches!(c, Component::Normal(_)))
            .count()
    }

    fn guard_indices(result: Result<SearchOutcome>) -> Vec<SlabIndex> {
        result
            .expect("search should succeed")
            .nodes
            .expect("noop cancellation token should not cancel")
    }

    fn guard_nodes(result: Result<Option<Vec<SearchResultNode>>>) -> Vec<SearchResultNode> {
        result
            .expect("query should succeed")
            .expect("noop cancellation token should not cancel")
    }

    fn query(cache: &mut SearchCache, query: impl Into<String>) -> Vec<SearchResultNode> {
        guard_nodes(cache.query_files(query.into(), CancellationToken::noop()))
    }

    fn make_node(name: &str, children: Vec<Node>) -> Node {
        Node {
            children,
            name: name.into(),
            metadata: None,
        }
    }

    fn make_leaf(name: &str) -> Node {
        make_node(name, vec![])
    }

    fn push_child(slab: &mut ThinSlab<SlabNode>, parent: SlabIndex, name: &str) -> SlabIndex {
        let idx = slab.insert(SlabNode::new(
            Some(parent),
            NAME_POOL.push(name),
            SlabNodeMetadataCompact::none(),
        ));
        slab[parent].children.push(idx);
        idx
    }

    fn find_node_index(cache: &SearchCache, path: &Path) -> SlabIndex {
        let path = path.strip_prefix("/").expect("absolute path");
        let mut current = cache.file_nodes.root();
        for name in path {
            let name = name.to_string_lossy();
            current = *cache.file_nodes[current]
                .children
                .iter()
                .find(|&&idx| cache.file_nodes[idx].name() == name.as_ref())
                .expect("node should exist");
        }
        current
    }

    fn manual_target_tree_file_nodes() -> (FileNodes, [SlabIndex; 3]) {
        let mut slab = ThinSlab::new();
        let root_idx = slab.insert(SlabNode::new(
            None,
            NAME_POOL.push("root"),
            SlabNodeMetadataCompact::none(),
        ));
        let alpha = push_child(&mut slab, root_idx, "alpha");
        let beta = push_child(&mut slab, root_idx, "beta");
        let root_target = push_child(&mut slab, root_idx, "target.txt");
        let alpha_target = push_child(&mut slab, alpha, "target.txt");
        let beta_target = push_child(&mut slab, beta, "target.txt");
        let file_nodes = FileNodes::new(PathBuf::from("/virtual/root"), Vec::new(), slab, root_idx);
        (file_nodes, [root_target, alpha_target, beta_target])
    }

    #[test]
    fn test_construct_node_slab_name_index_preserves_path_order() {
        let tree = make_node(
            "root",
            vec![
                make_node("alpha", vec![make_leaf("shared")]),
                make_node("beta", vec![make_node("gamma", vec![make_leaf("shared")])]),
                make_leaf("shared"),
            ],
        );
        let mut slab = ThinSlab::new();
        let mut name_index = NameIndex::default();
        let root = construct_node_slab_name_index(None, &tree, &mut slab, &mut name_index);
        let file_nodes = FileNodes::new(PathBuf::from("/virtual/root"), Vec::new(), slab, root);

        let shared_entries = name_index.get("shared").expect("shared entries");
        assert_eq!(shared_entries.len(), 3);
        let paths: Vec<PathBuf> = shared_entries
            .iter()
            .map(|index| file_nodes.node_path(*index).expect("path must exist"))
            .collect();
        let mut sorted = paths.clone();
        sorted.sort();
        assert_eq!(
            paths, sorted,
            "shared entries must follow lexicographic path order"
        );
    }

    #[test]
    fn test_name_index_add_index_sorts_paths() {
        let (file_nodes, targets) = manual_target_tree_file_nodes();
        let mut name_index = NameIndex::default();

        for &index in targets.iter().rev() {
            name_index.add_index("target.txt", index, &file_nodes);
        }

        let entries = name_index
            .get("target.txt")
            .expect("target.txt entries must exist");
        assert_eq!(entries.len(), 3);
        let paths: Vec<PathBuf> = entries
            .iter()
            .map(|index| file_nodes.node_path(*index).expect("path exists"))
            .collect();
        let mut sorted = paths.clone();
        sorted.sort();
        assert_eq!(paths, sorted, "add_index must maintain lexicographic order");
    }

    #[test]
    fn test_walk_fs_with_walk_data_preserves_name_index_order() {
        let temp_dir =
            TempDir::new("walk_fs_with_walk_data_orders").expect("Failed to create temp dir");
        let root = temp_dir.path();
        fs::create_dir(root.join("beta")).unwrap();
        fs::create_dir(root.join("alpha")).unwrap();
        fs::File::create(root.join("target.txt")).unwrap();
        fs::File::create(root.join("alpha/target.txt")).unwrap();
        fs::File::create(root.join("beta/target.txt")).unwrap();

        let walk_data = WalkData::simple(root, false);
        let cache = SearchCache::walk_fs_with_walk_data(&walk_data, None).expect("walk cache");

        let entries = cache
            .name_index
            .get("target.txt")
            .expect("target.txt entries");
        assert_eq!(entries.len(), 3);
        let paths: Vec<PathBuf> = entries
            .iter()
            .map(|index| cache.file_nodes.node_path(*index).expect("path exists"))
            .collect();
        let mut sorted = paths.clone();
        sorted.sort();
        assert_eq!(
            paths, sorted,
            "walk_fs_with_walk_data must yield lexicographically ordered slab indices"
        );
    }

    #[test]
    fn test_search_cache_walk_and_verify() {
        let temp_dir = TempDir::new("test_cache").expect("Failed to create temp directory");
        let temp_path = temp_dir.path();

        fs::create_dir_all(temp_path.join("subdir")).expect("Failed to create subdirectory");
        fs::File::create(temp_path.join("file1.txt")).expect("Failed to create file");
        fs::File::create(temp_path.join("subdir/file2.txt")).expect("Failed to create file");

        let cache = SearchCache::walk_fs(temp_path);

        assert_eq!(cache.file_nodes.len(), 4 + depth(temp_path));
        assert_eq!(cache.name_index.len(), 4 + depth(temp_path));
    }

    #[test]
    fn create_node_chain_existing_path_is_idempotent() {
        let temp_dir = TempDir::new("create_node_chain_existing_path_is_idempotent")
            .expect("Failed to create temp directory");
        let root = temp_dir.path();
        fs::create_dir_all(root.join("alpha/beta")).expect("Failed to create directories");

        let mut cache = SearchCache::walk_fs(root);
        let target = root.join("alpha/beta");
        let before = cache.file_nodes.len();
        let first = cache.create_node_chain(&target);
        let after_first = cache.file_nodes.len();
        let second = cache.create_node_chain(&target);

        assert_eq!(first, second, "existing path should return stable index");
        assert_eq!(before, after_first, "existing path should not add nodes");
        assert_eq!(
            cache.file_nodes.node_path(first).expect("node path exists"),
            target
        );
    }

    #[test]
    fn create_node_chain_creates_missing_tail_with_unaccessible_metadata() {
        let temp_dir = TempDir::new("create_node_chain_creates_missing_tail")
            .expect("Failed to create temp directory");
        let root = temp_dir.path();
        fs::create_dir_all(root.join("existing")).expect("Failed to create directories");

        let mut cache = SearchCache::walk_fs(root);
        let target = root.join("existing/missing/leaf");
        let before = cache.file_nodes.len();
        let index = cache.create_node_chain(&target);
        let after = cache.file_nodes.len();

        assert_eq!(after, before + 2, "missing tail should add two nodes");
        assert_eq!(
            cache.file_nodes.node_path(index).expect("node path exists"),
            target
        );

        let existing_index = find_node_index(&cache, &root.join("existing"));
        let missing_index = find_node_index(&cache, &root.join("existing/missing"));
        let leaf_index = find_node_index(&cache, &target);

        assert_eq!(cache.file_nodes[existing_index].state(), State::Some);
        assert_eq!(cache.file_nodes[missing_index].state(), State::Unaccessible);
        assert_eq!(cache.file_nodes[leaf_index].state(), State::Unaccessible);

        let missing_entries = cache
            .name_index
            .get("missing")
            .expect("missing entry should exist");
        assert!(missing_entries.iter().any(|&idx| idx == missing_index));
        let leaf_entries = cache
            .name_index
            .get("leaf")
            .expect("leaf entry should exist");
        assert!(leaf_entries.iter().any(|&idx| idx == leaf_index));
    }

    #[test]
    fn create_node_chain_existing_file_metadata_is_unchanged() {
        let temp_dir = TempDir::new("create_node_chain_existing_file_metadata")
            .expect("Failed to create temp directory");
        let root = temp_dir.path();
        fs::create_dir_all(root.join("dir")).expect("Failed to create directories");
        fs::File::create(root.join("dir/file.txt")).expect("Failed to create file");

        let mut cache = SearchCache::walk_fs(root);
        let target = root.join("dir/file.txt");
        let index = cache.create_node_chain(&target);

        assert_eq!(cache.file_nodes[index].state(), State::None);
        assert_eq!(cache.file_nodes[index].file_type_hint(), NodeFileType::File);
    }

    #[test]
    fn create_node_chain_root_returns_root() {
        let temp_dir = TempDir::new("create_node_chain_root_returns_root")
            .expect("Failed to create temp directory");
        let root = temp_dir.path();
        let mut cache = SearchCache::walk_fs(root);
        let before = cache.file_nodes.len();

        let index = cache.create_node_chain(Path::new("/"));

        assert_eq!(index, cache.file_nodes.root());
        assert_eq!(cache.file_nodes.len(), before);
    }

    #[test]
    #[should_panic(expected = "create_node_chain only accepts absolute path")]
    fn create_node_chain_rejects_relative_paths() {
        let temp_dir = TempDir::new("create_node_chain_rejects_relative_paths")
            .expect("Failed to create temp directory");
        let root = temp_dir.path();
        let mut cache = SearchCache::walk_fs(root);

        let _ = cache.create_node_chain(Path::new("relative/path"));
    }

    // --- New comprehensive tests for recent changes ---

    #[test]
    fn node_index_for_path_with_absolute_paths() {
        let temp_dir =
            TempDir::new("node_index_for_path_absolute").expect("Failed to create temp directory");
        let root = temp_dir.path();
        fs::create_dir_all(root.join("alpha/beta/gamma")).expect("Failed to create directories");
        fs::File::create(root.join("alpha/beta/file.txt")).expect("Failed to create file");

        let cache = SearchCache::walk_fs(root);

        // Test retrieval with absolute paths
        let alpha_index = cache.node_index_for_path(&root.join("alpha"));
        assert!(alpha_index.is_some(), "should find alpha directory");

        let beta_index = cache.node_index_for_path(&root.join("alpha/beta"));
        assert!(beta_index.is_some(), "should find alpha/beta directory");

        let file_index = cache.node_index_for_path(&root.join("alpha/beta/file.txt"));
        assert!(file_index.is_some(), "should find file.txt");

        let nonexistent = cache.node_index_for_path(&root.join("alpha/nonexistent"));
        assert!(nonexistent.is_none(), "should not find nonexistent path");
    }

    #[test]
    fn node_index_for_path_with_relative_path_fails() {
        let temp_dir =
            TempDir::new("node_index_for_path_relative").expect("Failed to create temp directory");
        let root = temp_dir.path();
        fs::create_dir_all(root.join("dir")).expect("Failed to create directory");

        let cache = SearchCache::walk_fs(root);

        // Relative paths should not be found
        let result = cache.node_index_for_path(Path::new("dir"));
        assert!(result.is_none(), "relative paths should not match");
    }

    #[test]
    fn node_path_returns_absolute_paths() {
        let temp_dir = TempDir::new("node_path_absolute").expect("Failed to create temp directory");
        let root = temp_dir.path();
        fs::create_dir_all(root.join("folder/subfolder")).expect("Failed to create directories");
        fs::File::create(root.join("folder/file.txt")).expect("Failed to create file");

        let cache = SearchCache::walk_fs(root);

        let folder_index = cache
            .node_index_for_path(&root.join("folder"))
            .expect("folder should exist");
        let folder_path = cache
            .node_path(folder_index)
            .expect("should get folder path");
        assert!(
            folder_path.is_absolute(),
            "returned path should be absolute"
        );
        assert_eq!(folder_path, root.join("folder"));

        let file_index = cache
            .node_index_for_path(&root.join("folder/file.txt"))
            .expect("file should exist");
        let file_path = cache.node_path(file_index).expect("should get file path");
        assert!(file_path.is_absolute(), "returned path should be absolute");
        assert_eq!(file_path, root.join("folder/file.txt"));
    }

    #[test]
    fn walk_it_creates_full_parent_chain() {
        let temp_dir =
            TempDir::new("walk_it_parent_chain").expect("Failed to create temp directory");
        let root = temp_dir.path();
        fs::create_dir_all(root.join("deep/nested/structure"))
            .expect("Failed to create directories");

        let cache = SearchCache::walk_fs(root);

        // Verify root node exists at the filesystem root
        let root_index = cache.file_nodes.root();
        let root_name = cache.file_nodes[root_index].name();
        assert_eq!(root_name, "/", "root node should be named '/'");

        // Traverse down to verify the chain is complete
        let mut current_index = root_index;
        for segment in root.strip_prefix("/").unwrap() {
            let found = cache.file_nodes[current_index]
                .children
                .iter()
                .find(|&&child_idx| cache.file_nodes[child_idx].name() == segment);
            assert!(found.is_some(), "should find segment in parent chain",);
            current_index = *found.unwrap();
        }
    }

    #[test]
    fn walk_it_deep_path_has_all_ancestors() {
        let temp_dir =
            TempDir::new("walk_it_deep_ancestors").expect("Failed to create temp directory");
        let root = temp_dir.path();
        fs::create_dir_all(root.join("a/b/c/d/e")).expect("Failed to create directories");

        let cache = SearchCache::walk_fs(root);

        // Find the deepest node
        let e_index = cache
            .node_index_for_path(&root.join("a/b/c/d/e"))
            .expect("deepest node should exist");

        // Verify we can navigate all the way back to root
        let mut current = e_index;
        let mut visited = vec![];

        // Traverse upwards to collect all ancestors
        loop {
            visited.push(current);
            if current == cache.file_nodes.root() {
                break;
            }
            let parent = cache.file_nodes[current].parent();
            if let Some(p) = parent {
                current = p;
            } else {
                panic!("Node should have parent until reaching root");
            }
        }

        // Should have at least: /, <temp_dir_segments...>, a, b, c, d, e
        assert!(
            visited.len() >= 6,
            "should have visited at least 6 ancestors (including self)"
        );
    }

    #[test]
    fn remove_node_path_with_absolute_paths() {
        let temp_dir =
            TempDir::new("remove_node_absolute").expect("Failed to create temp directory");
        let root = temp_dir.path();
        fs::create_dir_all(root.join("to_remove/child")).expect("Failed to create directories");
        fs::File::create(root.join("to_remove/file.txt")).expect("Failed to create file");

        let mut cache = SearchCache::walk_fs(root);

        let before_count = cache.file_nodes.len();
        let target_path = root.join("to_remove");

        // Verify node exists before removal
        assert!(cache.node_index_for_path(&target_path).is_some());

        // Remove the node
        let removed = cache.remove_node_path(&target_path);
        assert!(removed.is_some(), "should return removed node index");

        let after_count = cache.file_nodes.len();
        assert!(
            after_count < before_count,
            "node count should decrease after removal"
        );

        // Verify node no longer exists
        assert!(cache.node_index_for_path(&target_path).is_none());
        assert!(
            cache
                .node_index_for_path(&target_path.join("child"))
                .is_none()
        );
    }

    #[test]
    fn create_node_chain_with_deep_missing_ancestors() {
        let temp_dir = TempDir::new("create_node_chain_deep_missing")
            .expect("Failed to create temp directory");
        let root = temp_dir.path();
        fs::create_dir_all(root.join("exists")).expect("Failed to create directory");

        let mut cache = SearchCache::walk_fs(root);
        let before = cache.file_nodes.len();

        // Create chain for a deeply nested path that doesn't exist on disk
        let target = root.join("exists/missing1/missing2/missing3/leaf");
        let index = cache.create_node_chain(&target);
        let after = cache.file_nodes.len();

        // Should add 4 new nodes: missing1, missing2, missing3, leaf
        assert_eq!(after, before + 4, "should add 4 missing nodes");

        // Verify path is correct
        assert_eq!(
            cache.node_path(index).expect("node path should exist"),
            target
        );

        // Verify all intermediate nodes are marked as unaccessible
        let missing1_index = cache
            .node_index_for_path(&root.join("exists/missing1"))
            .expect("missing1 should exist");
        assert_eq!(
            cache.file_nodes[missing1_index].state(),
            State::Unaccessible
        );
    }

    #[test]
    fn create_node_chain_preserves_existing_metadata() {
        let temp_dir = TempDir::new("create_node_chain_preserves_metadata")
            .expect("Failed to create temp directory");
        let root = temp_dir.path();
        fs::create_dir_all(root.join("dir")).expect("Failed to create directory");
        fs::File::create(root.join("dir/existing.txt")).expect("Failed to create file");

        let mut cache = SearchCache::walk_fs(root);

        // Get the original metadata state
        let existing_index = cache
            .node_index_for_path(&root.join("dir/existing.txt"))
            .expect("file should exist");
        let original_state = cache.file_nodes[existing_index].state();
        let original_type = cache.file_nodes[existing_index].file_type_hint();

        // Call create_node_chain on the existing path
        let new_index = cache.create_node_chain(&root.join("dir/existing.txt"));

        // Should return the same index
        assert_eq!(new_index, existing_index);

        // Metadata should be unchanged
        assert_eq!(cache.file_nodes[new_index].state(), original_state);
        assert_eq!(cache.file_nodes[new_index].file_type_hint(), original_type);
    }

    #[test]
    fn scan_path_recursive_with_absolute_path() {
        let temp_dir =
            TempDir::new("scan_path_recursive_absolute").expect("Failed to create temp directory");
        let root = temp_dir.path();

        let mut cache = SearchCache::walk_fs(root);
        let initial_count = cache.file_nodes.len();

        // Create new content after initial walk
        fs::create_dir_all(root.join("new_dir/sub")).expect("Failed to create directories");
        fs::File::create(root.join("new_dir/file.txt")).expect("Failed to create file");

        // Scan the new path with absolute path
        let result = cache.scan_path_recursive(&root.join("new_dir"));
        assert!(result.is_some(), "should successfully scan new directory");

        let after_count = cache.file_nodes.len();
        assert!(
            after_count > initial_count,
            "should have added nodes from scan"
        );

        // Verify the new nodes are accessible
        assert!(cache.node_index_for_path(&root.join("new_dir")).is_some());
        assert!(
            cache
                .node_index_for_path(&root.join("new_dir/sub"))
                .is_some()
        );
        assert!(
            cache
                .node_index_for_path(&root.join("new_dir/file.txt"))
                .is_some()
        );
    }

    #[test]
    fn scan_path_recursive_handles_nonexistent_file() {
        let temp_dir =
            TempDir::new("scan_path_nonexistent").expect("Failed to create temp directory");
        let root = temp_dir.path();
        fs::create_dir_all(root.join("existing")).expect("Failed to create directory");

        let mut cache = SearchCache::walk_fs(root);

        // Try to scan a path that doesn't exist
        let result = cache.scan_path_recursive(&root.join("existing/nonexistent.txt"));
        assert!(result.is_none(), "should return None for nonexistent path");
    }

    #[test]
    fn path_handling_with_unicode_characters() {
        let temp_dir = TempDir::new("path_unicode").expect("Failed to create temp directory");
        let root = temp_dir.path();
        fs::create_dir_all(root.join("文件夹/子目录")).expect("Failed to create directories");
        fs::File::create(root.join("文件夹/文件.txt")).expect("Failed to create file");

        let cache = SearchCache::walk_fs(root);

        // Verify Unicode paths work correctly
        let folder_index = cache.node_index_for_path(&root.join("文件夹"));
        assert!(folder_index.is_some(), "should find Unicode folder");

        let file_index = cache.node_index_for_path(&root.join("文件夹/文件.txt"));
        assert!(file_index.is_some(), "should find Unicode file");

        // Verify path reconstruction
        let file_path = cache
            .node_path(file_index.unwrap())
            .expect("should get file path");
        assert_eq!(file_path, root.join("文件夹/文件.txt"));
    }

    #[test]
    fn path_handling_with_special_characters() {
        let temp_dir = TempDir::new("path_special").expect("Failed to create temp directory");
        let root = temp_dir.path();
        fs::create_dir_all(root.join("dir with spaces/sub-dir"))
            .expect("Failed to create directories");
        fs::File::create(root.join("dir with spaces/file (1).txt")).expect("Failed to create file");

        let cache = SearchCache::walk_fs(root);

        // Verify paths with spaces and special chars work
        let dir_index = cache.node_index_for_path(&root.join("dir with spaces"));
        assert!(dir_index.is_some(), "should find directory with spaces");

        let file_index = cache.node_index_for_path(&root.join("dir with spaces/file (1).txt"));
        assert!(
            file_index.is_some(),
            "should find file with special characters"
        );
    }

    #[test]
    fn walk_it_without_root_chain_comparison() {
        let temp_dir =
            TempDir::new("walk_without_root_chain").expect("Failed to create temp directory");
        let root = temp_dir.path();
        fs::create_dir_all(root.join("test_dir")).expect("Failed to create directory");

        let walk_data = WalkData::simple(root, true);

        // Test walk_it_without_root_chain
        let tree_without_chain = walk_it_without_root_chain(&walk_data).expect("walk succeeded");
        assert_eq!(
            &*tree_without_chain.name,
            root.file_name().unwrap().to_str().unwrap(),
            "root node should match directory name"
        );

        // Test walk_it (with root chain)
        let tree_with_chain = walk_it(&walk_data).expect("walk succeeded");
        assert_eq!(
            &*tree_with_chain.name, "/",
            "root of chain should be filesystem root"
        );

        // Navigate down the chain to find the actual root directory
        let root_name = root.file_name().unwrap().to_str().unwrap();

        // Traverse to find the target directory in the chain
        let mut found_root = false;
        fn find_in_tree<'a>(node: &'a Node, name: &str) -> Option<&'a Node> {
            if &*node.name == name {
                return Some(node);
            }
            for child in &node.children {
                if let Some(found) = find_in_tree(child, name) {
                    return Some(found);
                }
            }
            None
        }

        if let Some(target_node) = find_in_tree(&tree_with_chain, root_name) {
            found_root = true;
            // This node should have same structure as tree_without_chain
            assert_eq!(&*target_node.name, &*tree_without_chain.name);
        }

        assert!(found_root, "should find target directory in parent chain");
    }

    #[test]
    fn node_path_edge_cases() {
        let temp_dir = TempDir::new("node_path_edge").expect("Failed to create temp directory");
        let root = temp_dir.path();

        let cache = SearchCache::walk_fs(root);

        // Test root node path
        let root_index = cache.file_nodes.root();
        let root_path = cache.node_path(root_index);
        assert!(root_path.is_some(), "root should have a path");
        assert_eq!(
            root_path.unwrap(),
            PathBuf::from("/"),
            "root path should be '/'"
        );
    }

    #[test]
    fn create_node_chain_intermediate_nodes_have_correct_parents() {
        let temp_dir =
            TempDir::new("create_node_chain_parents").expect("Failed to create temp directory");
        let root = temp_dir.path();
        fs::create_dir_all(root.join("a")).expect("Failed to create directory");

        let mut cache = SearchCache::walk_fs(root);

        // Create a chain with multiple missing levels
        let target = root.join("a/b/c/d");
        cache.create_node_chain(&target);

        // Verify parent relationships
        let d_index = cache.node_index_for_path(&target).expect("d should exist");
        let c_index = cache
            .node_index_for_path(&root.join("a/b/c"))
            .expect("c should exist");
        let b_index = cache
            .node_index_for_path(&root.join("a/b"))
            .expect("b should exist");

        // Check parent pointers
        assert_eq!(
            cache.file_nodes[d_index].parent(),
            Some(c_index),
            "d's parent should be c"
        );
        assert_eq!(
            cache.file_nodes[c_index].parent(),
            Some(b_index),
            "c's parent should be b"
        );
    }

    #[test]
    fn test_handle_fs_event_add() {
        // Create a temporary directory.
        let temp_dir = TempDir::new("test_events").expect("Failed to create temp directory");
        let temp_path = temp_dir.path();

        let mut cache = SearchCache::walk_fs(temp_dir.path());

        assert_eq!(cache.file_nodes.len(), 1 + depth(temp_path));
        assert_eq!(cache.name_index.len(), 1 + depth(temp_path));

        fs::File::create(temp_path.join("new_file.txt")).expect("Failed to create file");

        let mock_events = vec![FsEvent {
            path: temp_path.join("new_file.txt"),
            id: cache.last_event_id + 1,
            flag: EventFlag::ItemCreated,
        }];

        cache.handle_fs_events(mock_events).unwrap();

        assert_eq!(cache.file_nodes.len(), 2 + depth(temp_path));
        assert_eq!(cache.name_index.len(), 2 + depth(temp_path));
        assert_eq!(cache.search("new_file.txt").unwrap().len(), 1);
    }

    #[test]
    fn test_handle_fs_event_add_before_search() {
        let temp_dir = TempDir::new("test_events").expect("Failed to create temp directory");
        let temp_path = temp_dir.path();
        fs::File::create(temp_path.join("new_file.txt")).expect("Failed to create file");

        let mut cache = SearchCache::walk_fs(temp_dir.path());

        assert_eq!(cache.file_nodes.len(), 2 + depth(temp_path));
        assert_eq!(cache.name_index.len(), 2 + depth(temp_path));

        let mock_events = vec![FsEvent {
            path: temp_path.join("new_file.txt"),
            id: cache.last_event_id + 1,
            flag: EventFlag::ItemCreated,
        }];

        cache.handle_fs_events(mock_events).unwrap();

        assert_eq!(cache.file_nodes.len(), 2 + depth(temp_path));
        assert_eq!(cache.name_index.len(), 2 + depth(temp_path));
        assert_eq!(cache.search("new_file.txt").unwrap().len(), 1);
    }

    // Processing outdated fs event is required to avoid bouncing.
    #[test]
    fn test_handle_outdated_fs_event() {
        let temp_dir = TempDir::new("test_events").expect("Failed to create temp directory");
        let temp_path = temp_dir.path();

        let mut cache = SearchCache::walk_fs(temp_dir.path());

        assert_eq!(cache.file_nodes.len(), 1 + depth(temp_path));
        assert_eq!(cache.name_index.len(), 1 + depth(temp_path));

        fs::File::create(temp_path.join("new_file.txt")).expect("Failed to create file");

        let mock_events = vec![FsEvent {
            path: temp_path.join("new_file.txt"),
            id: cache.last_event_id.saturating_sub(1),
            flag: EventFlag::ItemCreated,
        }];

        cache.handle_fs_events(mock_events).unwrap();

        assert_eq!(cache.file_nodes.len(), 2 + depth(temp_path));
        assert_eq!(cache.name_index.len(), 2 + depth(temp_path));
        assert_eq!(cache.search("new_file.txt").unwrap().len(), 1);
    }

    #[test]
    fn test_search_with_regex_query() {
        let temp_dir = TempDir::new("test_search_regex_query").unwrap();
        let dir = temp_dir.path();

        fs::File::create(dir.join("foo123.txt")).unwrap();
        fs::File::create(dir.join("bar.txt")).unwrap();

        let mut cache = SearchCache::walk_fs(dir);
        let indices = cache.search("regex:foo\\d+").unwrap();
        assert_eq!(indices.len(), 1);
        let nodes = cache.expand_file_nodes(&indices);
        assert_eq!(nodes.len(), 1);
        assert!(nodes[0].path.ends_with("foo123.txt"));

        // ensure other names are not matched
        let miss = cache.search("regex:bar\\d+").unwrap();
        assert!(miss.is_empty());
    }

    #[test]
    fn and_with_not_propagates_cancellation() {
        let temp_dir = TempDir::new("and_with_not_propagates_cancellation").unwrap();
        let dir = temp_dir.path();

        fs::File::create(dir.join("foo.txt")).unwrap();
        fs::File::create(dir.join("bar.txt")).unwrap();

        let mut cache = SearchCache::walk_fs(dir);
        let token = CancellationToken::new(10);
        let _ = CancellationToken::new(11); // cancel previous token

        let result = cache.search_with_options(
            "bar !foo",
            SearchOptions {
                case_insensitive: false,
            },
            token,
        );
        assert!(matches!(result, Ok(SearchOutcome { nodes: None, .. })));
    }

    #[test]
    fn test_search_case_insensitive_option() {
        let temp_dir = TempDir::new("test_search_case_insensitive_option").unwrap();
        let dir = temp_dir.path();

        fs::File::create(dir.join("Alpha.TXT")).unwrap();
        fs::File::create(dir.join("beta.txt")).unwrap();

        let mut cache = SearchCache::walk_fs(dir);
        let opts = SearchOptions {
            case_insensitive: true,
        };
        let indices =
            guard_indices(cache.search_with_options("alpha.txt", opts, CancellationToken::noop()));
        assert_eq!(indices.len(), 1);
        let nodes = cache.expand_file_nodes(&indices);
        assert_eq!(nodes.len(), 1);
        assert!(nodes[0].path.ends_with("Alpha.TXT"));

        let opts = SearchOptions {
            case_insensitive: true,
        };
        let miss =
            guard_indices(cache.search_with_options("gamma.txt", opts, CancellationToken::noop()));
        assert!(miss.is_empty());
    }

    #[test]
    fn test_wildcard_search_case_sensitivity() {
        let temp_dir = TempDir::new("test_wildcard_search_case_sensitivity").unwrap();
        let dir = temp_dir.path();

        fs::File::create(dir.join("AlphaOne.md")).unwrap();
        fs::File::create(dir.join("alphaTwo.md")).unwrap();
        fs::File::create(dir.join("beta.txt")).unwrap();

        let mut cache = SearchCache::walk_fs(dir);

        let opts = SearchOptions {
            case_insensitive: false,
        };
        let indices =
            guard_indices(cache.search_with_options("alpha*.md", opts, CancellationToken::noop()));
        let nodes = cache.expand_file_nodes(&indices);
        assert_eq!(nodes.len(), 1);
        assert!(nodes[0].path.ends_with("alphaTwo.md"));

        let opts = SearchOptions {
            case_insensitive: true,
        };
        let indices =
            guard_indices(cache.search_with_options("alpha*.md", opts, CancellationToken::noop()));
        let nodes = cache.expand_file_nodes(&indices);
        assert_eq!(nodes.len(), 2);
        assert!(nodes.iter().any(|node| node.path.ends_with("AlphaOne.md")));
        assert!(nodes.iter().any(|node| node.path.ends_with("alphaTwo.md")));
    }

    #[test]
    fn content_filter_matches_file_bodies() {
        let temp_dir = TempDir::new("content_filter_matches_file_bodies").unwrap();
        let dir = temp_dir.path();

        fs::write(dir.join("notes.txt"), b"rust memchr finder\nsecond line").unwrap();
        fs::write(dir.join("other.txt"), b"nothing to see here").unwrap();

        let mut cache = SearchCache::walk_fs(dir);
        let opts = SearchOptions {
            case_insensitive: false,
        };
        let indices = guard_indices(cache.search_with_options(
            "content:memchr",
            opts,
            CancellationToken::noop(),
        ));
        assert_eq!(indices.len(), 1);
        let nodes = cache.expand_file_nodes(&indices);
        assert!(nodes[0].path.ends_with("notes.txt"));

        let opts = SearchOptions {
            case_insensitive: true,
        };
        let insensitive = guard_indices(cache.search_with_options(
            "content:MEMCHR",
            opts,
            CancellationToken::noop(),
        ));
        assert_eq!(insensitive.len(), 1);
        let nodes = cache.expand_file_nodes(&insensitive);
        assert!(nodes[0].path.ends_with("notes.txt"));
    }

    #[test]
    fn content_filter_matches_across_chunks() {
        let temp_dir = TempDir::new("content_filter_matches_across_chunks").unwrap();
        let dir = temp_dir.path();

        let mut payload = vec![b'a'; CONTENT_BUFFER_BYTES.saturating_sub(1)];
        payload.extend_from_slice(b"XYZ");
        payload.extend(std::iter::repeat_n(b'a', 32));
        fs::write(dir.join("large.bin"), &payload).unwrap();

        let mut cache = SearchCache::walk_fs(dir);
        let opts = SearchOptions {
            case_insensitive: false,
        };
        let indices = guard_indices(cache.search_with_options(
            "content:XYZ",
            opts,
            CancellationToken::noop(),
        ));
        assert_eq!(indices.len(), 1);
        let nodes = cache.expand_file_nodes(&indices);
        assert!(nodes.iter().any(|node| node.path.ends_with("large.bin")));
    }

    #[test]
    fn content_filter_single_byte_respects_case_sensitivity() {
        let temp_dir =
            TempDir::new("content_filter_single_byte_respects_case_sensitivity").unwrap();
        let dir = temp_dir.path();

        fs::write(dir.join("letters.txt"), b"AaBb").unwrap();

        let mut cache = SearchCache::walk_fs(dir);

        let insensitive = guard_indices(cache.search_with_options(
            "content:a",
            SearchOptions {
                case_insensitive: true,
            },
            CancellationToken::noop(),
        ));
        assert_eq!(insensitive.len(), 1);

        let sensitive = guard_indices(cache.search_with_options(
            "content:a",
            SearchOptions {
                case_insensitive: false,
            },
            CancellationToken::noop(),
        ));
        assert_eq!(sensitive.len(), 1); // File contains lowercase 'a'

        // But searching for uppercase 'A' case-sensitively should also work
        let sensitive_upper = guard_indices(cache.search_with_options(
            "content:A",
            SearchOptions {
                case_insensitive: false,
            },
            CancellationToken::noop(),
        ));
        assert_eq!(sensitive_upper.len(), 1);

        // Searching for 'z' should return nothing
        let no_match = guard_indices(cache.search_with_options(
            "content:z",
            SearchOptions {
                case_insensitive: false,
            },
            CancellationToken::noop(),
        ));
        assert!(no_match.is_empty());
    }

    #[test]
    fn content_filter_matches_at_buffer_boundary() {
        let temp_dir = TempDir::new("content_filter_matches_at_buffer_boundary").unwrap();
        let dir = temp_dir.path();

        // Place the needle so that it starts at the final byte of the first buffer
        // read and finishes in the next read.
        let mut payload = vec![b'a'; CONTENT_BUFFER_BYTES.saturating_sub(1)];
        payload.push(b'X'); // last byte of first chunk
        payload.extend_from_slice(b"YZ"); // spans into second chunk
        payload.extend(std::iter::repeat_n(b'a', 32));
        fs::write(dir.join("boundary.bin"), &payload).unwrap();

        let mut cache = SearchCache::walk_fs(dir);
        let indices = guard_indices(cache.search_with_options(
            "content:XYZ",
            SearchOptions {
                case_insensitive: false,
            },
            CancellationToken::noop(),
        ));
        assert_eq!(indices.len(), 1);
        let nodes = cache.expand_file_nodes(&indices);
        assert!(nodes.iter().any(|node| node.path.ends_with("boundary.bin")));
    }

    #[test]
    fn content_filter_matches_when_needle_near_buffer_size() {
        let temp_dir = TempDir::new("content_filter_matches_when_needle_near_buffer_size").unwrap();
        let dir = temp_dir.path();

        // Build a long needle that exceeds the base buffer size, then place it once.
        let needle_len = CONTENT_BUFFER_BYTES + 8;
        let needle: String = std::iter::repeat_n('N', needle_len).collect();
        let needle_bytes = needle.as_bytes();

        let mut payload = Vec::new();
        payload.extend(std::iter::repeat_n(b'a', 16));
        payload.extend_from_slice(needle_bytes);
        payload.extend(std::iter::repeat_n(b'b', 16));
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
        let nodes = cache.expand_file_nodes(&indices);
        assert!(
            nodes
                .iter()
                .any(|node| node.path.ends_with("long_needle.bin"))
        );
    }

    #[test]
    fn test_search_empty_cancelled_returns_none() {
        let temp_dir = TempDir::new("search_empty_cancelled").unwrap();
        fs::File::create(temp_dir.path().join("alpha.txt")).unwrap();
        let cache = SearchCache::walk_fs(temp_dir.path());

        let token = CancellationToken::new(1000);
        let _ = CancellationToken::new(1001);

        assert!(cache.search_empty(token).is_none());
    }

    #[test]
    fn test_search_with_options_cancelled_returns_none() {
        let temp_dir = TempDir::new("search_with_options_cancelled").unwrap();
        fs::File::create(temp_dir.path().join("file_a.txt")).unwrap();
        let mut cache = SearchCache::walk_fs(temp_dir.path());

        let token = CancellationToken::new(2000);
        let _ = CancellationToken::new(2001);

        let result = cache.search_with_options(
            "file_a",
            SearchOptions {
                case_insensitive: false,
            },
            token,
        );
        assert!(matches!(result, Ok(SearchOutcome { nodes: None, .. })));
    }

    #[test]
    fn test_query_files_cancelled_returns_none() {
        let temp_dir = TempDir::new("query_files_cancelled").unwrap();
        fs::File::create(temp_dir.path().join("item.txt")).unwrap();
        let mut cache = SearchCache::walk_fs(temp_dir.path());

        let token = CancellationToken::new(3000);
        let _ = CancellationToken::new(3001);

        let result = cache.query_files("item.txt".to_string(), token);
        assert!(matches!(result, Ok(None)));
    }

    #[test]
    fn test_handle_fs_event_removal() {
        let temp_dir = TempDir::new("test_events").expect("Failed to create temp directory");
        let temp_path = temp_dir.path();
        fs::File::create(temp_path.join("new_file.txt")).expect("Failed to create file");

        let mut cache = SearchCache::walk_fs(temp_dir.path());

        assert_eq!(cache.file_nodes.len(), 2 + depth(temp_path));
        assert_eq!(cache.name_index.len(), 2 + depth(temp_path));

        fs::remove_file(temp_path.join("new_file.txt")).expect("Failed to remove file");

        let mock_events = vec![FsEvent {
            path: temp_path.join("new_file.txt"),
            id: cache.last_event_id + 1,
            flag: EventFlag::ItemRemoved,
        }];

        cache.handle_fs_events(mock_events).unwrap();

        // Though the file in fsevents removed, we should still preserve it since it exists on disk.
        assert_eq!(cache.file_nodes.len(), 1 + depth(temp_path));
        assert_eq!(cache.name_index.len(), 1 + depth(temp_path));
        assert_eq!(cache.search("new_file.txt").unwrap().len(), 0);
    }

    #[test]
    #[ignore]
    fn test_handle_fs_event_simulator() {
        let instant = std::time::Instant::now();
        let root = Path::new("/Library/Developer/CoreSimulator");
        let mut cache = SearchCache::walk_fs(root);
        let mut event_id = cache.last_event_id + 1;
        println!(
            "Cache size: {}, process time: {:?}",
            cache.file_nodes.len(),
            instant.elapsed()
        );
        // test speed of handling fs event
        loop {
            let instant = std::time::Instant::now();
            let mock_events = vec![FsEvent {
                path: PathBuf::from("/Library/Developer/CoreSimulator/Volumes/iOS_23A343"),
                id: event_id,
                flag: EventFlag::ItemCreated,
            }];

            cache.handle_fs_events(mock_events).unwrap();
            event_id += 1;
            println!(
                "Event id: {}, process time: {:?}",
                cache.last_event_id,
                instant.elapsed()
            );
        }
    }

    #[test]
    fn test_handle_fs_event_removal_fake() {
        let temp_dir = TempDir::new("test_events").expect("Failed to create temp directory");
        let temp_path = temp_dir.path();
        let mut cache = SearchCache::walk_fs(temp_dir.path());

        assert_eq!(cache.file_nodes.len(), 1 + depth(temp_path));
        assert_eq!(cache.name_index.len(), 1 + depth(temp_path));

        fs::File::create(temp_path.join("new_file.txt")).expect("Failed to create file");

        let mock_events = vec![FsEvent {
            path: temp_path.join("new_file.txt"),
            id: cache.last_event_id + 1,
            flag: EventFlag::ItemRemoved,
        }];

        cache.handle_fs_events(mock_events).unwrap();

        // Though the file in fsevents removed, we should still preserve it since it exists on disk.
        assert_eq!(cache.file_nodes.len(), 2 + depth(temp_path));
        assert_eq!(cache.name_index.len(), 2 + depth(temp_path));
        assert_eq!(cache.search("new_file.txt").unwrap().len(), 1);
    }

    #[test]
    fn test_handle_fs_event_add_and_removal() {
        let temp_dir = TempDir::new("test_events").expect("Failed to create temp directory");
        let temp_path = temp_dir.path();
        let mut cache = SearchCache::walk_fs(temp_dir.path());

        assert_eq!(cache.file_nodes.len(), 1 + depth(temp_path));
        assert_eq!(cache.name_index.len(), 1 + depth(temp_path));

        fs::File::create(temp_path.join("new_file.txt")).expect("Failed to create file");

        let mock_events = vec![
            FsEvent {
                path: temp_path.join("new_file.txt"),
                id: cache.last_event_id + 1,
                flag: EventFlag::ItemCreated,
            },
            FsEvent {
                path: temp_path.join("new_file.txt"),
                id: cache.last_event_id + 1,
                flag: EventFlag::ItemRemoved,
            },
        ];

        cache.handle_fs_events(mock_events).unwrap();

        // Though the file in fsevents removed, we should still preserve it since it exists on disk.
        assert_eq!(cache.file_nodes.len(), 2 + depth(temp_path));
        assert_eq!(cache.name_index.len(), 2 + depth(temp_path));
        assert_eq!(cache.search("new_file.txt").unwrap().len(), 1);
    }

    #[test]
    fn test_handle_fs_event_rescan0() {
        let temp_dir = TempDir::new("test_events").expect("Failed to create temp directory");
        let temp_path = temp_dir.path();
        fs::File::create(temp_path.join("new_file.txt")).expect("Failed to create file");
        fs::File::create(temp_path.join("new_file2.txt")).expect("Failed to create file");
        fs::File::create(temp_path.join("new_file3.txt")).expect("Failed to create file");
        fs::create_dir_all(temp_path.join("src/foo")).expect("Failed to create dir");
        fs::File::create(temp_path.join("src/foo/good.rs")).expect("Failed to create file");
        let mut cache = SearchCache::walk_fs(temp_dir.path());

        assert_eq!(cache.file_nodes.len(), 7 + depth(temp_path));
        assert_eq!(cache.name_index.len(), 7 + depth(temp_path));

        let mock_events = vec![FsEvent {
            path: temp_path.to_path_buf(),
            id: cache.last_event_id + 1,
            flag: EventFlag::RootChanged,
        }];

        cache.handle_fs_events(mock_events).unwrap_err();

        assert_eq!(cache.file_nodes.len(), 7 + depth(temp_path));
        assert_eq!(cache.name_index.len(), 7 + depth(temp_path));
        assert_eq!(cache.search("new_file").unwrap().len(), 3);
        assert_eq!(cache.search("good.rs").unwrap().len(), 1);
        assert_eq!(cache.search("foo").unwrap().len(), 1);
    }

    #[test]
    fn test_handle_fs_event_rescan1() {
        let temp_dir = TempDir::new("test_events").expect("Failed to create temp directory");
        let temp_path = temp_dir.path();
        let mut cache = SearchCache::walk_fs(temp_dir.path());

        assert_eq!(cache.file_nodes.len(), 1 + depth(temp_path));
        assert_eq!(cache.name_index.len(), 1 + depth(temp_path));

        fs::File::create(temp_path.join("new_file.txt")).expect("Failed to create file");
        fs::File::create(temp_path.join("new_file2.txt")).expect("Failed to create file");
        fs::File::create(temp_path.join("new_file3.txt")).expect("Failed to create file");
        fs::create_dir_all(temp_path.join("src/foo")).expect("Failed to create dir");
        fs::File::create(temp_path.join("src/foo/good.rs")).expect("Failed to create file");

        let mock_events = vec![FsEvent {
            path: temp_path.to_path_buf(),
            id: cache.last_event_id + 1,
            flag: EventFlag::RootChanged,
        }];

        cache.handle_fs_events(mock_events).unwrap_err();

        // Rescan is required
        assert_eq!(cache.file_nodes.len(), 1 + depth(temp_path));
        assert_eq!(cache.name_index.len(), 1 + depth(temp_path));
    }

    #[test]
    fn test_handle_fs_event_rescan_by_modify() {
        let temp_dir = TempDir::new("test_events").expect("Failed to create temp directory");
        let temp_path = temp_dir.path();
        let mut cache = SearchCache::walk_fs(temp_dir.path());

        assert_eq!(cache.file_nodes.len(), 1 + depth(temp_path));
        assert_eq!(cache.name_index.len(), 1 + depth(temp_path));

        fs::File::create(temp_path.join("new_file.txt")).expect("Failed to create file");
        fs::File::create(temp_path.join("new_file2.txt")).expect("Failed to create file");
        fs::File::create(temp_path.join("new_file3.txt")).expect("Failed to create file");
        fs::create_dir_all(temp_path.join("src/foo")).expect("Failed to create dir");
        fs::File::create(temp_path.join("src/foo/good.rs")).expect("Failed to create file");

        let mock_events = vec![FsEvent {
            path: temp_path.to_path_buf(),
            id: cache.last_event_id + 1,
            flag: EventFlag::ItemModified,
        }];

        cache.handle_fs_events(mock_events).unwrap_err();

        assert_eq!(cache.file_nodes.len(), 1 + depth(temp_path));
        assert_eq!(cache.name_index.len(), 1 + depth(temp_path));
    }

    #[test]
    fn test_handle_fs_event_dir_removal0() {
        let temp_dir = TempDir::new("test_events").expect("Failed to create temp directory");
        let temp_path = temp_dir.path();
        fs::create_dir_all(temp_path.join("Cargo.toml")).expect("Failed to create file");
        fs::File::create(temp_path.join("new_file.txt")).expect("Failed to create file");
        fs::File::create(temp_path.join("new_file2.txt")).expect("Failed to create file");
        fs::File::create(temp_path.join("new_file3.txt")).expect("Failed to create file");
        fs::create_dir_all(temp_path.join("src/foo")).expect("Failed to create dir");
        fs::File::create(temp_path.join("src/foo/good.rs")).expect("Failed to create file");
        fs::File::create(temp_path.join("src/foo.rs")).expect("Failed to create file");
        fs::File::create(temp_path.join("src/lib.rs")).expect("Failed to create file");
        fs::File::create(temp_path.join("src/boo.rs")).expect("Failed to create file");
        let mut cache = SearchCache::walk_fs(temp_dir.path());

        assert_eq!(cache.file_nodes.len(), 11 + depth(temp_path));
        assert_eq!(cache.name_index.len(), 11 + depth(temp_path));
        assert_eq!(cache.search("src").unwrap().len(), 1);
        assert_eq!(cache.search("new_file").unwrap().len(), 3);
        assert_eq!(cache.search("good.rs").unwrap().len(), 1);
        assert_eq!(cache.search("foo").unwrap().len(), 2);
        assert_eq!(cache.search("oo.rs/").unwrap().len(), 2);
        assert_eq!(cache.search("oo").unwrap().len(), 4);

        fs::remove_dir_all(temp_path.join("src")).expect("Failed to remove dir");

        let mock_events = vec![FsEvent {
            path: temp_path.join("src"),
            id: cache.last_event_id + 1,
            flag: EventFlag::ItemRemoved | EventFlag::ItemIsDir,
        }];

        cache.handle_fs_events(mock_events).unwrap();

        assert_eq!(cache.file_nodes.len(), 5 + depth(temp_path));
        assert_eq!(cache.name_index.len(), 5 + depth(temp_path));
        assert_eq!(cache.search("src").unwrap().len(), 0);
        assert_eq!(cache.search("new_file").unwrap().len(), 3);
        assert_eq!(cache.search("good.rs").unwrap().len(), 0);
        assert_eq!(cache.search("foo").unwrap().len(), 0);
        assert_eq!(cache.search("/foo").unwrap().len(), 0);
    }

    #[test]
    fn test_handle_fs_event_dir_removal_triggered_by_subdir_event() {
        let temp_dir = TempDir::new("test_events").expect("Failed to create temp directory");
        let temp_path = temp_dir.path();
        fs::create_dir_all(temp_path.join("Cargo.toml")).expect("Failed to create file");
        fs::File::create(temp_path.join("new_file.txt")).expect("Failed to create file");
        fs::File::create(temp_path.join("new_file2.txt")).expect("Failed to create file");
        fs::File::create(temp_path.join("new_file3.txt")).expect("Failed to create file");
        fs::create_dir_all(temp_path.join("src/foo")).expect("Failed to create dir");
        fs::File::create(temp_path.join("src/foo/good.rs")).expect("Failed to create file");
        fs::File::create(temp_path.join("src/foo.rs")).expect("Failed to create file");
        fs::File::create(temp_path.join("src/lib.rs")).expect("Failed to create file");
        fs::File::create(temp_path.join("src/boo.rs")).expect("Failed to create file");
        let mut cache = SearchCache::walk_fs(temp_dir.path());

        assert_eq!(cache.file_nodes.len(), 11 + depth(temp_path));
        assert_eq!(cache.name_index.len(), 11 + depth(temp_path));
        assert_eq!(cache.search("src").unwrap().len(), 1);
        assert_eq!(cache.search("new_file").unwrap().len(), 3);
        assert_eq!(cache.search("good.rs").unwrap().len(), 1);
        assert_eq!(cache.search("foo").unwrap().len(), 2);
        assert_eq!(cache.search("oo.rs/").unwrap().len(), 2);
        assert_eq!(cache.search("oo").unwrap().len(), 4);

        fs::remove_dir_all(temp_path.join("src")).expect("Failed to remove dir");

        let mock_events = vec![FsEvent {
            path: temp_path.join("src/foo"),
            id: cache.last_event_id + 1,
            flag: EventFlag::ItemRemoved | EventFlag::ItemIsDir,
        }];

        cache.handle_fs_events(mock_events).unwrap();

        assert_eq!(cache.file_nodes.len(), 9 + depth(temp_path));
        assert_eq!(cache.name_index.len(), 9 + depth(temp_path));
        assert_eq!(cache.search("src").unwrap().len(), 1);
        assert_eq!(cache.search("new_file").unwrap().len(), 3);
        assert_eq!(cache.search("good.rs").unwrap().len(), 0);
        assert_eq!(cache.search("foo").unwrap().len(), 1);
        assert_eq!(cache.search("/foo").unwrap().len(), 1);
        assert_eq!(cache.search("oo.rs/").unwrap().len(), 2);
        assert_eq!(cache.search("oo").unwrap().len(), 2);
    }

    #[test]
    fn test_walk_fs_new_metadata_is_always_none() {
        let temp_dir =
            TempDir::new("test_walk_fs_new_meta").expect("Failed to create temp directory");
        let root_path = temp_dir.path();

        fs::File::create(root_path.join("file1.txt")).expect("Failed to create file1.txt");
        fs::create_dir(root_path.join("subdir1")).expect("Failed to create subdir1");
        fs::File::create(root_path.join("subdir1/file2.txt")).expect("Failed to create file1.txt");

        let mut cache = SearchCache::walk_fs(root_path);

        // Directory nodes should always carry metadata.
        assert!(cache.file_nodes[cache.file_nodes.root()].metadata.is_some());

        // Check metadata for a file node
        let file_nodes = cache
            .search("file1.txt")
            .expect("Search for file1.txt failed");
        assert_eq!(file_nodes.len(), 1, "Expected 1 node for file1.txt");
        let file_node_idx = file_nodes.into_iter().next().unwrap();
        // File nodes should always have `metadata` set to `None`.
        assert!(
            cache.file_nodes[file_node_idx].metadata.is_none(),
            "Metadata for file node created by walk_fs_new should be None"
        );

        // Check metadata for a file node
        let file_nodes = cache
            .search("file2.txt")
            .expect("Search for file1.txt failed");
        assert_eq!(file_nodes.len(), 1);
        let file_node_idx = file_nodes.into_iter().next().unwrap();
        // File nodes should always have `metadata` set to `None`.
        assert!(
            cache.file_nodes[file_node_idx].metadata.is_none(),
            "Metadata for file node created by walk_fs_new should be None"
        );

        // Check metadata for a subdirectory node
        let dir_nodes = cache.search("subdir1").expect("Search for subdir1 failed");
        assert_eq!(dir_nodes.len(), 1, "Expected 1 node for subdir1");
        let dir_node_idx = dir_nodes.into_iter().next().unwrap();
        // Directory nodes should always carry metadata.
        assert!(
            cache.file_nodes[dir_node_idx].metadata.is_some(),
            "Metadata for directory node created by walk_fs_new should be Some"
        );
    }

    #[test]
    fn test_handle_fs_events_metadata() {
        let temp_dir = TempDir::new("test_event_meta").expect("Failed to create temp directory");
        let root_path = temp_dir.path();

        fs::File::create(root_path.join("file1.txt")).expect("Failed to create file1.txt");
        fs::create_dir(root_path.join("subdir1")).expect("Failed to create subdir1");
        fs::File::create(root_path.join("subdir1/file2.txt")).expect("Failed to create file1.txt");

        let mut cache = SearchCache::walk_fs(root_path);
        let mut last_event_id = cache.last_event_id();

        let new_file_path = root_path.join("event_file.txt");
        fs::write(&new_file_path, b"heck").expect("Failed to create event_file.txt");

        let new_file_meta_on_disk = fs::symlink_metadata(&new_file_path).unwrap();
        last_event_id += 1;

        let file_event = FsEvent {
            path: new_file_path.clone(),
            id: last_event_id,
            flag: EventFlag::ItemCreated,
        };
        cache.handle_fs_events(vec![file_event]).unwrap();

        let file_nodes = cache
            .search("event_file.txt")
            .expect("Search for event_file.txt failed");
        assert_eq!(
            file_nodes.len(),
            1,
            "Expected 1 node for event_file.txt after event"
        );
        let file_node_idx = file_nodes.into_iter().next().unwrap();
        let file_slab_meta = cache.file_nodes[file_node_idx]
            .metadata
            .as_ref()
            .expect("Metadata for event_file.txt should be populated by event handler");
        assert_eq!(
            file_slab_meta.size(),
            new_file_meta_on_disk.len() as i64,
            "Size mismatch for event_file.txt"
        );
        assert_eq!(file_slab_meta.size(), 4, "Size mismatch for event_file.txt");
        assert!(
            file_slab_meta.mtime().is_some(),
            "mtime should be populated for event_file.txt"
        );

        // Part 2: Event for a newly created directory (should populate metadata for itself and its children)
        let new_subdir_path = root_path.join("event_subdir");
        fs::create_dir(&new_subdir_path).expect("Failed to create event_subdir");

        let file_in_subdir_path = new_subdir_path.join("file_in_event_subdir.txt");
        fs::File::create(&file_in_subdir_path).expect("Failed to create file_in_event_subdir.txt");
        let file_in_subdir_meta_on_disk = fs::symlink_metadata(&file_in_subdir_path).unwrap();
        last_event_id += 1;

        let dir_event = FsEvent {
            path: new_subdir_path.clone(), // Event is for the directory
            id: last_event_id,
            flag: EventFlag::ItemCreated | EventFlag::ItemIsDir,
        };
        cache.handle_fs_events(vec![dir_event]).unwrap();

        // Check metadata for the directory itself
        let dir_nodes = cache
            .search("/event_subdir/")
            .expect("Search for event_subdir failed");
        assert_eq!(
            dir_nodes.len(),
            1,
            "Expected 1 node for event_subdir after event"
        );
        let dir_node_idx = dir_nodes.into_iter().next().unwrap();
        let dir_slab_meta = cache.file_nodes[dir_node_idx]
            .metadata
            .as_ref()
            .expect("Metadata for event_subdir should be populated by event handler");
        assert!(
            dir_slab_meta.mtime().is_some(),
            "mtime should be populated for event_subdir"
        );

        // Check metadata for the file inside the directory
        let file_in_subdir_nodes = cache
            .search("file_in_event_subdir.txt")
            .expect("Search for file_in_event_subdir.txt failed");
        assert_eq!(
            file_in_subdir_nodes.len(),
            1,
            "Expected 1 node for file_in_event_subdir.txt after event"
        );
        let file_in_subdir_node_idx = file_in_subdir_nodes.into_iter().next().unwrap();
        let file_in_subdir_slab_meta = cache.file_nodes[file_in_subdir_node_idx]
            .metadata
            .as_ref()
            .expect("Metadata for file_in_event_subdir.txt should be populated");
        assert_eq!(
            file_in_subdir_slab_meta.size(),
            file_in_subdir_meta_on_disk.len() as i64,
            "Size mismatch for file_in_event_subdir.txt"
        );
        assert!(
            file_in_subdir_slab_meta.mtime().is_some(),
            "mtime should be populated for file_in_event_subdir.txt"
        );
    }

    #[test]
    fn test_query_files_basic_and_no_results() {
        let temp_dir = TempDir::new("test_query_files_basic").unwrap();
        let root_path = temp_dir.path();

        fs::File::create(root_path.join("file_a.txt")).unwrap();
        fs::create_dir(root_path.join("dir_b")).unwrap();
        fs::File::create(root_path.join("dir_b/file_c.md")).unwrap();

        let mut cache = SearchCache::walk_fs(root_path);

        // 1. Query for a specific file
        let results1 = query(&mut cache, "file_a.txt");
        assert_eq!(results1.len(), 1);
        assert!(
            results1[0].path.ends_with("file_a.txt"),
            "Path was: {:?}",
            results1[0].path
        );
        assert!(
            results1[0].metadata.is_none(),
            "File metadata should be None after walk_fs_new"
        );

        // 2. Query for a file in a subdirectory
        let results2 = query(&mut cache, "file_c.md");
        assert_eq!(results2.len(), 1);
        assert!(
            results2[0].path.ends_with("dir_b/file_c.md"),
            "Path was: {:?}",
            results2[0].path
        );
        assert!(results2[0].metadata.is_none());

        // 3. Query for a directory
        let results3 = query(&mut cache, "dir_b");
        assert_eq!(results3.len(), 1);
        assert!(
            results3[0].path.ends_with("dir_b"),
            "Path was: {:?}",
            results3[0].path
        );
        assert!(
            results3[0].metadata.is_some(),
            "Directory metadata should be Some after walk_fs_new"
        );

        // 4. Query with no results
        let results4 = query(&mut cache, "non_existent.zip");
        assert_eq!(results4.len(), 0);
    }

    #[test]
    fn test_query_files_multiple_matches_and_segments() {
        let temp_dir = TempDir::new("test_query_files_multi").unwrap();
        let root_path = temp_dir.path();

        fs::File::create(root_path.join("file_a.txt")).unwrap();
        fs::File::create(root_path.join("another_file_a.log")).unwrap();
        fs::create_dir(root_path.join("dir_b")).unwrap();
        fs::File::create(root_path.join("dir_b/file_c.md")).unwrap();

        let mut cache = SearchCache::walk_fs(root_path);

        // 5. Query matching multiple files (substring)
        let results5 = query(&mut cache, "file_a");
        assert_eq!(
            results5.len(),
            2,
            "Expected to find 'file_a.txt' and 'another_file_a.log'"
        );
        let paths5: Vec<_> = results5.iter().map(|r| r.path.clone()).collect();
        assert!(paths5.iter().any(|p| p.ends_with("file_a.txt")));
        assert!(paths5.iter().any(|p| p.ends_with("another_file_a.log")));

        // 6. Query with multiple segments (path-like search)
        // "dir_b/file_c" should find "dir_b/file_c.md"
        let results6 = query(&mut cache, "dir_b/file_c");
        assert_eq!(results6.len(), 1);
        assert!(
            results6[0].path.ends_with("dir_b/file_c.md"),
            "Path was: {:?}",
            results6[0].path
        );
    }

    #[test]
    fn test_query_files_root_directory() {
        let temp_dir = TempDir::new("test_query_files_root").unwrap();
        let root_path = temp_dir.path();
        fs::File::create(root_path.join("some_file.txt")).unwrap(); // Add a file to make cache non-trivial

        let mut cache = SearchCache::walk_fs(root_path);
        let root_dir_name = root_path.file_name().unwrap().to_str().unwrap();

        let results = query(&mut cache, root_dir_name.to_string());
        assert_eq!(results.len(), 1, "Should find the root directory itself");
        let expected_path = root_path;
        assert_eq!(
            results[0].path, expected_path,
            "Path for root query mismatch. Expected: {:?}, Got: {:?}",
            expected_path, results[0].path
        );
        assert!(
            results[0].metadata.is_some(),
            "Root directory metadata should be Some"
        );
    }

    #[test]
    fn test_query_files_empty_query_string() {
        let temp_dir = TempDir::new("test_query_files_empty_q").unwrap();
        let mut cache = SearchCache::walk_fs(temp_dir.path());
        // Empty queries match everything.
        let result = cache.query_files("".to_string(), CancellationToken::noop());
        assert!(result.is_ok(), "empty query should succeed");
    }

    #[test]
    fn test_query_files_deep_path_construction_and_multi_segment() {
        let temp_dir = TempDir::new("test_query_deep_path").unwrap();
        let root = temp_dir.path();
        let sub1 = root.join("alpha_dir");
        let sub2 = sub1.join("beta_subdir");
        let file_in_sub2 = sub2.join("gamma_file.txt");

        fs::create_dir_all(&sub2).unwrap();
        fs::File::create(&file_in_sub2).unwrap();

        let mut cache = SearchCache::walk_fs(root);

        // Query for the deep file directly
        let results_deep_file = query(&mut cache, "gamma_file.txt");
        assert_eq!(results_deep_file.len(), 1);
        let expected_suffix_deep = "alpha_dir/beta_subdir/gamma_file.txt".to_string();
        assert!(
            results_deep_file[0].path.ends_with(&expected_suffix_deep),
            "Path was: {:?}",
            results_deep_file[0].path
        );

        // Query for intermediate directory
        let results_sub1 = query(&mut cache, "alpha_dir");
        assert_eq!(results_sub1.len(), 1);
        assert!(
            results_sub1[0].path.ends_with("alpha_dir"),
            "Path was: {:?}",
            results_sub1[0].path
        );

        // Query for nested intermediate directory
        let results_sub2 = query(&mut cache, "beta_subdir");
        assert_eq!(results_sub2.len(), 1);
        assert!(
            results_sub2[0].path.ends_with("alpha_dir/beta_subdir"),
            "Path was: {:?}",
            results_sub2[0].path
        );

        // Test multi-segment query for the deep file
        let results_multi_segment = query(&mut cache, "alpha_dir/beta_subdir/gamma_file");
        assert_eq!(results_multi_segment.len(), 1);
        assert!(
            results_multi_segment[0]
                .path
                .ends_with(&expected_suffix_deep),
            "Path was: {:?}",
            results_multi_segment[0].path
        );

        // Test multi-segment query for an intermediate directory
        let results_multi_segment_dir = query(&mut cache, "alpha_dir/beta_subdir");
        assert_eq!(results_multi_segment_dir.len(), 1);
        assert!(
            results_multi_segment_dir[0]
                .path
                .ends_with("alpha_dir/beta_subdir"),
            "Path was: {:?}",
            results_multi_segment_dir[0].path
        );
    }

    #[test]
    fn test_multi_segment_suffix_wildcard_matches_suffix_segments() {
        let temp_dir = TempDir::new("test_query_multi_segment_suffix_wildcard").unwrap();
        let root = temp_dir.path();
        let files = [
            "alpha_direct/mid_shared/goal.txt",
            "src_alpha_dir/mid_shared/goal.txt",
            "beta_segment/mid_shared/goal.txt",
        ];
        for relative in files {
            let full = root.join(relative);
            fs::create_dir_all(full.parent().unwrap()).unwrap();
            fs::File::create(&full).unwrap();
        }

        let mut cache = SearchCache::walk_fs(root);
        let results = query(&mut cache, "alpha*/mid_shared/goal.txt");
        let mut matched: Vec<_> = results
            .into_iter()
            .map(|node| {
                node.path
                    .strip_prefix(root)
                    .expect("path inside temp dir")
                    .to_path_buf()
            })
            .collect();
        matched.sort();

        assert_eq!(matched.len(), 1, "expected one suffix matches");
        let expected_alpha_direct = std::path::Path::new("alpha_direct")
            .join("mid_shared")
            .join("goal.txt");
        assert!(matched.iter().any(|path| path == &expected_alpha_direct));
    }

    #[test]
    fn test_multi_segment_exact_wildcard_matches_middle_segment() {
        let temp_dir = TempDir::new("test_query_multi_segment_exact_wildcard").unwrap();
        let root = temp_dir.path();
        let files = [
            "work-anchor/mid_release/goal.txt",
            "work-anchor/mid_research/goal.txt",
            "work-anchor/mid_exact/goal.txt",
        ];
        for relative in files {
            let full = root.join(relative);
            fs::create_dir_all(full.parent().unwrap()).unwrap();
            fs::File::create(&full).unwrap();
        }

        let mut cache = SearchCache::walk_fs(root);
        let results = query(&mut cache, "anchor/mid_re*/goal.txt");
        let mut matched: Vec<_> = results
            .into_iter()
            .map(|node| {
                node.path
                    .strip_prefix(root)
                    .expect("path inside temp dir")
                    .to_path_buf()
            })
            .collect();
        matched.sort();

        assert_eq!(matched.len(), 2, "expected two middle exact matches");
        let expected_release = std::path::Path::new("work-anchor")
            .join("mid_release")
            .join("goal.txt");
        let expected_research = std::path::Path::new("work-anchor")
            .join("mid_research")
            .join("goal.txt");
        assert!(matched.iter().any(|path| path == &expected_release));
        assert!(matched.iter().any(|path| path == &expected_research));
    }

    #[test]
    fn test_multi_segment_prefix_wildcard_matches_last_segment() {
        let temp_dir = TempDir::new("test_query_multi_segment_prefix_wildcard").unwrap();
        let root = temp_dir.path();
        let files = [
            "work-anchor/mid_release/prefix-target-alpha.txt",
            "work-anchor/mid_release/prefix-target-beta.txt",
            "work-anchor/mid_release/prefix-tool.txt",
        ];
        for relative in files {
            let full = root.join(relative);
            fs::create_dir_all(full.parent().unwrap()).unwrap();
            fs::File::create(&full).unwrap();
        }

        let mut cache = SearchCache::walk_fs(root);
        let results = query(&mut cache, "anchor/mid_release/prefix-target*");
        let mut matched: Vec<_> = results
            .into_iter()
            .map(|node| {
                node.path
                    .strip_prefix(root)
                    .expect("path inside temp dir")
                    .to_path_buf()
            })
            .collect();
        matched.sort();

        assert_eq!(matched.len(), 2, "expected two prefix matches");
        let expected_alpha = std::path::Path::new("work-anchor")
            .join("mid_release")
            .join("prefix-target-alpha.txt");
        let expected_beta = std::path::Path::new("work-anchor")
            .join("mid_release")
            .join("prefix-target-beta.txt");
        assert!(matched.iter().any(|path| path == &expected_alpha));
        assert!(matched.iter().any(|path| path == &expected_beta));
    }

    #[test]
    fn test_boolean_queries() {
        let temp_dir = TempDir::new("test_boolean_queries").unwrap();
        let root = temp_dir.path();
        fs::File::create(root.join("foo.txt")).unwrap();
        fs::File::create(root.join("bar.txt")).unwrap();
        fs::File::create(root.join("foobar.txt")).unwrap();

        let mut cache = SearchCache::walk_fs(root);

        let results_and = query(&mut cache, "foo bar");
        assert_eq!(results_and.len(), 1);
        assert!(
            results_and[0].path.ends_with("foobar.txt"),
            "AND query should keep files matching both terms"
        );

        let mut names_or: Vec<_> = query(&mut cache, "foo|bar")
            .into_iter()
            .filter_map(|node| {
                node.path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
            })
            .collect();
        names_or.sort();
        assert!(
            names_or.contains(&"foo.txt".to_string())
                && names_or.contains(&"bar.txt".to_string())
                && names_or.contains(&"foobar.txt".to_string()),
            "OR query should include any matching term. Found: {names_or:?}"
        );

        let excluded = query(&mut cache, "!foo");
        assert!(
            excluded.iter().all(|node| !node.path.ends_with("foo.txt")),
            "NOT query should exclude foo.txt"
        );
        assert!(
            excluded.iter().any(|node| node.path.ends_with("bar.txt")),
            "NOT query should keep unrelated files"
        );
    }

    #[test]
    fn test_type_filters() {
        let temp_dir = TempDir::new("test_type_filters").unwrap();
        let root = temp_dir.path();
        fs::create_dir(root.join("alpha_dir")).unwrap();
        fs::File::create(root.join("alpha_dir/file_a.txt")).unwrap();
        fs::File::create(root.join("beta.txt")).unwrap();

        let mut cache = SearchCache::walk_fs(root);

        let files = query(&mut cache, "file:beta");
        assert_eq!(files.len(), 1);
        assert!(files[0].path.ends_with("beta.txt"));

        let folders = query(&mut cache, "folder:alpha");
        assert_eq!(folders.len(), 1);
        assert!(folders[0].path.ends_with("alpha_dir"));
    }

    #[test]
    fn test_extension_and_path_filters() {
        let temp_dir = TempDir::new("test_extension_filters").unwrap();
        let root = temp_dir.path();
        let nested = root.join("nested");
        fs::create_dir(&nested).unwrap();
        fs::File::create(root.join("top.txt")).unwrap();
        fs::File::create(root.join("top.md")).unwrap();
        fs::File::create(nested.join("child.txt")).unwrap();

        let mut cache = SearchCache::walk_fs(root);

        let txt_results = query(&mut cache, "ext:txt");
        let mut txt_paths: Vec<_> = txt_results
            .into_iter()
            .filter_map(|node| {
                node.path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
            })
            .collect();
        txt_paths.sort();
        assert_eq!(
            txt_paths,
            vec!["child.txt".to_string(), "top.txt".to_string()]
        );

        let parent_query = format!(r#"parent:"{}""#, root.to_string_lossy());
        let direct_children = query(&mut cache, parent_query);
        let mut child_names: Vec<_> = direct_children
            .into_iter()
            .filter_map(|node| {
                node.path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
            })
            .collect();
        child_names.sort();
        assert!(
            child_names.contains(&"top.txt".to_string())
                && child_names.contains(&"top.md".to_string()),
            "parent: filter should return direct children"
        );

        let infolder_query = format!(r#"infolder:"{}""#, nested.to_string_lossy());
        let infolder_results = query(&mut cache, infolder_query);
        assert_eq!(infolder_results.len(), 1);
        assert!(infolder_results[0].path.ends_with("nested/child.txt"));
    }

    #[test]
    fn test_query_files_metadata_consistency_after_walk_and_event() {
        let temp_dir = TempDir::new("test_query_meta_consistency").unwrap();
        let root_path = temp_dir.path();

        let file_path_walk = root_path.join("walk_file.txt");
        let dir_path_walk = root_path.join("walk_dir");
        fs::File::create(&file_path_walk).unwrap();
        fs::create_dir(&dir_path_walk).unwrap();

        let mut cache = SearchCache::walk_fs(root_path);

        // Check metadata from initial walk_fs_new
        let results_file_walk = query(&mut cache, "walk_file.txt");
        assert_eq!(results_file_walk.len(), 1);
        assert!(
            results_file_walk[0].metadata.is_none(),
            "File metadata from walk_fs_new should be None"
        );

        let results_dir_walk = query(&mut cache, "walk_dir");
        assert_eq!(results_dir_walk.len(), 1);
        assert!(
            results_dir_walk[0].metadata.is_some(),
            "Directory metadata from walk_fs_new should be Some"
        );

        // Simulate an event for a new file
        let event_file_path = root_path.join("event_added_file.txt");
        fs::write(&event_file_path, "content123").unwrap(); // content of size 10
        let last_event_id = cache.last_event_id();
        let event = FsEvent {
            path: event_file_path.clone(),
            id: last_event_id + 1,
            flag: EventFlag::ItemCreated,
        };
        cache.handle_fs_events(vec![event]).unwrap();

        let results_event_file = query(&mut cache, "event_added_file.txt");
        assert_eq!(results_event_file.len(), 1);
        let event_file_meta = results_event_file[0]
            .metadata
            .as_ref()
            .expect("File metadata should be Some after event processing");
        assert_eq!(event_file_meta.size(), 10);

        // Simulate an event for a new directory with a file in it
        let event_dir_path = root_path.join("event_added_dir");
        fs::create_dir(&event_dir_path).unwrap();
        let file_in_event_dir_path = event_dir_path.join("inner_event.dat");
        fs::write(&file_in_event_dir_path, "data").unwrap(); // content of size 4

        let last_event_id_2 = cache.last_event_id();
        let event_dir = FsEvent {
            path: event_dir_path.clone(), // Event is for the directory
            id: last_event_id_2 + 1,
            flag: EventFlag::ItemCreated | EventFlag::ItemIsDir, // scan_path_recursive will scan children
        };
        cache.handle_fs_events(vec![event_dir]).unwrap();

        let results_event_dir = query(&mut cache, "event_added_dir");
        assert_eq!(results_event_dir.len(), 1);
        assert!(
            results_event_dir[0].metadata.is_some(),
            "Dir metadata should be Some after event processing for dir"
        );

        let results_file_in_event_dir = query(&mut cache, "inner_event.dat");
        assert_eq!(results_file_in_event_dir.len(), 1);
        let inner_file_meta = results_file_in_event_dir[0]
            .metadata
            .as_ref()
            .expect("File in event-added dir metadata should be Some");
        assert_eq!(inner_file_meta.size(), 4);
    }

    // --- scan_paths focused tests ---
    #[test]
    fn test_scan_paths_empty() {
        assert!(scan_paths(vec![]).is_empty());
    }

    #[test]
    fn test_scan_paths_only_rescan_events_kept_when_called_directly() {
        let root = PathBuf::from("/tmp/root");
        let events = vec![FsEvent {
            path: root.clone(),
            id: 1,
            flag: EventFlag::RootChanged,
        }];
        assert!(scan_paths(events).is_empty());
    }

    #[test]
    fn test_scan_paths_history_done_filtered() {
        let p = PathBuf::from("/tmp/a");
        let events = vec![FsEvent {
            path: p,
            id: 1,
            flag: EventFlag::HistoryDone,
        }];
        // HistoryDone => ScanType::Nop
        assert!(scan_paths(events).is_empty());
    }

    #[test]
    fn test_scan_paths_dedup_same_path() {
        let p = PathBuf::from("/tmp/a/b");
        let events = vec![
            FsEvent {
                path: p.clone(),
                id: 1,
                flag: EventFlag::ItemCreated | EventFlag::ItemIsDir,
            },
            FsEvent {
                path: p.clone(),
                id: 2,
                flag: EventFlag::ItemModified | EventFlag::ItemIsFile,
            }, // Assume the flag is incorrect; treat it as SingleNode anyway.
            FsEvent {
                path: p,
                id: 3,
                flag: EventFlag::ItemRemoved | EventFlag::ItemIsFile,
            },
        ];
        let out = scan_paths(events);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0], PathBuf::from("/tmp/a/b"));
    }

    #[test]
    fn test_scan_paths_child_then_parent_collapses() {
        let events = vec![
            FsEvent {
                path: PathBuf::from("/t/a/b/c"),
                id: 1,
                flag: EventFlag::ItemCreated | EventFlag::ItemIsFile,
            },
            FsEvent {
                path: PathBuf::from("/t/a/b"),
                id: 2,
                flag: EventFlag::ItemModified | EventFlag::ItemIsDir,
            },
            FsEvent {
                path: PathBuf::from("/t/a"),
                id: 3,
                flag: EventFlag::ItemModified | EventFlag::ItemIsDir,
            },
        ];
        let out = scan_paths(events);
        // Expect the ancestor /t/a to absorb the whole subtree.
        assert_eq!(out, vec![PathBuf::from("/t/a")]);
    }

    #[test]
    fn test_scan_paths_parent_then_child_skip_child() {
        let events = vec![
            FsEvent {
                path: PathBuf::from("/t/a"),
                id: 1,
                flag: EventFlag::ItemModified | EventFlag::ItemIsDir,
            },
            FsEvent {
                path: PathBuf::from("/t/a/b"),
                id: 2,
                flag: EventFlag::ItemCreated | EventFlag::ItemIsFile,
            },
            FsEvent {
                path: PathBuf::from("/t/a/b/c"),
                id: 3,
                flag: EventFlag::ItemCreated | EventFlag::ItemIsFile,
            },
        ];
        let out = scan_paths(events);
        assert_eq!(out, vec![PathBuf::from("/t/a")]);
    }

    #[test]
    fn test_scan_paths_siblings_all_retained() {
        let events = vec![
            FsEvent {
                path: PathBuf::from("/t/a/x"),
                id: 1,
                flag: EventFlag::ItemCreated | EventFlag::ItemIsFile,
            },
            FsEvent {
                path: PathBuf::from("/t/a/y"),
                id: 2,
                flag: EventFlag::ItemCreated | EventFlag::ItemIsFile,
            },
            FsEvent {
                path: PathBuf::from("/t/a/z"),
                id: 3,
                flag: EventFlag::ItemCreated | EventFlag::ItemIsFile,
            },
        ];
        let mut out = scan_paths(events);
        out.sort();
        assert_eq!(
            out,
            vec!["/t/a/x", "/t/a/y", "/t/a/z"]
                .into_iter()
                .map(PathBuf::from)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_scan_paths_prefix_but_not_parent() {
        // /foo/bar and /foo/barista share a prefix but are not parent/child; both should stay.
        let events = vec![
            FsEvent {
                path: PathBuf::from("/foo/bar"),
                id: 1,
                flag: EventFlag::ItemCreated | EventFlag::ItemIsDir,
            },
            FsEvent {
                path: PathBuf::from("/foo/barista"),
                id: 2,
                flag: EventFlag::ItemCreated | EventFlag::ItemIsDir,
            },
        ];
        let mut out = scan_paths(events);
        out.sort();
        assert_eq!(
            out,
            vec![PathBuf::from("/foo/bar"), PathBuf::from("/foo/barista")]
        );
    }

    #[test]
    fn test_scan_paths_mix_folder_and_single_node() {
        // Directory creation plus file modification: the directory should absorb its child.
        let events = vec![
            FsEvent {
                path: PathBuf::from("/mix/dir/sub/file.txt"),
                id: 1,
                flag: EventFlag::ItemModified | EventFlag::ItemIsFile,
            },
            FsEvent {
                path: PathBuf::from("/mix/dir/sub"),
                id: 2,
                flag: EventFlag::ItemCreated | EventFlag::ItemIsDir,
            },
        ];
        let out = scan_paths(events);
        assert_eq!(out, vec![PathBuf::from("/mix/dir/sub")]);
    }

    #[test]
    fn test_scan_paths_depth_then_lexicographic_ordering() {
        let events = vec![
            FsEvent {
                path: PathBuf::from("/z/child"),
                id: 1,
                flag: EventFlag::ItemCreated | EventFlag::ItemIsFile,
            },
            FsEvent {
                path: PathBuf::from("/a"),
                id: 2,
                flag: EventFlag::ItemCreated | EventFlag::ItemIsDir,
            },
            FsEvent {
                path: PathBuf::from("/m"),
                id: 3,
                flag: EventFlag::ItemCreated | EventFlag::ItemIsDir,
            },
            FsEvent {
                path: PathBuf::from("/a/child"),
                id: 4,
                flag: EventFlag::ItemCreated | EventFlag::ItemIsFile,
            },
        ];
        let out = scan_paths(events);
        assert_eq!(
            out,
            vec![
                PathBuf::from("/a"),
                PathBuf::from("/m"),
                PathBuf::from("/z/child")
            ]
        );
    }

    #[test]
    fn test_scan_paths_handles_root_ancestor() {
        let events = vec![
            FsEvent {
                path: PathBuf::from("/foo/bar"),
                id: 1,
                flag: EventFlag::ItemCreated | EventFlag::ItemIsFile,
            },
            FsEvent {
                path: PathBuf::from("/"),
                id: 2,
                flag: EventFlag::ItemCreated | EventFlag::ItemIsDir,
            },
        ];
        let out = scan_paths(events);
        assert_eq!(out, vec![PathBuf::from("/")]);
    }

    #[test]
    fn test_scan_paths_large_chain_collapse() {
        // Build a long chain where the ancestor arrives at the end.
        let mut events = Vec::new();
        let depth = ["a", "b", "c", "d", "e", "f"];
        for i in 0..depth.len() {
            let path = format!("/long/{}", depth[..=i].join("/"));
            events.push(FsEvent {
                path: PathBuf::from(path),
                id: i as u64,
                flag: EventFlag::ItemCreated | EventFlag::ItemIsDir,
            });
        }
        // Add the real ancestor /long.
        events.push(FsEvent {
            path: PathBuf::from("/long"),
            id: 99,
            flag: EventFlag::ItemModified | EventFlag::ItemIsDir,
        });
        let out = scan_paths(events);
        assert_eq!(out, vec![PathBuf::from("/long")]);
    }
}
