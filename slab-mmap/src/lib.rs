mod builder;
mod serde;

use memmap2::{MmapMut, MmapOptions};
use std::{
    fmt, io,
    marker::PhantomData,
    mem::{self, MaybeUninit},
    num::NonZeroUsize,
    slice,
};
use tempfile::NamedTempFile;

/// Disk-backed slab that keeps the node payloads in a temporary mmap file so the OS
/// can page the largest structure in and out of memory.
pub struct Slab<T> {
    /// Anonymous temporary file that owns the on-disk backing storage.
    file: NamedTempFile,

    /// Memory-mapped view of the file; stores the raw `Entry<T>` array.
    entries: MmapMut,
    /// Number of slots currently mapped.
    entries_capacity: NonZeroUsize,
    /// Number of slots that have been initialized.
    entries_len: usize,

    /// Logical element count (occupied slots only).
    len: usize,
    /// Head of the freelist (index of the next available slot).
    next: usize,

    _marker: PhantomData<T>,
}

#[derive(Clone)]
enum Entry<T> {
    /// Slot is free; stores the index of the next free slot.
    Vacant(usize),
    /// Slot is occupied by a fully initialized value.
    Occupied(T),
}

pub(crate) const INITIAL_SLOTS: NonZeroUsize = NonZeroUsize::new(1024).unwrap();

impl<T> Slab<T> {
    pub fn new() -> io::Result<Self> {
        Self::with_capacity(INITIAL_SLOTS)
    }

    fn with_capacity(capacity: NonZeroUsize) -> io::Result<Self> {
        let mut file = NamedTempFile::new()?;
        let mmap = Self::map_file(&mut file, capacity)?;
        Ok(Self {
            file,
            entries: mmap,
            len: 0,
            next: 0,
            entries_capacity: capacity,
            entries_len: 0,
            _marker: PhantomData,
        })
    }

    fn map_file(file: &mut NamedTempFile, slots: NonZeroUsize) -> io::Result<MmapMut> {
        let bytes = (slots.get() as u64).saturating_mul(mem::size_of::<Entry<T>>() as u64);
        file.as_file_mut().set_len(bytes)?;
        unsafe { MmapOptions::new().map_mut(file.as_file()) }
    }

    /// Ensure the mmap can host at least `min_slots` entries.
    ///
    /// We intentionally copy the classic “double until large enough” strategy from
    /// `Vec` to keep amortized O(1) `insert`s.  Growing the mmap is expensive
    /// (flush + set_len + remap), so avoiding incremental bumps keeps the number
    /// of system calls low.
    #[inline]
    fn ensure_capacity(&mut self, min_capacity: NonZeroUsize) -> io::Result<()> {
        if min_capacity <= self.entries_capacity {
            return Ok(());
        }
        let mut new_capacity = self.entries_capacity;
        while new_capacity < min_capacity {
            new_capacity = new_capacity.saturating_mul(NonZeroUsize::new(2).unwrap());
        }
        self.remap(new_capacity)
    }

    /// Flush dirty pages and remap the file with the new capacity.
    ///
    /// We now bubble up any flushing or mapping failure to the caller so the
    /// application can decide whether to retry, fall back, or abort.  After
    /// remapping we simply update the capacity counters; all the occupied/vacant
    /// metadata is still valid because indices remain stable.
    #[inline]
    fn remap(&mut self, new_capacity: NonZeroUsize) -> io::Result<()> {
        assert!(new_capacity.get() >= self.entries_len);
        self.entries.flush()?;
        self.entries = Self::map_file(&mut self.file, new_capacity)?;
        self.entries_capacity = new_capacity;
        Ok(())
    }

    fn entries(&self) -> &[MaybeUninit<Entry<T>>] {
        unsafe {
            slice::from_raw_parts(
                self.entries.as_ptr().cast::<MaybeUninit<Entry<T>>>(),
                self.entries_capacity.get(),
            )
        }
    }

    fn entries_mut(&mut self) -> &mut [MaybeUninit<Entry<T>>] {
        unsafe {
            slice::from_raw_parts_mut(
                self.entries.as_mut_ptr().cast::<MaybeUninit<Entry<T>>>(),
                self.entries_capacity.get(),
            )
        }
    }

    fn entry(&self, index: usize) -> Option<&Entry<T>> {
        (index < self.entries_len)
            .then(|| unsafe { self.entries().get_unchecked(index).assume_init_ref() })
    }

    fn entry_mut(&mut self, index: usize) -> Option<&mut Entry<T>> {
        (index < self.entries_len).then(|| unsafe {
            self.entries_mut()
                .get_unchecked_mut(index)
                .assume_init_mut()
        })
    }

    fn write_entry(&mut self, index: usize, entry: Entry<T>) {
        unsafe {
            self.entries_mut().get_unchecked_mut(index).write(entry);
        }
    }

    /// Grow to the next power-of-two-ish capacity.
    ///
    /// This helper exists so both inserts and builder operations share the same
    /// policy.  Picking a doubling factor matches the behaviour of `Vec`, so
    /// benchmark expectations carry over.
    fn grow(&mut self) -> io::Result<()> {
        let desired = self
            .entries_capacity
            .saturating_mul(NonZeroUsize::new(2).unwrap());
        self.ensure_capacity(desired)
    }

    /// Insert a value, returning its stable index.
    ///
    /// This is a thin wrapper around `insert_at` so that other APIs (e.g. vacant
    /// entry) can reuse the logic.  We reuse freelist indices whenever possible;
    /// if the freelist is empty we append to the end of the mmap buffer.
    pub fn insert(&mut self, value: T) -> io::Result<usize> {
        let key = self.next;

        self.insert_at(key, value)?;

        Ok(key)
    }

    /// Core insertion routine shared by the public API and `VacantEntry`.
    ///
    /// The branch on `key == entries_len` mirrors the upstream `slab`.  When the
    /// freelist is empty we simply append and bump `entries_len`, which keeps
    /// indices increasing monotonically.  Otherwise, we pop from the freelist and
    /// store the rescued `next` pointer back into `self.next`.
    fn insert_at(&mut self, key: usize, value: T) -> io::Result<()> {
        if key == self.entries_len {
            if self.entries_len == self.entries_capacity.get() {
                self.grow()?;
            }
            self.write_entry(self.entries_len, Entry::Occupied(value));
            self.entries_len += 1;
            self.next = self.entries_len;
        } else {
            let entry = self
                .entry_mut(key)
                .expect("slot must exist when reusing keys");
            let next_free = match entry {
                Entry::Vacant(next) => *next,
                Entry::Occupied(_) => unreachable!("slot unexpectedly occupied"),
            };
            *entry = Entry::Occupied(value);
            self.next = next_free;
        }
        self.len += 1;
        Ok(())
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        self.entry(index).and_then(|entry| match entry {
            Entry::Occupied(value) => Some(value),
            Entry::Vacant(_) => None,
        })
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.entry_mut(index).and_then(|entry| match entry {
            Entry::Occupied(value) => Some(value),
            Entry::Vacant(_) => None,
        })
    }

    /// Remove the value at `index` if it exists, pushing the slot onto the freelist.
    ///
    /// We use `mem::replace` instead of `Option::take` so that we can write the new
    /// `Entry::Vacant(next)` atomically; this keeps the freelist consistent even if
    /// the caller panics after we return.
    pub fn try_remove(&mut self, index: usize) -> Option<T> {
        // Cache `next` up front; otherwise the mutable borrow returned by
        // `entry_mut` would forbid us from reading another field on `self`.
        let next_free = self.next;
        if let Some(entry) = self.entry_mut(index) {
            let prev = mem::replace(entry, Entry::Vacant(next_free));

            if let Entry::Occupied(value) = prev {
                self.len = self.len.saturating_sub(1);
                self.next = index;
                return Some(value);
            } else {
                *entry = prev;
            }
        }
        None
    }

    /// Returns the number of occupied slots in the slab.
    ///
    /// This is the count of elements currently stored, not the total capacity.
    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn iter(&self) -> SlabIter<'_, T> {
        SlabIter {
            slab: self,
            index: 0,
        }
    }
}

impl<T> Drop for Slab<T> {
    fn drop(&mut self) {
        for i in 0..self.entries_len {
            unsafe {
                // Dropping every initialized entry is required because the mmap is
                // just raw bytes.  Invoking `assume_init_drop` is safe even for
                // Vacant slots: the `Entry` enum will only drop the payload when it
                // holds `Occupied(T)` and act as a no-op otherwise.
                self.entries_mut().get_unchecked_mut(i).assume_init_drop();
            }
        }
        let _ = self.entries.flush();
    }
}

impl<T> std::ops::Index<usize> for Slab<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        self.get(index).expect("invalid slab index")
    }
}

impl<T> std::ops::IndexMut<usize> for Slab<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.get_mut(index).expect("invalid slab index")
    }
}

pub struct SlabIter<'a, T> {
    slab: &'a Slab<T>,
    index: usize,
}

impl<'a, T> IntoIterator for &'a Slab<T> {
    type Item = (usize, &'a T);
    type IntoIter = SlabIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T> Iterator for SlabIter<'a, T> {
    type Item = (usize, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        // Walk every initialized slot in order and skip vacant holes.  This mirrors
        // the upstream slab iterator semantics so that serialized indices behave
        // identically regardless of the backing store.
        while self.index < self.slab.entries_len {
            let idx = self.index;
            self.index += 1;
            if let Some(value) = self.slab.get(idx) {
                return Some((idx, value));
            }
        }
        None
    }
}

impl<T> fmt::Debug for Slab<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Slab")
            .field("len", &self.len)
            .field("next", &self.next)
            .field("slots", &self.entries_len)
            .field("capacity", &self.entries_capacity)
            .finish()
    }
}

impl<T> Slab<T> {
    pub(crate) fn builder_reserve_slot(&mut self, index: usize) -> io::Result<()> {
        // The serde builder may request arbitrary keys out of order, so we must
        // materialize every slot up to `index` and chain them into the freelist.
        self.ensure_capacity(NonZeroUsize::new(index.saturating_add(1)).unwrap())?;
        while self.entries_len <= index {
            self.write_entry(self.entries_len, Entry::Vacant(self.next));
            self.entries_len += 1;
        }
        Ok(())
    }

    pub(crate) fn builder_entry_mut(&mut self, index: usize) -> &mut Entry<T> {
        self.entry_mut(index).expect("builder ensured slot exists")
    }

    pub(crate) fn builder_slots(&self) -> usize {
        self.entries_len
    }

    pub(crate) fn builder_set_next(&mut self, next: usize) {
        self.next = next;
    }

    pub(crate) fn builder_increment_len(&mut self) {
        self.len += 1;
    }
}
