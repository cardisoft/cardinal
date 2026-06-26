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

use crate::{NAME_POOL, PATH_POOL, SlabIndex, SlabNodeMetadataCompact};
use fswalk::NodeFileType;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

/// A single filesystem entry in the flat index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlatEntry {
    /// Interned full absolute path (e.g. `/Users/demo/src/main.rs`).
    pub path: &'static str,
    /// Interned last path segment (e.g. `main.rs`). Derived at index time.
    pub name: &'static str,
    /// The index into the slab (`FileNodes`) for this entry.
    pub slab_index: SlabIndex,
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

    /// Case-insensitive substring match on the path without allocation.
    /// Uses `eq_ignore_ascii_case` on each character for matching.
    pub fn path_match_ci(&self, needle_lower: &str) -> bool {
        let path_bytes = self.path.as_bytes();
        let needle_bytes = needle_lower.as_bytes();
        if needle_bytes.is_empty() {
            return true;
        }
        if needle_bytes.len() > path_bytes.len() {
            return false;
        }
        // Sliding window: check if any substring of path matches needle
        // case-insensitively (ASCII only).
        for i in 0..=(path_bytes.len() - needle_bytes.len()) {
            let mut found = true;
            for (j, &nb) in needle_bytes.iter().enumerate() {
                let pb = path_bytes[i + j];
                // Convert both to lowercase ASCII for comparison
                let pb_lower = pb.to_ascii_lowercase();
                if pb_lower != nb {
                    found = false;
                    break;
                }
            }
            if found {
                return true;
            }
        }
        false
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

/// The flat index: a sorted array of entries plus derived indexes.
#[derive(Debug, Clone, Default)]
pub struct FlatIndex {
    entries: Vec<FlatEntry>,
    /// Maps interned filenames → entry indices (for *.ext / word search).
    pub name_index: FlatNameIndex,
    /// Maps interned full paths → entry index (for path: filter lookups).
    path_map: BTreeMap<&'static str, SlabIndex>,
    /// Maps slab index → entry index in `entries`.
    slab_map: BTreeMap<SlabIndex, usize>,
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

    pub fn get(&self, slab_index: SlabIndex) -> Option<&FlatEntry> {
        self.slab_map
            .get(&slab_index)
            .and_then(|&i| self.entries.get(i))
    }

    /// Get an entry by its position in the sorted entries array.
    pub fn get_by_pos(&self, pos: usize) -> Option<&FlatEntry> {
        self.entries.get(pos)
    }

    pub fn get_mut(&mut self, slab_index: SlabIndex) -> Option<&mut FlatEntry> {
        self.slab_map
            .get(&slab_index)
            .copied()
            .and_then(move |i| self.entries.get_mut(i))
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
        let mut path_map: BTreeMap<&'static str, SlabIndex> = BTreeMap::new();
        let mut slab_map: BTreeMap<SlabIndex, usize> = BTreeMap::new();
        for (i, entry) in entries.iter().enumerate() {
            let idx = SlabIndex::new(i);
            name_map.entry(entry.name).or_default().push(idx);
            path_map.insert(entry.path, idx);
            slab_map.insert(entry.slab_index, i);
        }
        Self {
            entries,
            name_index: FlatNameIndex { map: name_map },
            path_map,
            slab_map,
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
        let end = self.entries[start..].partition_point(|e| e.path.starts_with(prefix)) + start;
        start..end
    }

    pub fn prefix_indices(&self, prefix: &str) -> Vec<SlabIndex> {
        let range = self.prefix_range(prefix);
        range.map(|i| self.entries[i].slab_index).collect()
    }

    pub fn node_path(&self, index: SlabIndex) -> Option<PathBuf> {
        self.get(index).map(|e| PathBuf::from(e.path))
    }

    pub fn node_name(&self, index: SlabIndex) -> Option<&'static str> {
        self.get(index).map(|e| e.name)
    }

    pub fn insert(&mut self, entry: FlatEntry) {
        let pos = self
            .entries
            .partition_point(|e| e.path.as_bytes() < entry.path.as_bytes());
        let slab_idx = entry.slab_index;
        self.entries.insert(pos, entry);
        // Incrementally update indexes instead of full rebuild.
        // Shift entry indices in maps for entries after the insertion point.
        self.name_index
            .map
            .entry(self.entries[pos].name)
            .or_default()
            .push(SlabIndex::new(pos));
        self.path_map
            .insert(self.entries[pos].path, SlabIndex::new(pos));
        self.slab_map.insert(slab_idx, pos);
    }

    pub fn remove(&mut self, slab_index: SlabIndex) -> Option<FlatEntry> {
        let pos = self.slab_map.get(&slab_index).copied()?;
        let entry = self.entries.remove(pos);
        // Incrementally update indexes.
        if let Some(indices) = self.name_index.map.get_mut(entry.name) {
            indices.retain(|&i| i.get() != pos);
        }
        self.path_map.remove(entry.path);
        self.slab_map.remove(&slab_index);
        Some(entry)
    }

    pub fn remove_prefix(&mut self, prefix: &str) -> usize {
        let range = self.prefix_range(prefix);
        let count = range.end - range.start;
        if count > 0 {
            // Remove entries and their index entries.
            for i in range.clone() {
                let entry = &self.entries[i];
                if let Some(indices) = self.name_index.map.get_mut(entry.name) {
                    indices.retain(|&idx| idx.get() != i);
                }
                self.path_map.remove(entry.path);
                self.slab_map.remove(&entry.slab_index);
            }
            self.entries.drain(range);
            // Full rebuild needed after bulk removal to fix shifted indices.
            self.rebuild_indexes();
        }
        count
    }

    fn rebuild_indexes(&mut self) {
        let mut name_map: BTreeMap<&'static str, Vec<SlabIndex>> = BTreeMap::new();
        let mut path_map: BTreeMap<&'static str, SlabIndex> = BTreeMap::new();
        let mut slab_map: BTreeMap<SlabIndex, usize> = BTreeMap::new();
        for (i, entry) in self.entries.iter().enumerate() {
            let idx = SlabIndex::new(i);
            name_map.entry(entry.name).or_default().push(idx);
            path_map.insert(entry.path, idx);
            slab_map.insert(entry.slab_index, i);
        }
        self.name_index = FlatNameIndex { map: name_map };
        self.path_map = path_map;
        self.slab_map = slab_map;
    }

    /// Look up the slab index for an interned full path.
    pub fn get_by_path(&self, path: &str) -> Option<SlabIndex> {
        self.path_map
            .get(path)
            .map(|entry_idx| self.entries[entry_idx.get()].slab_index)
    }
}

/// Build a `FlatEntry` from a full path, slab index, and optional metadata.
pub fn make_flat_entry(
    path: &Path,
    slab_index: SlabIndex,
    metadata: Option<fswalk::NodeMetadata>,
) -> FlatEntry {
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
        slab_index,
        metadata,
    }
}
