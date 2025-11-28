use slab_mmap::Slab;
use std::{
    sync::{Arc, Mutex},
    thread,
    vec::Vec,
};

#[test]
fn test_multiple_readers() {
    // Create a slab and populate it
    let slab = Arc::new({
        let mut slab = Slab::new().unwrap();
        for i in 0..1000 {
            slab.insert(i).unwrap();
        }
        slab
    });

    let mut handles = Vec::new();

    // Spawn 10 reader threads
    for thread_id in 0..10 {
        let slab_clone = slab.clone();
        let handle = thread::spawn(move || {
            // Each thread reads a different range
            let start = (thread_id * 100) % 901;
            let end = start + 100;

            for i in start..end {
                let value = slab_clone
                    .get(i)
                    .unwrap_or_else(|| panic!("Failed to get index {i}"));
                assert_eq!(*value, i);
            }
        });
        handles.push(handle);
    }

    // Wait for every thread to finish
    for handle in handles {
        handle.join().expect("Thread failed");
    }
}

#[test]
fn test_sequential_write_read() {
    // Write sequentially and read afterwards
    let shared_slab = Arc::new(Mutex::new(Slab::new().unwrap()));
    let keys = Arc::new(Mutex::new(Vec::new()));

    // Write phase
    {
        let mut slab = shared_slab.lock().unwrap();
        for i in 0..1000 {
            let key = slab.insert(i).unwrap();
            keys.lock().unwrap().push(key);
        }
    }

    // Read phase
    let keys_clone = keys.clone();
    let handle = thread::spawn(move || {
        let slab = shared_slab.lock().unwrap();
        let keys = keys_clone.lock().unwrap();

        for (i, &key) in keys.iter().enumerate() {
            let value = slab
                .get(key)
                .unwrap_or_else(|| panic!("Failed to get key {key}"));
            assert_eq!(*value, i as u32);
        }
    });

    handle.join().expect("Thread failed");
}

#[test]
fn test_concurrent_insert_and_remove() {
    // Note: this test needs external synchronization because Slab isn't thread-safe
    let shared_slab = Arc::new(Mutex::new(Slab::new().unwrap()));
    let keys = Arc::new(Mutex::new(Vec::new()));

    let mut handles = Vec::new();

    // Spawn multiple threads that insert and remove
    for thread_id in 0..4 {
        let shared_slab_clone = shared_slab.clone();
        let keys_clone = keys.clone();

        let handle = thread::spawn(move || {
            for i in 0..100 {
                let value = thread_id * 1000 + i;

                // Insert
                let key = {
                    let mut slab = shared_slab_clone.lock().unwrap();
                    let key = slab.insert(value).unwrap();
                    keys_clone.lock().unwrap().push(key);
                    key
                };

                // Brief sleep to allow other threads to run
                thread::sleep(std::time::Duration::from_micros(10));

                // Remove
                {
                    let mut slab = shared_slab_clone.lock().unwrap();
                    let removed = slab.try_remove(key);
                    assert_eq!(removed, Some(value));
                }
            }
        });

        handles.push(handle);
    }

    // Wait for every thread to finish
    for handle in handles {
        handle.join().expect("Thread failed");
    }

    // Ensure the slab ends empty
    let slab = shared_slab.lock().unwrap();
    assert!(slab.is_empty());
}

#[test]
fn test_stress_test_with_mutex() {
    // Use a Mutex to model multi-threaded slab access
    let shared_slab = Arc::new(Mutex::new(Slab::new().unwrap()));
    let operation_count = 10000;

    let mut handles = Vec::new();

    // Spawn worker threads
    for _ in 0..4 {
        let shared_slab_clone = shared_slab.clone();

        let handle = thread::spawn(move || {
            let mut local_keys = Vec::new();

            for i in 0..operation_count {
                if i % 3 == 0 && !local_keys.is_empty() {
                    // Remove
                    let key = local_keys.pop().unwrap();
                    let mut slab = shared_slab_clone.lock().unwrap();
                    slab.try_remove(key);
                } else {
                    // Insert
                    let mut slab = shared_slab_clone.lock().unwrap();
                    let key = slab.insert(i).unwrap();
                    local_keys.push(key);
                }
            }

            // Clean up any leftover local keys
            for key in local_keys {
                let mut slab = shared_slab_clone.lock().unwrap();
                slab.try_remove(key);
            }
        });

        handles.push(handle);
    }

    // Wait for every thread to finish
    for handle in handles {
        handle.join().expect("Thread failed");
    }

    // Verify the final state
    let slab = shared_slab.lock().unwrap();
    assert!(slab.is_empty());
}

#[test]
fn test_multithreaded_iteration() {
    // Build a pre-filled slab
    let slab = Arc::new({
        let mut slab = Slab::new().unwrap();
        for i in 0..1000 {
            slab.insert(i).unwrap();
        }
        // Remove some elements to create holes
        for i in 0..1000 {
            if i % 3 == 0 {
                slab.try_remove(i);
            }
        }
        slab
    });

    let mut handles = Vec::new();

    // Let multiple threads iterate over the slab concurrently
    for _ in 0..5 {
        let slab_clone = slab.clone();

        let handle = thread::spawn(move || {
            let mut count = 0;
            let mut sum = 0;

            for (_, &value) in slab_clone.iter() {
                count += 1;
                sum += value;
            }

            // Validate the iteration results
            assert_eq!(count, 666); // Removing 1/3 leaves 666 elements
            (count, sum)
        });

        handles.push(handle);
    }

    // Collect every thread's results
    let mut results = Vec::new();
    for handle in handles {
        results.push(handle.join().expect("Thread failed"));
    }

    // Ensure every thread observed the same values
    for &(count, sum) in &results[1..] {
        assert_eq!(count, results[0].0);
        assert_eq!(sum, results[0].1);
    }
}

#[test]
fn test_concurrent_with_reuse() {
    // Test slot reuse when multiple threads contend
    let shared_slab = Arc::new(Mutex::new(Slab::new().unwrap()));
    let keys = Arc::new(Mutex::new(Vec::new()));
    let total_ops = 1000;

    // Seed the slab with an initial batch
    {
        let mut slab = shared_slab.lock().unwrap();
        let mut initial_keys = Vec::new();
        for i in 0..100 {
            let key = slab.insert(i).unwrap();
            initial_keys.push(key);
        }
        drop(slab);
        *keys.lock().unwrap() = initial_keys;
    }

    let mut handles = Vec::new();

    // Spawn threads that delete and reinsert
    for thread_id in 0..4 {
        let shared_slab_clone = shared_slab.clone();
        let keys_clone = keys.clone();

        let handle = thread::spawn(move || {
            for i in 0..total_ops {
                let base_value = thread_id * total_ops + i;

                // Track active slots through the shared key list
                let key = {
                    let mut keys_vec = keys_clone.lock().unwrap();
                    if keys_vec.is_empty() {
                        continue;
                    }
                    let idx = (base_value as usize) % keys_vec.len();
                    keys_vec.remove(idx)
                };

                // Remove and insert again, expecting to reuse the freed slot
                let new_key = {
                    let mut slab = shared_slab_clone.lock().unwrap();
                    let removed = slab.try_remove(key);
                    assert!(removed.is_some());
                    slab.insert(base_value).unwrap()
                };

                keys_clone.lock().unwrap().push(new_key);
            }
        });

        handles.push(handle);
    }

    // Wait for every thread to finish
    for handle in handles {
        handle.join().expect("Thread failed");
    }

    // Verify the final state is consistent
    let slab = shared_slab.lock().unwrap();
    assert!(slab.len() <= 100); // Maximum possible number of occupied slots
}
