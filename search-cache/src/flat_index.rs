//! Flat index for Cardinal's search cache.
//!
//! Instead of a tree of `SlabNode`s with parent pointers, the flat index
//! stores every filesystem entry in a single sorted `Vec`. Full paths are
//! stored directly (not reconstructed from a parent chain), which makes
//! `path:`, `parent:`, `infolder:`, and `nosubfolders:` queries trivial.
//!
//! Two derived indexes are maintained alongside the entry array:
//! - A sorted-by-path array (the entries themselves), enabling prefix range
//!   queries for `parent:` / `infolder:`.
//! - A name index mapping the last path segment (filename) → entry indices,
//!   for `*.ext` and word search — identical to the existing approach.

use crate::{SlabIndex, SlabNodeMetadataCompact, NAME_POOL, PATH_POOL};
use fswalk::NodeFileType;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// A single filesystem entry in the flat index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlatEntry {
    /// Interned full absolute path (e.g. `/Users/demo/src/main.rs`).
    pub path: &'static str,
    /// Interned last path segment (e.g. `main.rs`). Derived at index time.
    pub name: &'static str,
    /// Compact metadata: file type, size, timestamps.
    pub metadata: SlabNodeMetadataCompact,
}

impl FlatEntry {
    pub fn is_dir(&self) -> bool {
        self.metadata.file_type_hint() == NodeFileType::Dir
    }

    pub fn path(&self) -> &Path {
        Path::new(self.path)
    }
}

/// Name index for the flat structure: maps interned filenames → entry indices.
#[derive(Debug, Clone, Default)]
pub struct FlatNameIndex {
    map: BTreeMap<&'static str, Vec<SlabIndex>>,
}

impl FlatNameIndex {
    pub fn get(&self, name: &str) -> Option<&[SlabIndex]> {
        self.map.get(name).map(|v| v.as_slice())
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

/// The flat index: a sorted array of entries plus a name index.
#[derive(Debug, Clone, Default)]
pub struct FlatIndex {
    entries: Vec<FlatEntry>,
    pub name_index: FlatNameIndex,
}

impl FlatIndex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn get(&self, index: SlabIndex) -> Option<&FlatEntry> {
        self.entries.get(index.get())
    }

    pub fn get_mut(&mut self, index: SlabIndex) -> Option<&mut FlatEntry> {
        self.entries.get_mut(index.get())
    }

    pub fn iter(&self) -> impl Iterator<Item = (SlabIndex, &FlatEntry)> {
        self.entries
            .iter()
            .enumerate()
            .map(|(i, e)| (SlabIndex::new(i), e))
    }

    pub fn all_indices(&self) -> Vec<SlabIndex> {
        (0..self.entries.len()).map(SlabIndex::new).collect()
    }

    /// Build from entries already sorted by path.
    pub fn build_from_entries(entries: Vec<FlatEntry>) -> Self {
        let mut name_map: BTreeMap<&'static str, Vec<SlabIndex>> = BTreeMap::new();
        for (i, entry) in entries.iter().enumerate() {
            name_map
                .entry(entry.name)
                .or_default()
                .push(SlabIndex::new(i));
        }
        Self {
            entries,
            name_index: FlatNameIndex { map: name_map },
        }
    }

    /// Range of entries whose path starts with `prefix` — O(log n).
    pub fn prefix_range(&self, prefix: &str) -> std::ops::Range<usize> {
        if self.entries.is_empty() {
            return 0..0;
        }
        let start = self
            .entries
            .partition_point(|e| e.path.as_bytes() < prefix.as_bytes());
        let end = self.entries[start..]
            .partition_point(|e| e.path.starts_with(prefix))
            + start;
        start..end
    }

    pub fn prefix_indices(&self, prefix: &str) -> Vec<SlabIndex> {
        let range = self.prefix_range(prefix);
        (range.start..range.end).map(SlabIndex::new).collect()
    }

    pub fn node_path(&self, index: SlabIndex) -> Option<PathBuf> {
        self.get(index).map(|e| PathBuf::from(e.path))
    }

    pub fn node_name(&self, index: SlabIndex) -> Option<&'static str> {
        self.get(index).map(|e| e.name)
    }

    pub fn insert(&mut self, entry: FlatEntry) -> SlabIndex {
        let pos = self
            .entries
            .partition_point(|e| e.path.as_bytes() < entry.path.as_bytes());
        self.entries.insert(pos, entry);
        self.rebuild_name_index();
        SlabIndex::new(pos)
    }

    pub fn remove(&mut self, index: SlabIndex) -> Option<FlatEntry> {
        if index.get() < self.entries.len() {
            let entry = self.entries.remove(index.get());
            self.rebuild_name_index();
            Some(entry)
        } else {
            None
        }
    }

    pub fn remove_prefix(&mut self, prefix: &str) -> usize {
        let range = self.prefix_range(prefix);
        let count = range.end - range.start;
        if count > 0 {
            self.entries.drain(range);
            self.rebuild_name_index();
        }
        count
    }

    fn rebuild_name_index(&mut self) {
        let mut name_map: BTreeMap<&'static str, Vec<SlabIndex>> = BTreeMap::new();
        for (i, entry) in self.entries.iter().enumerate() {
            name_map
                .entry(entry.name)
                .or_default()
                .push(SlabIndex::new(i));
        }
        self.name_index = FlatNameIndex { map: name_map };
    }
}

/// Build a `FlatEntry` from a full path and optional metadata.
pub fn make_flat_entry(path: &Path, metadata: Option<fswalk::NodeMetadata>) -> FlatEntry {
    let path_str = path.to_string_lossy();
    let interned_path = PATH_POOL.push(path_str.as_ref());
    let name = path
        .file_name()
        .map(|n| NAME_POOL.push(n.to_string_lossy().as_ref()))
        .unwrap_or_else(|| NAME_POOL.push(""));
    let metadata = match metadata {
        Some(m) => SlabNodeMetadataCompact::some(m),
        None => SlabNodeMetadataCompact::none(),
    };
    FlatEntry {
        path: interned_path,
        name,
        metadata,
    }
}
