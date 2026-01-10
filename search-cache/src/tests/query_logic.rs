use super::prelude::*;

#[test]
fn test_query_and_or_not_dedup_and_filtering() {
    let tmp = TempDir::new("query_bool").unwrap();
    fs::write(tmp.path().join("report.txt"), b"r").unwrap();
    fs::write(tmp.path().join("report.md"), b"r").unwrap();
    fs::write(tmp.path().join("other.txt"), b"o").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());

    // OR: union should return 3 distinct results
    let or = cache.search("report OR ext:txt").unwrap();
    assert_eq!(or.len(), 3, "OR should dedup overlapping results");

    // AND: intersection should narrow to the txt
    let and = cache.search("report ext:txt").unwrap();
    assert_eq!(and.len(), 1);

    // NOT: exclude names containing 'report'
    let not = cache.search("ext:txt !report").unwrap();
    assert_eq!(not.len(), 1);
    let path = cache.node_path(*not.first().unwrap()).unwrap();
    assert!(path.ends_with(PathBuf::from("other.txt")));
}

#[test]
fn test_regex_prefix_in_queries() {
    let tmp = TempDir::new("query_regex").unwrap();
    fs::write(tmp.path().join("Report Q1.md"), b"x").unwrap();
    fs::write(tmp.path().join("Report Q2.txt"), b"x").unwrap();
    fs::write(tmp.path().join("notes.txt"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());

    let idxs = cache.search("regex:^Report").unwrap();
    assert_eq!(idxs.len(), 2);
}

#[test]
fn test_ext_list_and_intersection() {
    let tmp = TempDir::new("query_ext_list").unwrap();
    fs::write(tmp.path().join("a.txt"), b"x").unwrap();
    fs::write(tmp.path().join("b.md"), b"x").unwrap();
    fs::write(tmp.path().join("c.rs"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());

    // ext list
    let list = cache.search("ext:txt;md").unwrap();
    assert_eq!(list.len(), 2);

    // Combine with word to intersect
    let only_b = cache.search("ext:txt;md b").unwrap();
    assert_eq!(only_b.len(), 1);
    let path = cache.node_path(*only_b.first().unwrap()).unwrap();
    assert!(path.ends_with(PathBuf::from("b.md")));
}

#[test]
fn test_or_then_and_intersection_precedence() {
    let tmp = TempDir::new("query_bool_prec").unwrap();
    fs::write(tmp.path().join("a.txt"), b"x").unwrap();
    fs::write(tmp.path().join("b.md"), b"x").unwrap();
    fs::write(tmp.path().join("c.txt"), b"x").unwrap();
    fs::write(tmp.path().join("d.bin"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());

    // OR has higher precedence; then intersect via implicit AND with ext:txt
    let res = cache.search("a OR b ext:txt").unwrap();
    assert_eq!(res.len(), 1);
    let path = cache.node_path(*res.first().unwrap()).unwrap();
    assert!(path.ends_with(PathBuf::from("a.txt")));

    let res2 = cache.search("a OR b OR c ext:txt").unwrap();
    assert_eq!(res2.len(), 2);
    let names: Vec<_> = res2.iter().map(|i| cache.node_path(*i).unwrap()).collect();
    assert!(names.iter().any(|p| p.ends_with(PathBuf::from("a.txt"))));
    assert!(names.iter().any(|p| p.ends_with(PathBuf::from("c.txt"))));
}

#[test]
fn test_groups_override_boolean_precedence() {
    let tmp = TempDir::new("query_groups_prec").unwrap();
    fs::write(tmp.path().join("ab.txt"), b"x").unwrap();
    fs::write(tmp.path().join("c.txt"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());

    let res = cache.search("(a b) | c").unwrap();
    let names: Vec<_> = res.iter().map(|i| cache.node_path(*i).unwrap()).collect();
    // Some searches also return the root directory node; ensure target files are present
    assert!(names.iter().any(|p| p.ends_with(PathBuf::from("ab.txt"))));
    assert!(names.iter().any(|p| p.ends_with(PathBuf::from("c.txt"))));
}

#[test]
fn test_not_precedence_with_intersection() {
    let tmp = TempDir::new("query_not_prec").unwrap();
    fs::write(tmp.path().join("a.txt"), b"x").unwrap();
    fs::write(tmp.path().join("b.txt"), b"x").unwrap();
    fs::write(tmp.path().join("notes.md"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());

    let res = cache.search("ext:txt !a").unwrap();
    assert_eq!(res.len(), 1);
    let path = cache.node_path(*res.first().unwrap()).unwrap();
    assert!(path.ends_with(PathBuf::from("b.txt")));
}

#[test]
fn test_regex_and_or_with_ext_intersection() {
    let tmp = TempDir::new("query_regex_prec").unwrap();
    fs::write(tmp.path().join("Report Q1.md"), b"x").unwrap();
    fs::write(tmp.path().join("Report Q2.txt"), b"x").unwrap();
    fs::write(tmp.path().join("notes.txt"), b"x").unwrap();
    let mut cache = SearchCache::walk_fs(tmp.path());

    let res = cache.search("regex:^Report OR notes ext:txt").unwrap();
    assert_eq!(res.len(), 2);
    let names: Vec<_> = res.iter().map(|i| cache.node_path(*i).unwrap()).collect();
    assert!(
        names
            .iter()
            .any(|p| p.ends_with(PathBuf::from("Report Q2.txt")))
    );
    assert!(
        names
            .iter()
            .any(|p| p.ends_with(PathBuf::from("notes.txt")))
    );
}

#[test]
fn test_extension_case_sensitivity_in_type_filter() {
    let tmp = TempDir::new("ext_case_type").unwrap();
    fs::write(tmp.path().join("photo.JPG"), b"x").unwrap();
    fs::write(tmp.path().join("image.jpg"), b"x").unwrap();
    fs::write(tmp.path().join("graphic.PNG"), b"x").unwrap();

    let mut cache = SearchCache::walk_fs(tmp.path());

    let results = cache.search("type:picture").unwrap();
    assert_eq!(results.len(), 3, "Should match case-insensitively");
}
