use crate::{SearchCache, SlabIndex, SlabNodeMetadataCompact};
use fswalk::{NodeFileType, NodeMetadata};
use jiff::{civil::Date, tz::TimeZone};
use std::num::NonZeroU64;

pub(super) const SECONDS_PER_DAY: i64 = 24 * 60 * 60;

pub(super) fn set_file_times(
    cache: &mut SearchCache,
    index: SlabIndex,
    created: i64,
    modified: i64,
) {
    let metadata = NodeMetadata {
        r#type: NodeFileType::File,
        size: 0,
        ctime: NonZeroU64::new(created as u64),
        mtime: NonZeroU64::new(modified as u64),
    };
    cache.file_nodes[index].metadata = SlabNodeMetadataCompact::some(metadata);
}

pub(super) fn assert_file_hits(cache: &SearchCache, indices: &[SlabIndex], expected: &[&str]) {
    let mut names: Vec<String> = indices
        .iter()
        .filter(|idx| cache.file_nodes[**idx].metadata.file_type_hint() == NodeFileType::File)
        .filter_map(|idx| cache.node_path(*idx))
        .filter_map(|path| {
            path.file_name()
                .map(|name| name.to_string_lossy().into_owned())
        })
        .collect();
    names.sort();
    let mut expected_vec: Vec<String> = expected.iter().map(|s| s.to_string()).collect();
    expected_vec.sort();
    assert_eq!(names, expected_vec);
}

pub(super) fn ts_for_date(year: i32, month: u32, day: u32) -> i64 {
    let tz = TimeZone::system();
    let date = Date::new(
        i16::try_from(year).expect("year fits in range"),
        month as i8,
        day as i8,
    )
    .expect("valid date components");
    tz.to_zoned(date.at(12, 0, 0, 0))
        .expect("valid local date")
        .timestamp()
        .as_second()
}

pub(super) fn node_name(cache: &SearchCache, index: SlabIndex) -> String {
    cache
        .node_path(index)
        .unwrap()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned()
}

pub(super) fn list_file_names(cache: &SearchCache, indices: &[SlabIndex]) -> Vec<String> {
    let mut out: Vec<String> = indices
        .iter()
        .filter(|i| cache.file_nodes[**i].metadata.file_type_hint() == NodeFileType::File)
        .map(|i| node_name(cache, *i))
        .collect();
    out.sort();
    out
}
