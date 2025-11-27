use slab_mmap::Slab;

#[test]
fn insert_get_remove_one() {
    let mut slab = Slab::new().unwrap();
    assert_eq!(slab.len(), 0);
    assert!(slab.is_empty());

    let key = slab.insert(10).unwrap();
    assert_eq!(slab.len(), 1);
    assert_eq!(slab[key], 10);
    assert_eq!(slab.get(key), Some(&10));

    assert_eq!(slab.try_remove(key), Some(10));
    assert_eq!(slab.try_remove(key), None);
    assert!(slab.is_empty());
}

#[test]
fn slots_are_reused_after_remove() {
    let mut slab = Slab::new().unwrap();
    let first = slab.insert("alpha").unwrap();
    let second = slab.insert("beta").unwrap();
    assert_eq!(first, 0);
    assert_eq!(second, 1);

    assert_eq!(slab.try_remove(first), Some("alpha"));
    let third = slab.insert("gamma").unwrap();
    // We recycle the freed slot.
    assert_eq!(third, first);
    assert_eq!(slab[third], "gamma");
    assert_eq!(slab.len(), 2);
}

#[test]
fn get_mut_allows_update() {
    let mut slab = Slab::new().unwrap();
    let key = slab.insert(1).unwrap();
    *slab.get_mut(key).unwrap() = 5;
    assert_eq!(slab[key], 5);
}

#[test]
fn iter_skips_holes_and_preserves_order() {
    let mut slab = Slab::new().unwrap();
    let a = slab.insert("a").unwrap();
    let b = slab.insert("b").unwrap();
    let c = slab.insert("c").unwrap();
    assert_eq!(slab.try_remove(b), Some("b"));

    let collected: Vec<_> = slab.iter().collect();
    assert_eq!(collected, vec![(a, &"a"), (c, &"c")]);
}
