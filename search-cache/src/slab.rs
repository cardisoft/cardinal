use serde::{Deserialize, Serialize};
use slab_mmap::{Slab, SlabIter};
use std::io;

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
#[repr(transparent)]
#[serde(transparent)]
pub struct OptionSlabIndex(u32);

impl OptionSlabIndex {
    pub fn none() -> Self {
        Self(u32::MAX)
    }

    pub fn some(index: SlabIndex) -> Self {
        Self(index.0)
    }

    pub fn from_option(index: Option<SlabIndex>) -> Self {
        index.map_or(Self::none(), Self::some)
    }

    pub fn to_option(self) -> Option<SlabIndex> {
        if self.0 == u32::MAX {
            None
        } else {
            Some(SlabIndex(self.0))
        }
    }
}

// 0..=(u32::MAX-1), u32::MAX is reserved
//
// slab index starts from 0, therefore we can say if parent is u32::MAX, it means no parent
// small and dirty size optimization :(
#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
#[serde(transparent)]
pub struct SlabIndex(u32);

impl SlabIndex {
    pub fn new(index: usize) -> Self {
        assert!(
            index < u32::MAX as usize,
            "slab index must be less than u32::MAX"
        );
        Self(index as u32)
    }

    pub fn get(&self) -> usize {
        self.0 as usize
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
#[repr(transparent)]
pub struct ThinSlab<T>(Slab<T>);

impl<T> Default for ThinSlab<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> ThinSlab<T> {
    /// Construct a ThinSlab, panicking if the mmap initialization fails.
    pub fn new() -> Self {
        Self::try_new().expect("ThinSlab::new failed to initialize memory-mapped slab")
    }

    /// Construct a ThinSlab while propagating I/O failures to the caller.
    pub fn try_new() -> io::Result<Self> {
        Slab::new().map(Self)
    }

    /// Inserts a value into the slab.
    ///
    /// # Panics
    ///
    /// This method panics if the underlying memory-mapped slab needs to grow and the operation fails due to an I/O error (e.g., disk full, permission denied, etc.).
    /// If you need to handle such errors gracefully, use [`try_insert`](Self::try_insert) instead.
    pub fn insert(&mut self, value: T) -> SlabIndex {
        self.try_insert(value)
            .expect("ThinSlab::insert failed to grow backing slab")
    }

    /// Insert a value while allowing callers to handle any I/O failures emitted
    /// by the backing slab.
    pub fn try_insert(&mut self, value: T) -> io::Result<SlabIndex> {
        self.0.insert(value).map(SlabIndex::new)
    }

    pub fn get(&self, index: SlabIndex) -> Option<&T> {
        self.0.get(index.get())
    }

    pub fn get_mut(&mut self, index: SlabIndex) -> Option<&mut T> {
        self.0.get_mut(index.get())
    }

    pub fn try_remove(&mut self, index: SlabIndex) -> Option<T> {
        self.0.try_remove(index.get())
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn iter(&self) -> ThinSlabIter<'_, T> {
        ThinSlabIter(self.0.iter())
    }
}

impl<T> std::ops::Index<SlabIndex> for ThinSlab<T> {
    type Output = T;

    fn index(&self, index: SlabIndex) -> &Self::Output {
        &self.0[index.get()]
    }
}

impl<T> std::ops::IndexMut<SlabIndex> for ThinSlab<T> {
    fn index_mut(&mut self, index: SlabIndex) -> &mut Self::Output {
        &mut self.0[index.get()]
    }
}

pub struct ThinSlabIter<'a, T>(SlabIter<'a, T>);

impl<'a, T> Iterator for ThinSlabIter<'a, T> {
    type Item = (SlabIndex, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        self.0
            .next()
            .map(|(index, value)| (SlabIndex::new(index), value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thin_slab_try_new_is_empty() {
        let slab = ThinSlab::<i32>::try_new().expect("ThinSlab::try_new should succeed");
        assert!(slab.is_empty());
        assert_eq!(slab.len(), 0);
    }

    #[test]
    fn thin_slab_try_insert_round_trips() {
        let mut slab = ThinSlab::<i32>::try_new().expect("ThinSlab::try_new should succeed");
        let idx_try = slab
            .try_insert(7)
            .expect("ThinSlab::try_insert should succeed for in-memory data");
        assert_eq!(slab.get(idx_try), Some(&7));

        let idx_insert = slab.insert(99);
        assert_eq!(slab[idx_insert], 99);
        assert_ne!(idx_try, idx_insert);
    }
}
