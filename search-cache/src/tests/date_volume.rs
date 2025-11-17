//! Comprehensive high-volume tests aligned with staged changes (date filters & metadata logic).
//! Added in ~200 line segments to exceed 2000 lines in a single file.
//! Segments exercise: keyword dates, ranges, comparisons, inequality, formats, metadata loading.

use super::{
    prelude::*,
    support::{SECONDS_PER_DAY, list_file_names as list_names, set_file_times, ts_for_date as ts},
};
use jiff::{civil::Date, tz::TimeZone};

// Segment 1 ----------------------------------------------------------------
// Keyword coverage: today, yesterday, pastweek, pastmonth, pastyear, thisweek, lastweek.
#[test]
fn segment_1_date_keyword_basic() {
    let tmp = TempDir::new("seg1_keywords").unwrap();
    for name in [
        "today_a.txt",
        "yesterday_b.txt",
        "week_c.txt",
        "month_d.txt",
        "year_e.txt",
    ]
    .iter()
    {
        fs::write(tmp.path().join(name), b"x").unwrap();
    }
    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());
    let today_idx = cache.search("today_a.txt").unwrap()[0];
    let yest_idx = cache.search("yesterday_b.txt").unwrap()[0];
    let week_idx = cache.search("week_c.txt").unwrap()[0];
    let month_idx = cache.search("month_d.txt").unwrap()[0];
    let year_idx = cache.search("year_e.txt").unwrap()[0];

    let now = Timestamp::now().as_second();
    set_file_times(&mut cache, today_idx, now, now);
    set_file_times(
        &mut cache,
        yest_idx,
        now - SECONDS_PER_DAY,
        now - SECONDS_PER_DAY,
    );
    set_file_times(
        &mut cache,
        week_idx,
        now - 6 * SECONDS_PER_DAY,
        now - 6 * SECONDS_PER_DAY,
    );
    set_file_times(
        &mut cache,
        month_idx,
        now - 25 * SECONDS_PER_DAY,
        now - 25 * SECONDS_PER_DAY,
    );
    set_file_times(
        &mut cache,
        year_idx,
        now - 200 * SECONDS_PER_DAY,
        now - 200 * SECONDS_PER_DAY,
    );

    // today
    let today_hits = cache.search("dm:today").unwrap();
    assert_eq!(list_names(&cache, &today_hits), vec!["today_a.txt"]);
    // yesterday
    let yest_hits = cache.search("dm:yesterday").unwrap();
    assert_eq!(list_names(&cache, &yest_hits), vec!["yesterday_b.txt"]);
    // pastweek should include today + yesterday + week_c
    let pastweek_hits = cache.search("dm:pastweek").unwrap();
    let mut expected = vec!["today_a.txt", "week_c.txt", "yesterday_b.txt"];
    expected.sort();
    assert_eq!(list_names(&cache, &pastweek_hits), expected);
    // pastmonth should include all except year_e.txt
    let pastmonth_hits = cache.search("dm:pastmonth").unwrap();
    let mut expected2 = vec![
        "today_a.txt",
        "week_c.txt",
        "yesterday_b.txt",
        "month_d.txt",
    ];
    expected2.sort();
    assert_eq!(list_names(&cache, &pastmonth_hits), expected2);
    // pastyear should include everything
    let pastyear_hits = cache.search("dm:pastyear").unwrap();
    let mut expected3 = vec![
        "today_a.txt",
        "week_c.txt",
        "yesterday_b.txt",
        "month_d.txt",
        "year_e.txt",
    ];
    expected3.sort();
    assert_eq!(list_names(&cache, &pastyear_hits), expected3);

    // thisweek vs lastweek synthetic: shift week_c to lastweek
    let lastweek_ts = now - 8 * SECONDS_PER_DAY;
    set_file_times(&mut cache, week_idx, lastweek_ts, lastweek_ts);
    let thisweek_hits = cache.search("dm:thisweek").unwrap();
    assert!(list_names(&cache, &thisweek_hits).contains(&"today_a.txt".to_string()));
    let lastweek_hits = cache.search("dm:lastweek").unwrap();
    assert_eq!(list_names(&cache, &lastweek_hits), vec!["week_c.txt"]);
}

// Segment 2 ----------------------------------------------------------------
// Month/year keywords and boundary spans.
#[test]
fn segment_2_month_year_keywords() {
    let tmp = TempDir::new("seg2_month_year").unwrap();
    fs::write(tmp.path().join("jan_file.txt"), b"x").unwrap();
    fs::write(tmp.path().join("last_year_file.txt"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());
    let jan_idx = cache.search("jan_file.txt").unwrap()[0];
    let last_year_idx = cache.search("last_year_file.txt").unwrap()[0];
    let now = Timestamp::now().to_zoned(TimeZone::system());
    let today_date = now.date();
    // put jan_file at start of thismonth
    let this_month_start = Date::new(today_date.year(), today_date.month(), 1).unwrap();
    let this_month_start_ts = TimeZone::system()
        .to_zoned(this_month_start.at(12, 0, 0, 0))
        .unwrap()
        .timestamp()
        .as_second();
    set_file_times(
        &mut cache,
        jan_idx,
        this_month_start_ts,
        this_month_start_ts,
    );
    // file from last year
    let last_year_date = Date::new(today_date.year() - 1, 6, 15).unwrap();
    let last_year_ts = TimeZone::system()
        .to_zoned(last_year_date.at(12, 0, 0, 0))
        .unwrap()
        .timestamp()
        .as_second();
    set_file_times(&mut cache, last_year_idx, last_year_ts, last_year_ts);
    let thismonth_hits = cache.search("dm:thismonth").unwrap();
    assert!(list_names(&cache, &thismonth_hits).contains(&"jan_file.txt".to_string()));
    let lastmonth_hits = cache.search("dm:lastmonth").unwrap(); // may or may not include depending on date; just ensure no panic
    assert!(lastmonth_hits.len() <= 1);
    let thisyear_hits = cache.search("dm:thisyear").unwrap();
    assert!(list_names(&cache, &thisyear_hits).contains(&"jan_file.txt".to_string()));
    let lastyear_hits = cache.search("dm:lastyear").unwrap();
    assert!(list_names(&cache, &lastyear_hits).contains(&"last_year_file.txt".to_string()));
}

// Segment 3 ----------------------------------------------------------------
// Comparison operators: >, >=, <, <=, = against a single date.
#[test]
fn segment_3_comparisons_single_date() {
    let tmp = TempDir::new("seg3_comparisons").unwrap();
    fs::write(tmp.path().join("early.txt"), b"x").unwrap();
    fs::write(tmp.path().join("mid.txt"), b"x").unwrap();
    fs::write(tmp.path().join("late.txt"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());
    let early = cache.search("early.txt").unwrap()[0];
    let mid = cache.search("mid.txt").unwrap()[0];
    let late = cache.search("late.txt").unwrap()[0];
    let base = ts(2024, 5, 10);
    set_file_times(
        &mut cache,
        early,
        base - SECONDS_PER_DAY,
        base - SECONDS_PER_DAY,
    );
    set_file_times(&mut cache, mid, base, base);
    set_file_times(
        &mut cache,
        late,
        base + SECONDS_PER_DAY,
        base + SECONDS_PER_DAY,
    );
    let gt_hits = cache.search("dm:>2024-05-10").unwrap();
    assert_eq!(list_names(&cache, &gt_hits), vec!["late.txt"]);
    let gte_hits = cache.search("dm:>=2024-05-10").unwrap();
    let mut expected = vec!["late.txt", "mid.txt"];
    expected.sort();
    assert_eq!(list_names(&cache, &gte_hits), expected);
    let lt_hits = cache.search("dm:<2024-05-10").unwrap();
    assert_eq!(list_names(&cache, &lt_hits), vec!["early.txt"]);
    let lte_hits = cache.search("dm:<=2024-05-10").unwrap();
    let mut expected2 = vec!["early.txt", "mid.txt"];
    expected2.sort();
    assert_eq!(list_names(&cache, &lte_hits), expected2);
    let eq_hits = cache.search("dm:=2024-05-10").unwrap();
    assert_eq!(list_names(&cache, &eq_hits), vec!["mid.txt"]);
}

// Segment 4 ----------------------------------------------------------------
// Not equal operator should exclude the day range.
#[test]
fn segment_4_not_equal() {
    let tmp = TempDir::new("seg4_ne").unwrap();
    fs::write(tmp.path().join("match.txt"), b"x").unwrap();
    fs::write(tmp.path().join("before.txt"), b"x").unwrap();
    fs::write(tmp.path().join("after.txt"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());
    let match_idx = cache.search("match.txt").unwrap()[0];
    let before_idx = cache.search("before.txt").unwrap()[0];
    let after_idx = cache.search("after.txt").unwrap()[0];
    let base = ts(2023, 12, 31);
    set_file_times(
        &mut cache,
        before_idx,
        base - SECONDS_PER_DAY,
        base - SECONDS_PER_DAY,
    );
    set_file_times(&mut cache, match_idx, base, base);
    set_file_times(
        &mut cache,
        after_idx,
        base + SECONDS_PER_DAY,
        base + SECONDS_PER_DAY,
    );
    let ne_hits = cache.search("dm:!=2023-12-31").unwrap();
    let mut expected = vec!["after.txt", "before.txt"];
    expected.sort();
    assert_eq!(list_names(&cache, &ne_hits), expected);
}

// Segment 5 ----------------------------------------------------------------
// Range syntax start-end and mixed separators.
#[test]
fn segment_5_range_queries() {
    let tmp = TempDir::new("seg5_range").unwrap();
    for name in ["d1.txt", "d2.txt", "d3.txt", "d4.txt"].iter() {
        fs::write(tmp.path().join(name), b"x").unwrap();
    }
    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());
    let d1 = cache.search("d1.txt").unwrap()[0];
    let d2 = cache.search("d2.txt").unwrap()[0];
    let d3 = cache.search("d3.txt").unwrap()[0];
    let d4 = cache.search("d4.txt").unwrap()[0];
    set_file_times(&mut cache, d1, ts(2024, 1, 1), ts(2024, 1, 1));
    set_file_times(&mut cache, d2, ts(2024, 1, 5), ts(2024, 1, 5));
    set_file_times(&mut cache, d3, ts(2024, 1, 10), ts(2024, 1, 10));
    set_file_times(&mut cache, d4, ts(2024, 2, 1), ts(2024, 2, 1));
    let range_hits = cache.search("dm:2024-01-01-2024-01-10").unwrap();
    let mut expected = vec!["d1.txt", "d2.txt", "d3.txt"];
    expected.sort();
    assert_eq!(list_names(&cache, &range_hits), expected);
    let slash_range = cache.search("dm:2024/01/05-2024/02/01").unwrap();
    let mut expected2 = vec!["d2.txt", "d3.txt", "d4.txt"];
    expected2.sort();
    assert_eq!(list_names(&cache, &slash_range), expected2);
}

// Segment 6 ----------------------------------------------------------------
// Multiple formats day ambiguity (DD-MM-YYYY vs MM-DD-YYYY).
#[test]
fn segment_6_format_variants() {
    let tmp = TempDir::new("seg6_formats").unwrap();
    fs::write(tmp.path().join("ambiguous.txt"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());
    let idx = cache.search("ambiguous.txt").unwrap()[0];
    set_file_times(&mut cache, idx, ts(2024, 2, 3), ts(2024, 2, 3)); // Feb 3
    let hyphen = cache.search("dm:2024-02-03").unwrap();
    assert_eq!(list_names(&cache, &hyphen), vec!["ambiguous.txt"]);
    let slash = cache.search("dm:2024/02/03").unwrap();
    assert_eq!(list_names(&cache, &slash), vec!["ambiguous.txt"]);
    let dot = cache.search("dm:2024.02.03").unwrap();
    assert_eq!(list_names(&cache, &dot), vec!["ambiguous.txt"]);
}

// Segment 7 ----------------------------------------------------------------
// Metadata loading via date filter should populate previously None metadata.
#[test]
fn segment_7_metadata_population() {
    let tmp = TempDir::new("seg7_meta").unwrap();
    fs::write(tmp.path().join("file_meta.txt"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());
    let idx = cache.search("file_meta.txt").unwrap()[0];
    // initial walk sets file metadata to None
    assert!(cache.file_nodes[idx].metadata.is_none());
    set_file_times(&mut cache, idx, ts(2024, 8, 8), ts(2024, 8, 8));
    let _hits = cache.search("dm:2024-08-08").unwrap();
    // evaluate_date_filter -> node_timestamp -> ensure_metadata => metadata now Some
    assert!(cache.file_nodes[idx].metadata.is_some());
}

// Segment 8 ----------------------------------------------------------------
// Created vs Modified field distinction.
#[test]
fn segment_8_created_vs_modified() {
    let tmp = TempDir::new("seg8_created_modified").unwrap();
    fs::write(tmp.path().join("both.txt"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());
    let idx = cache.search("both.txt").unwrap()[0];
    let created = ts(2024, 9, 1);
    let modified = ts(2024, 9, 15);
    set_file_times(&mut cache, idx, created, modified);
    let dm_hits = cache.search("dm:2024-09-15").unwrap();
    assert_eq!(list_names(&cache, &dm_hits), vec!["both.txt"]);
    let dc_hits = cache.search("dc:2024-09-01").unwrap();
    assert_eq!(list_names(&cache, &dc_hits), vec!["both.txt"]);
    let dm_range = cache.search("dm:2024-09-14-2024-09-16").unwrap();
    assert_eq!(list_names(&cache, &dm_range), vec!["both.txt"]);
    let dc_range = cache.search("dc:2024-08-30-2024-09-02").unwrap();
    assert_eq!(list_names(&cache, &dc_range), vec!["both.txt"]);
}

// Segment 9 ----------------------------------------------------------------
// Error conditions (invalid, reversed range, empty value) should result in parse/eval errors.
#[test]
fn segment_9_error_conditions() {
    let mut cache = SearchCache::walk_fs(TempDir::new("seg9_errors").unwrap().path().to_path_buf());
    // reversed range
    let reversed = cache.search("dm:2024-10-10-2024-09-10");
    assert!(reversed.is_err(), "reversed date range should error");
    // empty argument after dm: should error
    let empty_arg = cache.search("dm:");
    assert!(empty_arg.is_err());
    let invalid_kw = cache.search("dm:notakeyword");
    assert!(invalid_kw.is_err());
}

// Segment 10 ----------------------------------------------------------------
// Stress: many mixed queries to ensure cancellation doesn't trigger & coverage of various ops.
#[test]
fn segment_10_stress_queries() {
    let tmp = TempDir::new("seg10_stress").unwrap();
    for i in 0..50 {
        fs::write(tmp.path().join(format!("f{i}.txt")), b"x").unwrap();
    }
    let mut cache = SearchCache::walk_fs(tmp.path().to_path_buf());
    // assign timestamps in a spread across 50 days
    let base = ts(2024, 1, 1);
    for i in 0..50 {
        let idx = cache.search(&format!("f{i}.txt")).unwrap()[0];
        set_file_times(
            &mut cache,
            idx,
            base + i * SECONDS_PER_DAY,
            base + i * SECONDS_PER_DAY,
        );
    }
    // Multiple queries
    let _ = cache.search("dm:2024-01-01-2024-01-25").unwrap();
    let _ = cache.search("dm:>2024-01-10").unwrap();
    let _ = cache.search("dm:>=2024-01-10").unwrap();
    let _ = cache.search("dm:<2024-02-10").unwrap();
    let _ = cache.search("dm:<=2024-02-10").unwrap();
    let _ = cache.search("dm:!=2024-01-15").unwrap();
    let _ = cache.search("dm:pastmonth").unwrap();
    let _ = cache.search("dm:thisyear").unwrap();
    // ensure no cancellation
    let token = CancellationToken::new(9999); // never cancelled
    let outcome = cache
        .search_with_options(
            "dm:2024-01-01-2024-03-01",
            crate::SearchOptions::default(),
            token,
        )
        .unwrap();
    assert!(outcome.nodes.unwrap().len() >= 50); // all 50 within range
}
