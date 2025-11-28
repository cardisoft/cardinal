use super::{Entry, Slab};
use std::{io, num::NonZeroUsize};

/// A helper struct for reconstructing a `Slab` from arbitrary key/value pairs during deserialization.
///
/// The `Builder` is used to incrementally rebuild a slab by inserting key/value pairs,
/// typically as part of a deserialization process. It ensures that the slab's internal
/// free list is correctly reconstructed by scanning for unoccupied slots after all pairs
/// have been inserted. This guarantees that subsequent allocations from the slab will
/// behave as expected.
///
/// When inserting a key/value pair, if the key already exists and is occupied, the old
/// value is dropped and replaced with the new one. This matches the behavior of typical
/// deserialization, where the last occurrence of a key takes precedence. Duplicate keys
/// are handled by dropping the previous value to avoid memory leaks.
///
/// # Usage
/// - Use `Builder::with_capacity` to create a builder with a given capacity.
/// - Call `pair(key, value)` for each key/value to insert.
/// - Call `build()` to finalize and obtain the reconstructed `Slab`.
///
/// The serialized form is a sparse map from index to payload.  During
/// deserialization we may see keys out of order, so we can't rely on the normal
/// push-based slab API.  The builder is therefore allowed to poke at the private
/// freelist helpers to recreate exactly the same in-memory shape that the slab
/// would have produced organically.
pub(crate) struct Builder<T> {
    slab: Slab<T>,
}

impl<T> Builder<T> {
    pub(crate) fn with_capacity(capacity: NonZeroUsize) -> io::Result<Self> {
        Ok(Self {
            slab: Slab::with_capacity(capacity)?,
        })
    }

    pub(crate) fn pair(&mut self, key: usize, value: T) -> io::Result<()> {
        self.slab.builder_reserve_slot(key)?;
        let entry = self.slab.builder_entry_mut(key);
        match entry {
            Entry::Occupied(existing) => {
                // Overwrite in place if the serialized data contains duplicate keys.
                *existing = value;
            }
            Entry::Vacant(_) => {
                *entry = Entry::Occupied(value);
                self.slab.builder_increment_len();
            }
        }
        Ok(())
    }

    pub(crate) fn build(mut self) -> Slab<T> {
        let mut next = self.slab.builder_slots();
        for idx in (0..self.slab.builder_slots()).rev() {
            let entry = self.slab.builder_entry_mut(idx);
            if matches!(entry, Entry::Vacant(_)) {
                // Reconstruct the freelist tail-first so that the next vacant
                // insert reuses the smallest index, mirroring the runtime behaviour.
                *entry = Entry::Vacant(next);
                next = idx;
            }
        }
        self.slab.builder_set_next(next);
        self.slab
    }
}
