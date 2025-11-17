use super::{
    prelude::*,
    support::{SECONDS_PER_DAY, assert_file_hits, set_file_times, ts_for_date},
};

#[test]
fn test_date_filters_cover_keywords_and_ranges() {
    let tmp = TempDir::new("date_filters").unwrap();
    fs::write(tmp.path().join("recent.txt"), b"x").unwrap();
    fs::write(tmp.path().join("old.txt"), b"x").unwrap();
    fs::write(tmp.path().join("very_old.txt"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());

    let recent_idx = cache.search("recent.txt").unwrap()[0];
    let old_idx = cache.search("old.txt").unwrap()[0];
    let ancient_idx = cache.search("very_old.txt").unwrap()[0];

    let now = Timestamp::now().as_second();
    set_file_times(&mut cache, recent_idx, now, now);
    set_file_times(
        &mut cache,
        old_idx,
        ts_for_date(2020, 5, 10),
        now - 40 * SECONDS_PER_DAY,
    );
    set_file_times(
        &mut cache,
        ancient_idx,
        ts_for_date(2014, 8, 15),
        ts_for_date(2014, 8, 15),
    );

    let dm_today = cache.search("dm:today").unwrap();
    assert_file_hits(&cache, &dm_today, &["recent.txt"]);

    let dm_past_month = cache.search("dm:pastmonth").unwrap();
    assert_file_hits(&cache, &dm_past_month, &["recent.txt"]);

    let dm_range = cache.search("dm:>=2020-01-01").unwrap();
    assert_file_hits(&cache, &dm_range, &["recent.txt", "old.txt"]);

    let dc_year = cache.search("dc:2020/01/01-2020/12/31").unwrap();
    assert_file_hits(&cache, &dc_year, &["old.txt"]);

    let dc_hyphen = cache.search("dc:1/8/2014-31/8/2014").unwrap();
    assert_file_hits(&cache, &dc_hyphen, &["very_old.txt"]);
}
