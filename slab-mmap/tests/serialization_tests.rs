use serde::{Deserialize, Serialize};
use slab_mmap::Slab;
use std::{collections::HashMap, fmt::Debug};

// Structure used for complex serialization tests
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
struct ComplexData {
    id: u32,
    name: String,
    values: Vec<i32>,
    metadata: HashMap<String, String>,
}

// Serialization helper
fn roundtrip_serde<T>(slab: &Slab<T>) -> Slab<T>
where
    T: Serialize + for<'de> Deserialize<'de> + PartialEq + Debug,
{
    // Serialize to JSON
    let json = serde_json::to_string(slab).expect("Failed to serialize slab");

    // Deserialize from JSON
    let deserialized: Slab<T> = serde_json::from_str(&json).expect("Failed to deserialize slab");

    deserialized
}

#[test]
fn test_basic_serialization() {
    // Serialize basic value types
    let mut slab = Slab::new().unwrap();

    // Insert some data
    let idx1 = slab.insert(42).unwrap();
    let idx2 = slab.insert(100).unwrap();
    let idx3 = slab.insert(-1).unwrap();

    // Round-trip through serialization
    let deserialized = roundtrip_serde(&slab);

    // Validate data consistency
    assert_eq!(deserialized.len(), slab.len());
    assert_eq!(deserialized.get(idx1), Some(&42));
    assert_eq!(deserialized.get(idx2), Some(&100));
    assert_eq!(deserialized.get(idx3), Some(&-1));
}

#[test]
fn test_empty_slab_serialization() {
    // Serialize an empty slab
    let slab: Slab<i32> = Slab::new().unwrap();

    let deserialized = roundtrip_serde(&slab);

    assert!(deserialized.is_empty());
    assert_eq!(deserialized.len(), 0);
}

#[test]
fn test_with_holes_serialization() {
    // Serialize slabs that contain holes
    let mut slab = Slab::new().unwrap();

    // Insert data
    let idx1 = slab.insert("a".to_string()).unwrap();
    let idx2 = slab.insert("b".to_string()).unwrap();
    let idx3 = slab.insert("c".to_string()).unwrap();
    let idx4 = slab.insert("d".to_string()).unwrap();

    // Remove items to create holes
    slab.try_remove(idx2);
    slab.try_remove(idx4);

    // Round-trip through serialization
    let deserialized = roundtrip_serde(&slab);

    // Validate data consistency
    assert_eq!(deserialized.len(), slab.len());
    assert_eq!(deserialized.get(idx1), Some(&"a".to_string()));
    assert!(deserialized.get(idx2).is_none());
    assert_eq!(deserialized.get(idx3), Some(&"c".to_string()));
    assert!(deserialized.get(idx4).is_none());

    // Iteration should remain consistent
    let original_iter: Vec<_> = slab.iter().collect();
    let deserialized_iter: Vec<_> = deserialized.iter().collect();
    assert_eq!(original_iter, deserialized_iter);
}

#[test]
fn test_complex_structure_serialization() {
    // Serialize complex structures
    let mut slab = Slab::new().unwrap();

    // Build and insert complex data
    let data1 = ComplexData {
        id: 1,
        name: "test1".to_string(),
        values: vec![1, 2, 3, 4, 5],
        metadata: HashMap::from([
            ("type".to_string(), "example".to_string()),
            ("version".to_string(), "1.0".to_string()),
        ]),
    };

    let data2 = ComplexData {
        id: 2,
        name: "test2".to_string(),
        values: vec![],
        metadata: HashMap::new(),
    };

    let idx1 = slab.insert(data1.clone()).unwrap();
    let idx2 = slab.insert(data2.clone()).unwrap();

    // Round-trip through serialization
    let deserialized = roundtrip_serde(&slab);

    // Validate data consistency
    assert_eq!(deserialized.len(), slab.len());
    assert_eq!(deserialized.get(idx1), Some(&data1));
    assert_eq!(deserialized.get(idx2), Some(&data2));
}

#[test]
fn test_large_data_serialization() {
    // Serialize large data sets
    let mut slab = Slab::new().unwrap();

    // Insert 1000 elements
    for i in 0..1000 {
        slab.insert(format!("Item {i}")).unwrap();
    }

    // Remove some entries to create holes
    for i in 0..1000 {
        if i % 5 == 0 {
            slab.try_remove(i);
        }
    }

    // Round-trip through serialization
    let deserialized = roundtrip_serde(&slab);

    // Validate data consistency
    assert_eq!(deserialized.len(), slab.len());

    // Spot-check a handful of entries
    for i in 0..1000 {
        if i % 5 != 0 {
            // Elements that were not deleted
            assert_eq!(deserialized.get(i), Some(&format!("Item {i}")));
        } else {
            // Elements that were deleted
            assert!(deserialized.get(i).is_none());
        }
    }
}

#[test]
fn test_nested_slab_serialization() {
    // Serialize nested data structures
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct NestedData {
        name: String,
        numbers: Vec<u32>,
    }

    let mut slab = Slab::new().unwrap();

    // Insert nested data
    for i in 0..100 {
        let nested = NestedData {
            name: format!("Nested {i}"),
            numbers: (0..i).collect(),
        };
        slab.insert(nested).unwrap();
    }

    // Round-trip through serialization
    let deserialized = roundtrip_serde(&slab);

    // Validate data consistency
    assert_eq!(deserialized.len(), slab.len());

    // Validate every element
    for i in 0..100 {
        let original = slab.get(i).unwrap();
        let deserialized_item = deserialized.get(i).unwrap();

        assert_eq!(original.name, deserialized_item.name);
        assert_eq!(original.numbers, deserialized_item.numbers);
    }
}

#[test]
fn test_mixed_types_serialization() {
    // Use an enum to cover mixed types
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    enum MixedType {
        Int(i32),
        String(String),
        Bool(bool),
        List(Vec<i32>),
    }

    let mut slab = Slab::new().unwrap();

    // Insert different type variants
    slab.insert(MixedType::Int(42)).unwrap();
    slab.insert(MixedType::String("hello".to_string())).unwrap();
    slab.insert(MixedType::Bool(true)).unwrap();
    slab.insert(MixedType::List(vec![1, 2, 3, 4, 5])).unwrap();

    // Round-trip through serialization
    let deserialized = roundtrip_serde(&slab);

    // Validate data consistency
    assert_eq!(deserialized.len(), 4);
    assert_eq!(deserialized.get(0), Some(&MixedType::Int(42)));
    assert_eq!(
        deserialized.get(1),
        Some(&MixedType::String("hello".to_string()))
    );
    assert_eq!(deserialized.get(2), Some(&MixedType::Bool(true)));
    assert_eq!(
        deserialized.get(3),
        Some(&MixedType::List(vec![1, 2, 3, 4, 5]))
    );
}

#[test]
fn test_serialization_compatibility() {
    // Check compatibility with the std slab serialization
    use slab::Slab as StdSlab;

    // Create both slabs with matching contents
    let mut mmap_slab = Slab::new().unwrap();
    let mut std_slab = StdSlab::new();

    for i in 0..100 {
        mmap_slab.insert(i).unwrap();
        std_slab.insert(i);
    }

    // Data serialized from mmap_slab should represent the std slab correctly
    let mmap_json = serde_json::to_string(&mmap_slab).expect("Failed to serialize mmap slab");
    let std_json = serde_json::to_string(&std_slab).expect("Failed to serialize std slab");

    // Note: JSON may differ due to implementation details, but the values should match
    // We verify this by deserializing

    let mmap_from_std: Slab<i32> =
        serde_json::from_str(&std_json).expect("Failed to deserialize from std slab");
    let std_from_mmap: StdSlab<i32> =
        serde_json::from_str(&mmap_json).expect("Failed to deserialize from mmap slab");

    // Validate data consistency
    assert_eq!(mmap_from_std.len(), 100);
    assert_eq!(std_from_mmap.len(), 100);

    for i in 0..100 {
        let v = i as i32;
        assert_eq!(mmap_from_std.get(i), Some(&v));
        assert_eq!(std_from_mmap.get(i), Some(&v));
    }
}

#[test]
fn test_serialization_with_large_indexes() {
    // Serialize slabs that only retain large indexes
    let mut slab = Slab::new().unwrap();

    // Start by populating some elements
    for i in 0..100 {
        slab.insert(i).unwrap();
    }

    // Remove most entries, keeping only the tail
    for i in 0..95 {
        slab.try_remove(i);
    }

    // Only slots 95-99 remain populated now
    assert_eq!(slab.len(), 5);

    // Round-trip through serialization
    let deserialized = roundtrip_serde(&slab);

    // Validate data consistency
    assert_eq!(deserialized.len(), 5);
    for i in 95..100 {
        assert_eq!(deserialized.get(i), Some(&i));
    }
    for i in 0..95 {
        assert!(deserialized.get(i).is_none());
    }
}
