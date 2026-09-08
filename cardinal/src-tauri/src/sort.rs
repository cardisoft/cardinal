use fswalk::NodeFileType;
use search_cache::{SearchResultNode, SlabIndex, SlabNodeMetadataCompact};
use serde::Deserialize;
use std::{cmp::Ordering as StdOrdering, path::Path};

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SortStatePayload {
    pub key: SortKeyPayload,
    pub direction: SortDirectionPayload,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SortKeyPayload {
    Filename,
    FullPath,
    Size,
    Mtime,
    Ctime,
}

impl SortKeyPayload {
    pub(crate) fn uses_metadata(self) -> bool {
        matches!(self, Self::Size | Self::Mtime | Self::Ctime)
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortDirectionPayload {
    Asc,
    Desc,
}

#[derive(Debug)]
pub(crate) struct SortEntry {
    pub(crate) slab_index: SlabIndex,
    node: SearchResultNode,
    path_key: String,
    name_key: String,
}

impl SortEntry {
    pub(crate) fn new(slab_index: SlabIndex, node: SearchResultNode) -> Self {
        let path_key = normalize_path(&node.path);
        let name_key = extract_filename(&node);
        Self {
            slab_index,
            node,
            path_key,
            name_key,
        }
    }
}

pub(crate) fn sort_entries(entries: &mut [SortEntry], sort: &SortStatePayload) {
    entries.sort_by(|a, b| compare_entries(a, b, sort));
}

pub(crate) fn sort_metadata_indices_cancellable(
    entries: &mut [(SlabIndex, SlabNodeMetadataCompact)],
    sort: &SortStatePayload,
    is_cancelled: impl Fn() -> bool,
) -> bool {
    const CHUNK_SIZE: usize = 16_384;
    if is_cancelled() {
        return false;
    }
    let compare = |(index_a, metadata_a): &(SlabIndex, SlabNodeMetadataCompact),
                   (index_b, metadata_b): &(SlabIndex, SlabNodeMetadataCompact)| {
        let ordering = metadata_numeric(metadata_a, sort.key)
            .cmp(&metadata_numeric(metadata_b, sort.key))
            .then_with(|| index_a.get().cmp(&index_b.get()));
        match sort.direction {
            SortDirectionPayload::Asc => ordering,
            SortDirectionPayload::Desc => ordering.reverse(),
        }
    };

    for chunk in entries.chunks_mut(CHUNK_SIZE) {
        if is_cancelled() {
            return false;
        }
        chunk.sort_unstable_by(&compare);
    }

    let mut source = entries.to_vec();
    let mut target = source.clone();
    let mut width = CHUNK_SIZE;
    while width < source.len() {
        if is_cancelled() {
            return false;
        }
        for start in (0..source.len()).step_by(width.saturating_mul(2)) {
            let middle = (start + width).min(source.len());
            let end = (start + width.saturating_mul(2)).min(source.len());
            let (mut left, mut right, mut output) = (start, middle, start);
            while left < middle && right < end {
                if output.is_multiple_of(CHUNK_SIZE) && is_cancelled() {
                    return false;
                }
                if compare(&source[left], &source[right]) != StdOrdering::Greater {
                    target[output] = source[left];
                    left += 1;
                } else {
                    target[output] = source[right];
                    right += 1;
                }
                output += 1;
            }
            target[output..output + (middle - left)].clone_from_slice(&source[left..middle]);
            output += middle - left;
            target[output..output + (end - right)].clone_from_slice(&source[right..end]);
        }
        std::mem::swap(&mut source, &mut target);
        width = width.saturating_mul(2);
    }
    entries.clone_from_slice(&source);
    !is_cancelled()
}

fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn extract_filename(node: &SearchResultNode) -> String {
    node.path
        .file_name()
        .and_then(|name| name.to_str())
        .map(|x| x.to_string())
        .unwrap_or_else(|| node.path.to_string_lossy().into_owned())
}

fn metadata_numeric(meta: &SlabNodeMetadataCompact, key: SortKeyPayload) -> i64 {
    let Some(meta_ref) = meta.as_ref() else {
        return i64::MIN;
    };
    match key {
        SortKeyPayload::Size => meta_ref.size(),
        SortKeyPayload::Mtime => meta_ref
            .mtime()
            .map(|value| value.get() as i64)
            .unwrap_or(i64::MIN),
        SortKeyPayload::Ctime => meta_ref
            .ctime()
            .map(|value| value.get() as i64)
            .unwrap_or(i64::MIN),
        SortKeyPayload::FullPath | SortKeyPayload::Filename => 0,
    }
}

fn type_order(node: &SearchResultNode) -> u8 {
    match node.metadata.as_ref().map(|m| m.r#type()) {
        Some(NodeFileType::Dir) => 0,
        None => 2,
        _ => 1,
    }
}

fn compare_entries(a: &SortEntry, b: &SortEntry, sort: &SortStatePayload) -> StdOrdering {
    let ordering = match sort.key {
        SortKeyPayload::FullPath => a
            .path_key
            .cmp(&b.path_key)
            .then_with(|| a.name_key.cmp(&b.name_key))
            .then_with(|| type_order(&a.node).cmp(&type_order(&b.node))),
        SortKeyPayload::Filename => a
            .name_key
            .cmp(&b.name_key)
            .then_with(|| type_order(&a.node).cmp(&type_order(&b.node)))
            .then_with(|| a.path_key.cmp(&b.path_key)),
        SortKeyPayload::Size | SortKeyPayload::Mtime | SortKeyPayload::Ctime => {
            metadata_numeric(&a.node.metadata, sort.key)
                .cmp(&metadata_numeric(&b.node.metadata, sort.key))
                .then_with(|| a.name_key.cmp(&b.name_key))
                .then_with(|| type_order(&a.node).cmp(&type_order(&b.node)))
                .then_with(|| a.path_key.cmp(&b.path_key))
        }
    };

    match sort.direction {
        SortDirectionPayload::Asc => ordering,
        SortDirectionPayload::Desc => ordering.reverse(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fswalk::NodeMetadata;
    use std::path::PathBuf;

    fn entry_with_metadata(
        slab_index: usize,
        path: &str,
        metadata: SlabNodeMetadataCompact,
    ) -> SortEntry {
        let node = SearchResultNode {
            path: PathBuf::from(path),
            metadata,
        };

        SortEntry::new(SlabIndex::new(slab_index), node)
    }

    fn metadata_with_type(r#type: NodeFileType, size: u64) -> SlabNodeMetadataCompact {
        SlabNodeMetadataCompact::some(NodeMetadata {
            r#type,
            size,
            ctime: None,
            mtime: None,
        })
    }

    fn metadata_with_dates(
        r#type: NodeFileType,
        size: u64,
        ctime: u64,
        mtime: u64,
    ) -> SlabNodeMetadataCompact {
        SlabNodeMetadataCompact::some(NodeMetadata {
            r#type,
            size,
            ctime: std::num::NonZeroU64::new(ctime),
            mtime: std::num::NonZeroU64::new(mtime),
        })
    }

    #[test]
    fn filename_sort_keeps_directories_before_files() {
        let sort_state = SortStatePayload {
            key: SortKeyPayload::Filename,
            direction: SortDirectionPayload::Asc,
        };
        let mut entries = vec![
            entry_with_metadata(
                1,
                "/tmp/b/foo.txt",
                metadata_with_type(NodeFileType::File, 0),
            ),
            entry_with_metadata(2, "/tmp/c/foo.txt", SlabNodeMetadataCompact::none()),
            entry_with_metadata(
                0,
                "/tmp/a/foo.txt",
                metadata_with_type(NodeFileType::Dir, 0),
            ),
        ];

        sort_entries(&mut entries, &sort_state);
        let order: Vec<usize> = entries.iter().map(|entry| entry.slab_index.get()).collect();

        assert_eq!(
            order,
            vec![0, 1, 2],
            "directories should be listed before files, and files before nodes without metadata"
        );
    }

    #[test]
    fn size_sort_prioritizes_directories_and_paths_for_ties() {
        let sort_state = SortStatePayload {
            key: SortKeyPayload::Size,
            direction: SortDirectionPayload::Asc,
        };
        let mut entries = vec![
            entry_with_metadata(1, "/tmp/z/foo", metadata_with_type(NodeFileType::File, 5)),
            entry_with_metadata(0, "/tmp/m/foo", metadata_with_type(NodeFileType::Dir, 5)),
            entry_with_metadata(2, "/tmp/a/foo", metadata_with_type(NodeFileType::File, 5)),
        ];

        sort_entries(&mut entries, &sort_state);
        let order: Vec<usize> = entries.iter().map(|entry| entry.slab_index.get()).collect();

        assert_eq!(
            order,
            vec![0, 2, 1],
            "directories stay ahead when size and names match, while files fall back to path order"
        );
    }

    #[test]
    fn metadata_sort_orders_recent_files_without_expanding_paths() {
        let sort_state = SortStatePayload {
            key: SortKeyPayload::Mtime,
            direction: SortDirectionPayload::Desc,
        };
        let mut entries = vec![
            (
                SlabIndex::new(7),
                metadata_with_dates(NodeFileType::File, 10, 100, 200),
            ),
            (
                SlabIndex::new(3),
                metadata_with_dates(NodeFileType::File, 10, 100, 900),
            ),
            (
                SlabIndex::new(5),
                metadata_with_dates(NodeFileType::File, 10, 100, 400),
            ),
        ];

        assert!(sort_metadata_indices_cancellable(
            &mut entries,
            &sort_state,
            || false
        ));

        assert_eq!(
            entries
                .iter()
                .map(|(slab_index, _)| slab_index.get())
                .collect::<Vec<_>>(),
            vec![3, 5, 7]
        );
    }

    #[test]
    fn metadata_sort_stops_when_generation_is_cancelled() {
        let sort_state = SortStatePayload {
            key: SortKeyPayload::Mtime,
            direction: SortDirectionPayload::Desc,
        };
        let mut entries = vec![
            (
                SlabIndex::new(1),
                metadata_with_dates(NodeFileType::File, 1, 1, 1),
            ),
            (
                SlabIndex::new(2),
                metadata_with_dates(NodeFileType::File, 1, 1, 2),
            ),
        ];

        assert!(!sort_metadata_indices_cancellable(
            &mut entries,
            &sort_state,
            || true
        ));
    }

    #[test]
    fn cancelled_metadata_sort_does_not_poison_the_next_sort() {
        use std::cell::Cell;

        let sort_state = SortStatePayload {
            key: SortKeyPayload::Size,
            direction: SortDirectionPayload::Asc,
        };
        let mut entries = (0..40_000)
            .rev()
            .map(|index| {
                (
                    SlabIndex::new(index),
                    metadata_with_type(NodeFileType::File, index as u64),
                )
            })
            .collect::<Vec<_>>();
        let checks = Cell::new(0usize);
        assert!(!sort_metadata_indices_cancellable(
            &mut entries,
            &sort_state,
            || {
                checks.set(checks.get() + 1);
                checks.get() > 2
            }
        ));

        assert!(sort_metadata_indices_cancellable(
            &mut entries,
            &sort_state,
            || false
        ));
        assert_eq!(entries.first().map(|entry| entry.0.get()), Some(0));
        assert_eq!(entries.last().map(|entry| entry.0.get()), Some(39_999));
    }
}
