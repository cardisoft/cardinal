use serde::Deserialize;
use serde_value::{Value, ValueDeserializer};
use slab_mmap::Slab;
use std::collections::BTreeMap;

fn canonical_bytes_from_map(map: &BTreeMap<u64, Value>) -> Vec<u8> {
    let len = map
        .keys()
        .copied()
        .max()
        .map(|m| m as usize + 1)
        .unwrap_or(0);
    let mut seq: Vec<Option<i32>> = vec![None; len];
    for (k, v) in map {
        let idx = *k as usize;
        let num = match v {
            Value::I64(n) => *n as i32,
            Value::U64(n) => *n as i32,
            Value::I32(n) => *n,
            Value::U32(n) => *n as i32,
            other => panic!("unexpected value kind in map: {other:?}"),
        };
        seq[idx] = Some(num);
    }
    postcard::to_allocvec(&seq).expect("encode postcard")
}

// Generate a deterministic mixed pattern of inserts/removes >= 1200 ops.
fn build_patterned_slabs() -> (Slab<i32>, slab::Slab<i32>) {
    let mut mmap = Slab::new().unwrap();
    let mut slab_std = slab::Slab::with_capacity(8);
    let mut keys = Vec::new();
    let mut seed: u64 = 0x1234_5678_9abc_def0;

    for step in 0..1500 {
        // xorshift64* for deterministic spread
        seed ^= seed << 7;
        seed ^= seed >> 9;
        seed ^= seed << 8;
        let do_insert = keys.is_empty() || !seed.is_multiple_of(3);
        if do_insert {
            let value = (step ^ (seed as i32)) & 0x7fff;
            let k_m = mmap.insert(value).unwrap();
            let k_s = slab_std.insert(value);
            keys.push((k_m, k_s));
        } else {
            // Remove a pseudo-random key to leave holes.
            let idx = (seed as usize) % keys.len();
            let (km, ks) = keys.swap_remove(idx);
            assert_eq!(mmap.try_remove(km), slab_std.try_remove(ks));
        }
    }
    (mmap, slab_std)
}

fn canonical_map_from_value(value: Value) -> BTreeMap<u64, Value> {
    match value {
        Value::Map(entries) => entries
            .into_iter()
            .map(|(k, v)| match k {
                Value::U64(i) => (i, v),
                Value::I64(i) => (i as u64, v),
                other => panic!("unexpected key: {other:?}"),
            })
            .collect(),
        Value::Seq(seq) => seq
            .into_iter()
            .enumerate()
            .filter_map(|(idx, entry)| match entry {
                Value::Option(Some(inner)) => Some((idx as u64, *inner)),
                Value::Option(None) => None,
                other => panic!("unexpected seq entry: {other:?}"),
            })
            .collect(),
        other => panic!("unexpected top-level: {other:?}"),
    }
}

fn map_to_value(map: &BTreeMap<u64, Value>) -> Value {
    Value::Map(
        map.iter()
            .map(|(k, v)| (Value::U64(*k), v.clone()))
            .collect(),
    )
}

#[test]
fn serde_bridge_round_trip_large_pattern() {
    let (mmap, slab_std) = build_patterned_slabs();

    // Normalize both encodings to maps of index -> value.
    let mmap_map = canonical_map_from_value(serde_value::to_value(&mmap).unwrap());
    let slab_map = canonical_map_from_value(serde_value::to_value(&slab_std).unwrap());
    assert_eq!(mmap_map, slab_map, "canonical entry maps must match");
    assert_eq!(
        canonical_bytes_from_map(&mmap_map),
        canonical_bytes_from_map(&slab_map),
        "canonical postcard bytes must match"
    );

    // Convert slab (map encoding) and deserialize into mmap slab.
    let map_from_slab = map_to_value(&slab_map);
    let mmap_from_slab: Slab<i32> = Slab::deserialize(ValueDeserializer::<
        serde_value::DeserializerError,
    >::new(map_from_slab))
    .expect("decode mmap from slab value");
    let mmap_from_slab_map =
        canonical_map_from_value(serde_value::to_value(&mmap_from_slab).unwrap());
    assert_eq!(mmap_from_slab_map, slab_map);

    // Convert mmap map encoding and deserialize into std slab.
    let map_from_mmap = map_to_value(&mmap_map);
    let slab_from_mmap: slab::Slab<i32> = slab::Slab::deserialize(ValueDeserializer::<
        serde_value::DeserializerError,
    >::new(map_from_mmap))
    .expect("decode std slab from mmap value");
    let slab_from_mmap_map =
        canonical_map_from_value(serde_value::to_value(&slab_from_mmap).unwrap());
    assert_eq!(slab_from_mmap_map, mmap_map);
}

#[test]
fn serde_bridge_empty_and_sparse_extremes() {
    // Empty
    let mmap_empty = Slab::<i32>::new().unwrap();
    let slab_empty = slab::Slab::<i32>::new();
    let mmap_map = canonical_map_from_value(serde_value::to_value(&mmap_empty).unwrap());
    let slab_map = canonical_map_from_value(serde_value::to_value(&slab_empty).unwrap());
    assert_eq!(mmap_map, slab_map);

    // Sparse with high index hole coverage (force > 1000 slots)
    let mut mmap = Slab::new().unwrap();
    let mut slab_std = slab::Slab::new();
    let mut kept = Vec::new();
    for i in 0..1200 {
        let km = mmap.insert(i).unwrap();
        let ks = slab_std.insert(i);
        // Drop most of them to create deep holes
        if i % 10 == 0 {
            kept.push((km, ks));
        } else {
            assert_eq!(mmap.try_remove(km), slab_std.try_remove(ks));
        }
    }
    // Ensure a few survivors remain.
    assert!(mmap.len() >= 100);
    assert_eq!(mmap.len(), slab_std.len());
    let mmap_map = canonical_map_from_value(serde_value::to_value(&mmap).unwrap());
    let slab_map = canonical_map_from_value(serde_value::to_value(&slab_std).unwrap());
    assert_eq!(mmap_map, slab_map);
}
