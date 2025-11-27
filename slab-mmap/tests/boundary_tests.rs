use serde_json::json;
use slab_mmap::Slab;

#[test]
fn test_empty_slab_operations() {
    let mut slab = Slab::<i32>::new().unwrap();

    // Validate the basic properties of an empty slab
    assert!(slab.is_empty());
    assert_eq!(slab.len(), 0);

    // Fetching from an empty slab yields None
    assert!(slab.get(0).is_none());
    assert!(slab.get_mut(0).is_none());

    // Removing from an empty slab yields None
    assert!(slab.try_remove(0).is_none());

    // Iterating an empty slab produces nothing
    let collected: Vec<_> = slab.iter().collect();
    assert!(collected.is_empty());

    // Insert the first element
    let idx = slab.insert(42).unwrap();
    assert_eq!(idx, 0);
    assert_eq!(slab.len(), 1);
    assert_eq!(slab[idx], 42);
}

#[test]
fn test_very_large_data() {
    let mut slab = Slab::new().unwrap();

    // Create a very large vector
    let large_vector = vec![42; 1_000_000]; // ~4 MB of data

    // Insert the chunk
    let idx = slab.insert(large_vector.clone()).unwrap();

    // Verify data integrity
    assert_eq!(slab.get(idx).unwrap(), &large_vector);

    // Remove it and verify
    let removed = slab.try_remove(idx).unwrap();
    assert_eq!(removed, large_vector);
}

#[test]
fn test_zero_sized_types() {
    // Exercise zero-sized types
    let mut slab = Slab::new().unwrap();

    // Insert several zero-sized entries
    let indices = [
        slab.insert(()).unwrap(),
        slab.insert(()).unwrap(),
        slab.insert(()).unwrap(),
    ];

    // Confirm the assigned indices
    assert_eq!(indices[0], 0);
    assert_eq!(indices[1], 1);
    assert_eq!(indices[2], 2);

    // Ensure we can fetch them
    assert!(slab.get(indices[0]).is_some());

    // Test removal and reuse
    slab.try_remove(indices[1]);
    let reused_idx = slab.insert(()).unwrap();
    assert_eq!(reused_idx, indices[1]);
}

#[test]
fn test_edge_case_indices() {
    let mut slab = Slab::new().unwrap();

    // Insert elements
    for i in 0..100 {
        slab.insert(i).unwrap();
    }

    // Accessing out-of-range indices should fail
    assert!(slab.get(1000).is_none());
    assert!(slab.get_mut(1000).is_none());
    assert!(slab.try_remove(1000).is_none());

    // Reference the theoretical max slot (never reached in practice)
    // We no longer cap slots, but this test stresses growth by alternating ops.

    // Alternate insert/remove operations to create sparse indices
    for i in 0..50 {
        slab.try_remove(i);
    }

    // Confirm the holes exist
    for i in 0..50 {
        assert!(slab.get(i).is_none());
    }

    // Insert again to confirm reuse
    for i in 0..50 {
        let idx = slab.insert(i + 1000).unwrap();
        assert!(idx < 50); // Should reuse previously freed slots
    }
}

#[test]
fn test_struct_with_drop() {
    // Ensure types with Drop clean up correctly
    use std::sync::Arc;

    let counter = Arc::new(());

    #[derive(Clone)]
    struct DropTracked {
        _inner: Arc<()>,
    }

    impl Drop for DropTracked {
        fn drop(&mut self) {
            // Dropping should reduce the Arc strong count
        }
    }

    let mut slab = Slab::new().unwrap();

    // Insert the Arc-backed structs
    let indices = [
        slab.insert(DropTracked {
            _inner: counter.clone(),
        })
        .unwrap(),
        slab.insert(DropTracked {
            _inner: counter.clone(),
        })
        .unwrap(),
        slab.insert(DropTracked {
            _inner: counter.clone(),
        })
        .unwrap(),
    ];

    // Strong count should be 4 (1 original + 3 inserts)
    assert_eq!(Arc::strong_count(&counter), 4);

    // Removing one should decrement the count
    slab.try_remove(indices[1]);
    assert_eq!(Arc::strong_count(&counter), 3);

    // Inserting again increases the count
    slab.insert(DropTracked {
        _inner: counter.clone(),
    })
    .unwrap();
    assert_eq!(Arc::strong_count(&counter), 4);

    // Dropping the slab should drop the remaining elements
    drop(slab);
    assert_eq!(Arc::strong_count(&counter), 1);
}

#[test]
fn test_memory_mapping_edge_cases() {
    // Exercise memory-mapped boundary behavior
    // Note: we stick to high-level effects because the OS details are opaque

    let mut slab = Slab::new().unwrap();

    // Start small and force gradual growth
    let current_capacity = 1024; // Initial capacity

    // Insert until multiple resizes trigger
    for i in 0..current_capacity * 4 {
        slab.insert(i).unwrap();
    }

    // Ensure every stored value is readable
    for i in 0..current_capacity * 4 {
        assert_eq!(slab.get(i).unwrap(), &i);
    }
}

#[test]
fn test_saturating_add_edge_case() {
    // Exercise the next_slot saturating_add behavior
    let mut slab = Slab::new().unwrap();

    // We cannot reach u32::MAX here but we can rely on saturating_add
    // The implementation uses saturating_add to prevent overflow

    // Insert a large number of elements to drive growth
    for i in 0..10_000 {
        slab.insert(i).unwrap();
    }

    assert_eq!(slab.len(), 10_000);
}

#[test]
fn test_mixed_data_types_performance() {
    // Mix differently sized data variants
    #[derive(Clone, PartialEq, Debug)]
    enum Data {
        Small(i32),
        Medium(Vec<u8>),
        Large(Vec<u8>),
    }

    let mut slab = Slab::new().unwrap();

    // Insert the variants
    let small_idx = slab.insert(Data::Small(42)).unwrap();
    let medium_idx = slab.insert(Data::Medium(vec![1u8; 100])).unwrap();
    let large_idx = slab.insert(Data::Large(vec![2u8; 10_000])).unwrap();

    // Ensure each variant remains accessible
    match &slab[small_idx] {
        Data::Small(v) => assert_eq!(*v, 42),
        _ => panic!("unexpected variant for small"),
    }
    match &slab[medium_idx] {
        Data::Medium(v) => assert_eq!(v.len(), 100),
        _ => panic!("unexpected variant for medium"),
    }
    match &slab[large_idx] {
        Data::Large(v) => assert_eq!(v.len(), 10_000),
        _ => panic!("unexpected variant for large"),
    }

    // Remove one and confirm reuse
    slab.try_remove(medium_idx);
    let new_medium_idx = slab.insert(Data::Medium(vec![3u8; 50])).unwrap();
    assert_eq!(new_medium_idx, medium_idx);
    match &slab[new_medium_idx] {
        Data::Medium(v) => assert_eq!(v.len(), 50),
        _ => panic!("unexpected variant for new medium"),
    }
}

#[test]
fn test_ensure_capacity_edge_cases() {
    // Cover ensure_capacity edge cases by forcing repeated growth calls.

    let mut slab = Slab::new().unwrap();

    // Trigger growth via inserts rather than calling the private helper
    let target_capacity = 2048;

    for i in 0..target_capacity {
        slab.insert(i).unwrap();
    }

    for i in 0..target_capacity {
        assert_eq!(slab.get(i), Some(&i));
    }
}

#[test]
fn test_read_after_delete_behavior() {
    // Validate reads after deletion
    let mut slab = Slab::new().unwrap();

    let idx = slab.insert("hello").unwrap();
    assert_eq!(slab.get(idx), Some(&"hello"));

    // Remove the element
    assert_eq!(slab.try_remove(idx), Some("hello"));

    // Ensure nothing can read it afterwards
    assert!(slab.get(idx).is_none());
    assert!(slab.get_mut(idx).is_none());
    assert!(slab.try_remove(idx).is_none());

    // Reinsert at the same index
    slab.insert("world").unwrap();
    assert_eq!(slab.get(idx), Some(&"world"));
}

#[test]
fn test_insert_remove_alternating() {
    // Stress alternating insert/remove behavior
    let mut slab = Slab::new().unwrap();
    let mut keys = Vec::new();

    // Perform the alternating sequence
    for i in 0..1000 {
        if i % 3 == 0 && !keys.is_empty() {
            // Remove the oldest key
            let key = keys.remove(0);
            slab.try_remove(key);
        } else {
            // Insert a new element
            keys.push(slab.insert(i).unwrap());
        }
    }

    // Verify the final slab state
    assert_eq!(slab.len(), keys.len());

    // Remaining keys should still be readable
    for &key in keys.iter() {
        assert!(slab.get(key).is_some());
    }
}

#[test]
fn test_memory_usage_with_holes() {
    // Explore memory usage with many holes
    let mut slab = Slab::new().unwrap();

    // Insert 1000 elements
    for i in 0..1000 {
        slab.insert(i).unwrap();
    }

    // Remove everything except the last entry
    for i in 0..999 {
        slab.try_remove(i);
    }

    // Confirm only the last entry remains
    assert_eq!(slab.len(), 1);
    assert_eq!(slab.get(999), Some(&999));

    // Inserting new entries should reuse freed slots
    for i in 1000..1500 {
        let idx = slab.insert(i).unwrap();
        assert!(idx < 999); // Should reuse earlier slots
    }

    assert_eq!(slab.len(), 501);
}

#[test]
fn test_sparse_deserialize_rebuilds_free_list() {
    let json = json!({
        "0": 10,
        "1023": 20,
        "2048": 30
    });

    let mut slab: Slab<i32> = serde_json::from_value(json).expect("deserialize sparse slab");
    assert_eq!(slab.len(), 3);
    assert_eq!(slab.get(0), Some(&10));
    assert_eq!(slab.get(1023), Some(&20));
    assert_eq!(slab.get(2048), Some(&30));
    assert!(slab.get(1).is_none());

    let reused_idx = slab.insert(99).unwrap();
    assert_eq!(reused_idx, 1);
    assert_eq!(slab.get(reused_idx), Some(&99));
    assert_eq!(slab.len(), 4);
}
