use super::state::{SortDirection, SortKey, SortState};
use search_cache::SearchResultNode;
use std::cmp::Ordering;

pub(super) fn compare_results(
    left: &SearchResultNode,
    right: &SearchResultNode,
    sort: SortState,
) -> Ordering {
    let ordering = match sort.key {
        SortKey::Filename => compare_strings(filename_for_sort(left), filename_for_sort(right)),
        SortKey::FullPath => compare_strings(
            &left.path.display().to_string(),
            &right.path.display().to_string(),
        ),
        SortKey::Size => compare_option_u32(size_for_sort(left), size_for_sort(right))
            .then_with(|| compare_strings(filename_for_sort(left), filename_for_sort(right))),
        SortKey::Modified => compare_option_u32(mtime_for_sort(left), mtime_for_sort(right))
            .then_with(|| compare_strings(filename_for_sort(left), filename_for_sort(right))),
        SortKey::Created => compare_option_u32(ctime_for_sort(left), ctime_for_sort(right))
            .then_with(|| compare_strings(filename_for_sort(left), filename_for_sort(right))),
    };

    match sort.direction {
        SortDirection::Asc => ordering,
        SortDirection::Desc => ordering.reverse(),
    }
}

fn compare_strings(left: &str, right: &str) -> Ordering {
    left.to_lowercase()
        .cmp(&right.to_lowercase())
        .then_with(|| left.cmp(right))
}

fn compare_option_u32(left: Option<u32>, right: Option<u32>) -> Ordering {
    match (left, right) {
        (Some(left), Some(right)) => left.cmp(&right),
        (Some(_), None) => Ordering::Greater,
        (None, Some(_)) => Ordering::Less,
        (None, None) => Ordering::Equal,
    }
}

fn filename_for_sort(result: &SearchResultNode) -> &str {
    result
        .path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
}

fn size_for_sort(result: &SearchResultNode) -> Option<u32> {
    result
        .metadata
        .as_ref()
        .as_ref()
        .and_then(|metadata| u32::try_from(metadata.size()).ok())
}

fn mtime_for_sort(result: &SearchResultNode) -> Option<u32> {
    result
        .metadata
        .as_ref()
        .and_then(|metadata| metadata.mtime().map(|t| t.get()))
}

fn ctime_for_sort(result: &SearchResultNode) -> Option<u32> {
    result
        .metadata
        .as_ref()
        .and_then(|metadata| metadata.ctime().map(|t| t.get()))
}
