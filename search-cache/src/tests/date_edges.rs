//! Additional edge & boundary tests for date filters.
//! Focus: open ranges, leap day, ambiguous formats, boolean combos.

use super::{
    prelude::*,
    support::{set_file_times, ts_for_date},
};

#[test]
fn explicit_range_instead_of_open_range() {
    let tmp = TempDir::new("explicit_range").unwrap();
    fs::write(tmp.path().join("a.txt"), b"x").unwrap();
    fs::write(tmp.path().join("b.txt"), b"x").unwrap();
    fs::write(tmp.path().join("c.txt"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());
    let a = cache.search("a.txt").unwrap()[0];
    let b = cache.search("b.txt").unwrap()[0];
    let c = cache.search("c.txt").unwrap()[0];
    set_file_times(
        &mut cache,
        a,
        ts_for_date(2024, 5, 1),
        ts_for_date(2024, 5, 1),
    );
    set_file_times(
        &mut cache,
        b,
        ts_for_date(2024, 5, 10),
        ts_for_date(2024, 5, 10),
    );
    set_file_times(
        &mut cache,
        c,
        ts_for_date(2024, 6, 1),
        ts_for_date(2024, 6, 1),
    );
    // Explicit bounded range covering first half of May.
    let hits = cache.search("dm:2024-05-01-2024-05-15").unwrap();
    let mut names: Vec<_> = hits
        .iter()
        .filter(|i| cache.file_nodes[**i].file_type_hint() == NodeFileType::File)
        .map(|i| {
            cache
                .node_path(*i)
                .unwrap()
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    assert_eq!(names, vec!["a.txt", "b.txt"]);
}

#[test]
fn leap_day_query() {
    let tmp = TempDir::new("leap_day").unwrap();
    fs::write(tmp.path().join("leap.txt"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());
    let leap = cache.search("leap.txt").unwrap()[0];
    set_file_times(
        &mut cache,
        leap,
        ts_for_date(2024, 2, 29),
        ts_for_date(2024, 2, 29),
    );
    let eq = cache.search("dm:=2024-02-29").unwrap();
    assert_eq!(eq.len(), 1);
    let range = cache.search("dm:2024-02-28-2024-03-01").unwrap();
    assert_eq!(range.len(), 1);
}

#[test]
fn dual_format_distinct_dates() {
    let tmp = TempDir::new("dual_formats").unwrap();
    fs::write(tmp.path().join("fmt_dd_mm.txt"), b"x").unwrap();
    fs::write(tmp.path().join("fmt_mm_dd.txt"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());
    let dd_idx = cache.search("fmt_dd_mm.txt").unwrap()[0];
    let mm_idx = cache.search("fmt_mm_dd.txt").unwrap()[0];
    // 02-03-2024 -> 2 Mar 2024; 03-02-2024 -> 3 Feb 2024
    set_file_times(
        &mut cache,
        dd_idx,
        ts_for_date(2024, 3, 2),
        ts_for_date(2024, 3, 2),
    );
    set_file_times(
        &mut cache,
        mm_idx,
        ts_for_date(2024, 2, 3),
        ts_for_date(2024, 2, 3),
    );
    let mar_hits = cache.search("dm:02-03-2024").unwrap();
    assert_eq!(mar_hits.len(), 1);
    let feb_hits = cache.search("dm:03-02-2024").unwrap();
    assert_eq!(feb_hits.len(), 1);
    // Ensure they are distinct
    let mar_name = cache
        .node_path(mar_hits[0])
        .unwrap()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let feb_name = cache
        .node_path(feb_hits[0])
        .unwrap()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    assert_ne!(mar_name, feb_name);
}

#[test]
fn boolean_or_between_dm_and_dc() {
    let tmp = TempDir::new("boolean_dm_dc").unwrap();
    fs::write(tmp.path().join("created.txt"), b"x").unwrap();
    fs::write(tmp.path().join("modified.txt"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());
    let created_idx = cache.search("created.txt").unwrap()[0];
    let modified_idx = cache.search("modified.txt").unwrap()[0];
    let zoned_now = Timestamp::now().to_zoned(jiff::tz::TimeZone::system());
    let today = zoned_now.date();
    let current_year = today.year();
    let last_year = current_year - 1;
    // created file: ctime last year, mtime this year (matches dc:lastyear, dm:thisyear)
    let now_ts = zoned_now.timestamp().as_second();
    set_file_times(
        &mut cache,
        created_idx,
        ts_for_date(last_year as i32, 7, 15),
        now_ts,
    );
    // modified file: ctime this year, mtime this year (matches dm:thisyear only)
    set_file_times(&mut cache, modified_idx, now_ts, now_ts);
    let thisyear_or_lastyear = cache.search("dc:lastyear|dm:thisyear").unwrap();
    let count = thisyear_or_lastyear
        .iter()
        .filter(|i| cache.file_nodes[**i].file_type_hint() == NodeFileType::File)
        .count();
    assert_eq!(count, 2, "OR across dc/dm should include both files");
}

#[test]
fn boolean_and_intersection() {
    let tmp = TempDir::new("boolean_and_intersection").unwrap();
    fs::write(tmp.path().join("weekly.txt"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());
    let idx = cache.search("weekly.txt").unwrap()[0];
    let tz = jiff::tz::TimeZone::system();
    let zoned_now = Timestamp::now().to_zoned(tz.clone());
    let today = zoned_now.date();
    let mut modified_date = today;
    for _ in 0..2 {
        if let Ok(prev) = modified_date.yesterday()
            && prev.year() == today.year()
        {
            modified_date = prev;
        }
    }
    let modified_ts = tz
        .to_zoned(modified_date.at(12, 0, 0, 0))
        .expect("valid local date")
        .timestamp()
        .as_second();
    let now = zoned_now.timestamp().as_second();
    set_file_times(&mut cache, idx, now, modified_ts);
    let hits = cache.search("dm:pastweek dm:thisyear").unwrap();
    let count = hits
        .iter()
        .filter(|i| cache.file_nodes[**i].file_type_hint() == NodeFileType::File)
        .count();
    assert_eq!(count, 1, "intersection should retain only the file node");
}

#[test]
fn single_point_range_equivalence() {
    let tmp = TempDir::new("single_point_range").unwrap();
    fs::write(tmp.path().join("point.txt"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());
    let idx = cache.search("point.txt").unwrap()[0];
    set_file_times(
        &mut cache,
        idx,
        ts_for_date(2024, 5, 10),
        ts_for_date(2024, 5, 10),
    );
    let eq_hits = cache.search("dm:=2024-05-10").unwrap();
    let range_hits = cache.search("dm:2024-05-10-2024-05-10").unwrap();
    assert_eq!(eq_hits.len(), range_hits.len());
}
