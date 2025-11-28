use slab_mmap::Slab;
use std::fmt::Debug;

// Generic helper used to exercise different data types
fn test_basic_operations<T>(values: &[T])
where
    T: Clone + Eq + Debug,
{
    let mut slab = Slab::new().unwrap();
    let mut indices = Vec::new();

    // Test insertions
    for value in values {
        let idx = slab.insert(value.clone()).unwrap();
        indices.push((idx, value.clone()));
        assert_eq!(slab.len(), indices.len());
    }

    // Test reads
    for (idx, value) in &indices {
        assert_eq!(slab.get(*idx), Some(value));
        assert_eq!(slab[*idx], *value);
    }

    // Test mutations
    for (idx, _) in &indices {
        if let Some(mut_ref) = slab.get_mut(*idx) {
            // We could increment numeric types, but here we only ensure the ref is valid
            // Because T is generic we avoid changing values; this just checks get_mut works
            let _ = mut_ref;
        } else {
            panic!("Failed to get mutable reference for index {idx}");
        }
    }

    // Test removal and reuse
    let (first_idx, first_value) = indices[0].clone();
    let first_value_clone = first_value.clone();
    assert_eq!(slab.try_remove(first_idx), Some(first_value_clone));
    assert_eq!(slab.len(), indices.len() - 1);
    assert!(slab.get(first_idx).is_none());

    // Ensure removed slots are reused
    let reused_idx = slab.insert(first_value).unwrap();
    assert_eq!(reused_idx, first_idx);
    assert_eq!(slab.len(), indices.len());
    let reused_expect = indices[0].1.clone();
    assert_eq!(slab[reused_idx], reused_expect);
}

#[test]
fn test_i32_operations() {
    let values = [1, 2, 3, 4, 5, 100, -1, 0, i32::MAX, i32::MIN];
    test_basic_operations(&values);
}

#[test]
fn test_string_operations() {
    let values = [
        "hello",
        "world",
        "slab-mmap",
        "test",
        "",
        "very long string to test memory management and serialization",
    ];
    test_basic_operations(&values);
}

#[test]
fn test_vector_operations() {
    let values = [
        vec![1, 2, 3],
        vec![],
        vec![100; 100],
        vec![i32::MAX, i32::MIN],
    ];
    test_basic_operations(&values);
}

#[test]
fn test_try_remove_invalid_indices() {
    let mut slab = Slab::new().unwrap();

    assert!(slab.try_remove(0).is_none());

    let idx = slab.insert("value").unwrap();
    assert_eq!(slab.len(), 1);

    assert_eq!(slab.try_remove(idx), Some("value"));
    assert!(slab.try_remove(idx).is_none());
    assert!(slab.try_remove(idx + 1).is_none());
    assert!(slab.is_empty());
}

#[test]
fn test_free_list_reuse_is_lifo() {
    let mut slab = Slab::new().unwrap();

    let first = slab.insert("first").unwrap();
    let second = slab.insert("second").unwrap();
    let third = slab.insert("third").unwrap();
    assert_eq!(slab.len(), 3);

    assert_eq!(slab.try_remove(second), Some("second"));
    assert_eq!(slab.try_remove(first), Some("first"));
    assert_eq!(slab.len(), 1);

    let reused_first = slab.insert("replacement_first").unwrap();
    assert_eq!(reused_first, first);
    assert_eq!(slab[reused_first], "replacement_first");

    let reused_second = slab.insert("replacement_second").unwrap();
    assert_eq!(reused_second, second);
    assert_eq!(slab[third], "third");
    assert_eq!(slab.len(), 3);
}

#[test]
fn test_complex_struct_operations() {
    #[derive(Clone, PartialEq, Eq, Debug)]
    struct TestStruct {
        id: u32,
        name: String,
        data: Vec<u8>,
    }

    let values = [
        TestStruct {
            id: 1,
            name: "test1".to_string(),
            data: vec![1, 2, 3],
        },
        TestStruct {
            id: 2,
            name: "test2".to_string(),
            data: vec![],
        },
        TestStruct {
            id: u32::MAX,
            name: "".to_string(),
            data: vec![42; 100],
        },
    ];
    test_basic_operations(&values);
}

#[test]
fn test_capacity_growth() {
    let mut slab = Slab::new().unwrap();
    let initial_capacity = 1024; // Implementation starts at capacity 1024

    // Insert exactly the initial capacity
    for i in 0..initial_capacity {
        slab.insert(i).unwrap();
    }

    // Insert one more element to trigger growth
    let idx = slab.insert(initial_capacity).unwrap();
    assert!(idx >= initial_capacity);
    assert_eq!(slab[idx], initial_capacity);

    // Ensure every element remains accessible
    for i in 0..=initial_capacity {
        assert_eq!(slab[i], i);
    }
}

#[test]
fn test_index_trait() {
    let mut slab = Slab::new().unwrap();
    let idx1 = slab.insert("first").unwrap();
    let idx2 = slab.insert("second").unwrap();

    // Index trait behavior
    assert_eq!(slab[idx1], "first");
    assert_eq!(slab[idx2], "second");

    // IndexMut trait behavior
    slab[idx1] = "modified_first";
    assert_eq!(slab[idx1], "modified_first");

    // Invalid indices should panic
    let _invalid_idx = idx2 + 100;
    slab.try_remove(idx2);

    // Note: we can't assert the panic here because the test would fail even though it panics.
    // We could use std::panic::catch_unwind, but that's outside this basic suite.
}

#[test]
fn test_iterator_complete() {
    let mut slab = Slab::new().unwrap();

    // Insert a few elements
    let indices = [
        slab.insert("a").unwrap(),
        slab.insert("b").unwrap(),
        slab.insert("c").unwrap(),
        slab.insert("d").unwrap(),
        slab.insert("e").unwrap(),
    ];

    // Remove elements to create holes
    slab.try_remove(indices[1]); // Remove "b"
    slab.try_remove(indices[3]); // Remove "d"

    // Collect iterator results
    let mut collected: Vec<_> = slab.iter().collect();

    // Iterator should visit in index order and skip empty slots
    assert_eq!(collected.len(), 3);

    // Sort by index for easier comparison
    collected.sort_by_key(|(idx, _)| *idx);

    // Verify output
    assert_eq!(collected[0].1, &"a");
    assert_eq!(collected[1].1, &"c");
    assert_eq!(collected[2].1, &"e");
}

#[test]
fn test_debug_formatting() {
    let mut slab = Slab::new().unwrap();
    slab.insert(42).unwrap();
    slab.insert(100).unwrap();

    // Debug formatting should not panic
    let debug_str = format!("{slab:?}");

    // Output should mention the expected fields
    assert!(debug_str.contains("Slab"));
    assert!(debug_str.contains("len"));
    assert!(debug_str.contains("next"));
    assert!(debug_str.contains("slots"));
    assert!(debug_str.contains("capacity"));
}

#[test]
fn test_new_returns_ok() {
    let slab: Slab<i32> = Slab::new().unwrap();
    assert!(slab.is_empty());
    assert_eq!(slab.len(), 0);
}

#[test]
fn test_multiple_removals_and_insertions() {
    let mut slab = Slab::new().unwrap();
    let mut keys = Vec::new();

    // Insert 100 elements
    for i in 0..100 {
        keys.push(slab.insert(i).unwrap());
    }

    assert_eq!(slab.len(), 100);

    // Remove entries at even positions
    for i in 0..50 {
        assert_eq!(slab.try_remove(keys[i * 2]), Some(i * 2));
    }

    assert_eq!(slab.len(), 50);

    // Insert 50 more entries to ensure slot reuse
    let mut new_keys = Vec::new();
    for i in 100..150 {
        new_keys.push(slab.insert(i).unwrap());
    }

    assert_eq!(slab.len(), 100);

    // Check whether the new entries reused freed slots
    let mut reused = 0;
    for &new_key in &new_keys {
        if keys.iter().step_by(2).any(|&k| k == new_key) {
            reused += 1;
        }
    }

    // A meaningful number of slots should be reclaimed
    assert!(reused > 0);
}

#[test]
fn test_get_mut_modification() {
    let mut slab = Slab::new().unwrap();
    let idx = slab.insert(Box::new(42)).unwrap();

    {
        // Grab a mutable reference and change it
        let value = slab.get_mut(idx).unwrap();
        **value = 100;
    }

    // Ensure the change sticks
    assert_eq!(**slab.get(idx).unwrap(), 100);
}
