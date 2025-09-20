use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, ffi::CStr, sync::LazyLock};

fn get_parallelism() -> usize {
    *LazyLock::new(|| {
        std::thread::available_parallelism()
            .map_or(4, |n| n.get())
            .min(32)
    })
}

#[derive(Serialize, Deserialize)]
pub struct NamePool {
    // e.g. `\0aaa\0bbb\0ccc\0`
    // \0 is used as a separator
    pool: Vec<u8>,
}

impl NamePool {
    pub fn new() -> Self {
        Self { pool: vec![b'\0'] }
    }

    pub fn len(&self) -> usize {
        self.pool.len()
    }

    pub fn push(&mut self, name: &str) -> usize {
        let start = self.pool.len();
        self.pool.extend_from_slice(name.as_bytes());
        self.pool.push(0);
        start
    }

    // returns index of the trailing \0 and the string
    fn get(&self, offset: usize) -> (usize, &str) {
        // as this function should only be called by ourselves
        debug_assert!(offset < self.pool.len());
        // offset seperates string like this `\0 aaa\0 bbb\0 ccc\0`
        let begin = self.pool[..offset]
            .iter()
            .rposition(|&x| x == 0)
            .map(|x| x + 1)
            .unwrap_or(0);
        let end = self.pool[offset..]
            .iter()
            .position(|&x| x == 0)
            .map(|x| x + offset)
            .unwrap_or(self.pool.len());
        (end, unsafe {
            std::str::from_utf8_unchecked(&self.pool[begin..end])
        })
    }

    pub fn search_substr<'a>(&'a self, substr: &'a str) -> impl Iterator<Item = &'a str> + 'a {
        let mut last_end = 0;
        // TODO(ldm0): remove x > last_end check
        memchr::memmem::find_iter(&self.pool, substr.as_bytes()).filter_map(move |x| {
            (x > last_end).then(|| {
                let (new_end, s) = self.get(x);
                last_end = new_end;
                s
            })
        })
    }

    pub fn search_subslice<'search, 'pool: 'search>(
        &'pool self,
        subslice: &'search [u8],
    ) -> impl Iterator<Item = &'pool str> + 'search {
        let mut last_end = 0;
        memchr::memmem::find_iter(&self.pool, subslice).filter_map(move |x| {
            (x > last_end).then(|| {
                let (new_end, s) = self.get(x);
                last_end = new_end;
                s
            })
        })
    }

    pub fn search_suffix<'search, 'pool: 'search>(
        &'pool self,
        suffix: &'search CStr,
    ) -> impl Iterator<Item = &'pool str> + 'search {
        let mut last_end = 0;
        memchr::memmem::find_iter(&self.pool, suffix.to_bytes_with_nul()).filter_map(move |x| {
            (x > last_end).then(|| {
                let (new_end, s) = self.get(x);
                last_end = new_end;
                s
            })
        })
    }

    // prefix should starts with a \0, e.g. b"\0hello"
    pub fn search_prefix<'search, 'pool: 'search>(
        &'pool self,
        prefix: &'search [u8],
    ) -> impl Iterator<Item = &'pool str> + 'search {
        assert_eq!(prefix[0], 0);
        let mut last_end = 0;
        memchr::memmem::find_iter(&self.pool, prefix)
            // To make sure it points to the end of the prefix. If we use the begin index, we will get a string before the correct one.
            .map(|x| x + prefix.len() - 1)
            .filter_map(move |x| {
                (x > last_end).then(|| {
                    let (new_end, s) = self.get(x);
                    last_end = new_end;
                    s
                })
            })
    }

    // `exact` should starts with a '\0', and ends with a '\0',
    // e.g. b"\0hello\0"
    pub fn search_exact<'search, 'pool: 'search>(
        &'pool self,
        exact: &'search [u8],
    ) -> impl Iterator<Item = &'pool str> + 'search {
        assert_eq!(exact[0], 0);
        assert_eq!(exact[exact.len() - 1], 0);
        memchr::memmem::find_iter(&self.pool, exact).map(|x| {
            let (_, s) = self.get(x + exact.len() - 1);
            s
        })
    }

    pub fn par_search_substr<'a>(&'a self, substr: &'a str) -> Vec<&'a str> {
        let substr_bytes = substr.as_bytes();
        let pool_len = self.pool.len();
        if pool_len == 0 || substr_bytes.is_empty() {
            return vec![];
        }
        let chunk_size = (pool_len / get_parallelism()).max(1024).min(pool_len);

        (0..pool_len)
            .into_par_iter()
            .step_by(chunk_size)
            .flat_map(|i| {
                let search_end = (i + chunk_size).min(pool_len);
                let read_end = (search_end + substr_bytes.len() - 1).min(pool_len);
                let slice = &self.pool[i..read_end];
                memchr::memmem::find_iter(slice, substr_bytes)
                    .filter(move |&x| i + x < search_end)
                    .map(move |x| self.get(i + x))
                    .collect::<Vec<_>>()
            })
            .collect::<BTreeMap<_, _>>()
            .into_values()
            .collect()
    }

    pub fn par_search_subslice<'search, 'pool: 'search>(
        &'pool self,
        subslice: &'search [u8],
    ) -> Vec<&'pool str> {
        let pool_len = self.pool.len();
        if pool_len == 0 || subslice.is_empty() {
            return vec![];
        }
        let chunk_size = (pool_len / get_parallelism()).max(1024).min(pool_len);

        (0..pool_len)
            .into_par_iter()
            .step_by(chunk_size)
            .flat_map(|i| {
                let search_end = (i + chunk_size).min(pool_len);
                let read_end = (search_end + subslice.len() - 1).min(pool_len);
                let slice = &self.pool[i..read_end];
                memchr::memmem::find_iter(slice, subslice)
                    .filter(move |&x| i + x < search_end)
                    .map(move |x| self.get(i + x))
                    .collect::<Vec<_>>()
            })
            .collect::<BTreeMap<_, _>>()
            .into_values()
            .collect()
    }

    pub fn par_search_suffix<'search, 'pool: 'search>(
        &'pool self,
        suffix: &'search CStr,
    ) -> Vec<&'pool str> {
        let suffix_bytes = suffix.to_bytes_with_nul();
        let pool_len = self.pool.len();
        if pool_len == 0 || suffix_bytes.is_empty() {
            return vec![];
        }
        let chunk_size = (pool_len / get_parallelism()).max(1024).min(pool_len);

        (0..pool_len)
            .into_par_iter()
            .step_by(chunk_size)
            .flat_map(|i| {
                let search_end = (i + chunk_size).min(pool_len);
                let read_end = (search_end + suffix_bytes.len() - 1).min(pool_len);
                let slice = &self.pool[i..read_end];
                memchr::memmem::find_iter(slice, suffix_bytes)
                    .filter(move |&x| i + x < search_end)
                    .map(move |x| self.get(i + x))
                    .collect::<Vec<_>>()
            })
            .collect::<BTreeMap<_, _>>()
            .into_values()
            .collect()
    }

    // prefix should starts with a '\0', e.g. b"\0hello"
    pub fn par_search_prefix<'search, 'pool: 'search>(
        &'pool self,
        prefix: &'search [u8],
    ) -> Vec<&'pool str> {
        assert_eq!(prefix[0], 0);
        let pool_len = self.pool.len();
        if pool_len == 0 {
            return vec![];
        }
        let chunk_size = (pool_len / get_parallelism()).max(1024).min(pool_len);

        (0..pool_len)
            .into_par_iter()
            .step_by(chunk_size)
            .flat_map(|i| {
                let search_end = (i + chunk_size).min(pool_len);
                let read_end = (search_end + prefix.len() - 1).min(pool_len);
                let slice = &self.pool[i..read_end];
                memchr::memmem::find_iter(slice, prefix)
                    .filter(move |&x| i + x < search_end)
                    .map(move |x| self.get(i + x + prefix.len() - 1))
                    .collect::<Vec<_>>()
            })
            .collect::<BTreeMap<_, _>>()
            .into_values()
            .collect()
    }

    // `exact` should starts with a '\0', and ends with a '\0',
    // e.g. b"\0hello\0"
    pub fn par_search_exact<'search, 'pool: 'search>(
        &'pool self,
        exact: &'search [u8],
    ) -> Vec<&'pool str> {
        assert_eq!(exact[0], 0);
        assert_eq!(exact[exact.len() - 1], 0);
        let pool_len = self.pool.len();
        if pool_len == 0 {
            return vec![];
        }
        let chunk_size = (pool_len / get_parallelism()).max(1024).min(pool_len);

        (0..pool_len)
            .into_par_iter()
            .step_by(chunk_size)
            .flat_map(|i| {
                let search_end = (i + chunk_size).min(pool_len);
                let read_end = (search_end + exact.len() - 1).min(pool_len);
                let slice = &self.pool[i..read_end];
                memchr::memmem::find_iter(slice, exact)
                    .filter(move |&x| i + x < search_end)
                    .map(move |x| self.get(i + x + exact.len() - 1))
                    .collect::<Vec<_>>()
            })
            .collect::<BTreeMap<_, _>>()
            .into_values()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_pool() {
        let pool = NamePool::new();
        assert_eq!(pool.len(), 1); // Only the initial \0
        assert_eq!(pool.get(0), (0, ""));
    }

    #[test]
    fn test_push_and_get() {
        let mut pool = NamePool::new();
        let offset1 = pool.push("foo");
        let offset2 = pool.push("bar");
        let offset3 = pool.push("baz");

        assert_eq!(offset1, 1);
        assert_eq!(offset2, 5);
        assert_eq!(offset3, 9);

        assert_eq!(pool.get(offset1), (4, "foo"));
        assert_eq!(pool.get(offset2), (8, "bar"));
        assert_eq!(pool.get(offset3), (12, "baz"));
    }

    #[test]
    fn test_push_empty_string() {
        let mut pool = NamePool::new();
        let offset = pool.push("");
        assert_eq!(offset, 1);
        assert_eq!(pool.get(offset), (1, ""));
        assert_eq!(pool.len(), 2); // Initial \0 + pushed \0
    }

    #[test]
    fn test_get_with_offsets() {
        let mut pool = NamePool::new();
        let offset = pool.push("hello");
        assert_eq!(offset, 1);
        assert_eq!(pool.get(offset), (6, "hello"));
        assert_eq!(pool.get(0), (0, ""));
        for i in 1..=6 {
            assert_eq!(pool.get(i), (6, "hello"));
        }

        let offset = pool.push("world");
        assert_eq!(offset, 7);
        assert_eq!(pool.get(offset), (12, "world"));
        for i in 7..=12 {
            assert_eq!(pool.get(i), (12, "world"));
        }
    }

    #[test]
    fn test_search_substr() {
        let mut pool = NamePool::new();
        pool.push("hello");
        pool.push("world");
        pool.push("hello world");
        pool.push("hello world hello");

        let substr = "hello";
        let result: Vec<_> = pool.search_substr(substr).collect();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], "hello");
        assert_eq!(result[1], "hello world");
        assert_eq!(result[2], "hello world hello");
    }

    #[test]
    fn test_search_subslice() {
        let mut pool = NamePool::new();
        pool.push("hello");
        pool.push("world");
        pool.push("hello world");
        pool.push("hello world hello");

        let subslice = b"world";
        let result: Vec<_> = pool.search_subslice(subslice).collect();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], "world");
        assert_eq!(result[1], "hello world");
        assert_eq!(result[2], "hello world hello");
    }

    #[test]
    fn test_search_suffix() {
        let mut pool = NamePool::new();
        pool.push("hello");
        pool.push("world");
        pool.push("hello world");
        pool.push("hello world hello");

        let suffix = c"world";
        let result: Vec<_> = pool.search_suffix(suffix).collect();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], "world");
        assert_eq!(result[1], "hello world");
    }

    #[test]
    fn test_search_nonexistent() {
        let mut pool = NamePool::new();
        pool.push("hello");
        pool.push("world");

        let substr = "nonexistent";
        let result: Vec<_> = pool.search_substr(substr).collect();
        assert!(result.is_empty());

        let subslice = b"nonexistent";
        let result: Vec<_> = pool.search_subslice(subslice).collect();
        assert!(result.is_empty());
    }

    #[test]
    fn test_search_partial_match() {
        let mut pool = NamePool::new();
        pool.push("hello");
        pool.push("world");
        pool.push("hell");

        let substr = "hell";
        let result: Vec<_> = pool.search_substr(substr).collect();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], "hello");
        assert_eq!(result[1], "hell");
    }

    #[test]
    fn test_search_suffix_partial() {
        let mut pool = NamePool::new();
        pool.push("hello");
        pool.push("world");
        pool.push("hell");

        let suffix = c"ell";
        let result: Vec<_> = pool.search_suffix(suffix).collect();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], "hell");
    }

    #[test]
    fn test_push_unicode() {
        let mut pool = NamePool::new();
        let offset = pool.push("こんにちは");
        assert_eq!(offset, 1);
        assert_eq!(pool.get(offset), (16, "こんにちは"));
    }

    #[test]
    fn test_search_unicode() {
        let mut pool = NamePool::new();
        pool.push("こんにちは");
        pool.push("世界");
        pool.push("こんにちは世界");

        let substr = "世界";
        let result: Vec<_> = pool.search_substr(substr).collect();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], "世界");
        assert_eq!(result[1], "こんにちは世界");
    }

    #[test]
    fn test_search_unicode_suffix() {
        let mut pool = NamePool::new();
        pool.push("こんにちは");
        pool.push("世界");
        pool.push("こんにちは世界");

        let suffix = c"世界";
        let result: Vec<_> = pool.search_suffix(suffix).collect();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], "世界");
        assert_eq!(result[1], "こんにちは世界");
    }

    #[test]
    fn test_search_prefix() {
        let mut pool = NamePool::new();
        pool.push("hello");
        pool.push("world");
        pool.push("hello world");
        pool.push("hello world hello");

        let prefix = b"\0hello";
        let result: Vec<_> = pool.search_prefix(prefix).collect();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], "hello");
        assert_eq!(result[1], "hello world");
        assert_eq!(result[2], "hello world hello");
    }

    #[test]
    fn test_search_prefix_nonexistent() {
        let mut pool = NamePool::new();
        pool.push("hello");
        pool.push("world");

        let prefix = b"\0nonexistent";
        let result: Vec<_> = pool.search_prefix(prefix).collect();
        assert!(result.is_empty());
    }

    #[test]
    fn test_search_prefix_partial_match() {
        let mut pool = NamePool::new();
        pool.push("hello");
        pool.push("world");
        pool.push("hell");

        let prefix = b"\0hell";
        let result: Vec<_> = pool.search_prefix(prefix).collect();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], "hello");
        assert_eq!(result[1], "hell");
    }

    #[test]
    fn test_search_prefix_unicode() {
        let mut pool = NamePool::new();
        pool.push("こんにちは");
        pool.push("世界");
        pool.push("こんにちは世界");

        let prefix = "\0こんにちは";
        let result: Vec<_> = pool.search_prefix(prefix.as_bytes()).collect();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], "こんにちは");
        assert_eq!(result[1], "こんにちは世界");
    }

    #[test]
    #[should_panic(expected = "assertion `left == right` failed\n  left: 104\n right: 0")]
    fn test_search_prefix_should_panic() {
        let mut pool = NamePool::new();
        pool.push("hello");
        pool.push("world");

        // This should panic because the prefix does not start with \0
        let prefix = b"hello";
        let _result: Vec<_> = pool.search_prefix(prefix).collect();
    }

    #[test]
    fn test_search_exact() {
        let mut pool = NamePool::new();
        pool.push("hello");
        pool.push("world");
        pool.push("hello world");
        pool.push("hello world hello");

        let exact = b"\0hello\0";
        let result: Vec<_> = pool.search_exact(exact).collect();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], "hello");

        let exact = b"\0world\0";
        let result: Vec<_> = pool.search_exact(exact).collect();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], "world");

        let exact = b"\0hello world\0";
        let result: Vec<_> = pool.search_exact(exact).collect();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], "hello world");

        let exact = b"\0hello world hello\0";
        let result: Vec<_> = pool.search_exact(exact).collect();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], "hello world hello");
    }

    #[test]
    fn test_search_exact_nonexistent() {
        let mut pool = NamePool::new();
        pool.push("hello");
        pool.push("world");

        let exact = b"\0nonexistent\0";
        let result: Vec<_> = pool.search_exact(exact).collect();
        assert!(result.is_empty());
    }

    #[test]
    fn test_search_exact_partial_match() {
        let mut pool = NamePool::new();
        pool.push("hello");
        pool.push("world");
        pool.push("hell");

        let exact = b"\0hell\0";
        let result: Vec<_> = pool.search_exact(exact).collect();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], "hell");

        let exact = b"\0hello\0";
        let result: Vec<_> = pool.search_exact(exact).collect();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], "hello");
    }

    #[test]
    fn test_search_exact_unicode() {
        let mut pool = NamePool::new();
        pool.push("こんにちは");
        pool.push("世界");
        pool.push("こんにちは世界");

        let exact = "\0こんにちは\0".as_bytes();
        let result: Vec<_> = pool.search_exact(exact).collect();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], "こんにちは");

        let exact = "\0世界\0".as_bytes();
        let result: Vec<_> = pool.search_exact(exact).collect();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], "世界");

        let exact = "\0こんにちは世界\0".as_bytes();
        let result: Vec<_> = pool.search_exact(exact).collect();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], "こんにちは世界");
    }

    #[test]
    #[should_panic(expected = "assertion `left == right` failed\n  left: 104\n right: 0")]
    fn test_search_exact_should_panic_no_leading_null() {
        let mut pool = NamePool::new();
        pool.push("hello");

        // This should panic because the exact string does not start with \0
        let exact = b"hello\0";
        let _result: Vec<_> = pool.search_exact(exact).collect();
    }

    #[test]
    #[should_panic(expected = "assertion `left == right` failed\n  left: 111\n right: 0")]
    fn test_search_exact_should_panic_no_trailing_null() {
        let mut pool = NamePool::new();
        pool.push("hello");

        // This should panic because the exact string does not end with '\0'
        let exact = b"\0hello";
        let _result: Vec<_> = pool.search_exact(exact).collect();
    }

    #[test]
    fn test_par_search_substr() {
        let mut pool = NamePool::new();
        pool.push("hello");
        pool.push("world");
        pool.push("hello world");
        pool.push("hello world hello");

        let substr = "hello";
        let mut result = pool.par_search_substr(substr);
        result.sort(); // The order is not guaranteed
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], "hello");
        assert_eq!(result[1], "hello world");
        assert_eq!(result[2], "hello world hello");
    }

    #[test]
    fn test_par_search_substr_boundary() {
        let mut pool = NamePool::new();
        // Let's construct the pool first, then calculate chunk size.
        pool.push(&"a".repeat(1000));
        pool.push("b");
        pool.push(&"c".repeat(1000));

        let pool_len = pool.pool.len();
        let chunk_size = (pool_len / get_parallelism()).max(1024);

        // We need to place a name across a chunk boundary.
        // Let's clear and rebuild the pool.
        pool = NamePool::new();
        let prefix_len = chunk_size - 50;
        let name_to_find = "b".repeat(100);

        pool.push(&"a".repeat(prefix_len));
        let name_start_pos = pool.pool.len();
        pool.push(&name_to_find);
        pool.push(&"c".repeat(1000));

        // Check if the name is actually across the boundary
        assert!(
            name_start_pos < chunk_size,
            "Test setup failed: name does not start before chunk boundary"
        );
        assert!(
            name_start_pos + name_to_find.len() > chunk_size,
            "Test setup failed: name does not cross chunk boundary"
        );

        let result = pool.par_search_substr(&name_to_find);
        assert_eq!(result.len(), 1, "The name at the boundary was not found");
        assert_eq!(result[0], name_to_find);
    }

    #[test]
    fn test_par_search_exact() {
        let mut pool = NamePool::new();
        pool.push("hello");
        pool.push("world");
        pool.push("hello world");
        pool.push("hello world hello");

        let exact = b"\0hello\0";
        let result = pool.par_search_exact(exact);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], "hello");
    }

    #[test]
    fn test_par_search_empty_pool() {
        let pool = NamePool::new();
        let result = pool.par_search_substr("hello");
        assert!(result.is_empty());
    }

    #[test]
    fn test_par_search_empty_query() {
        let mut pool = NamePool::new();
        pool.push("hello");
        let result = pool.par_search_substr("");
        assert!(result.is_empty());
    }

    #[test]
    fn test_par_search_unicode() {
        let mut pool = NamePool::new();
        pool.push("こんにちは");
        pool.push("世界");
        pool.push("こんにちは世界");

        let substr = "世界";
        let result = pool.par_search_substr(substr);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], "世界");
        assert_eq!(result[1], "こんにちは世界");
    }

    #[test]
    fn test_par_search_nonexistent() {
        let mut pool = NamePool::new();
        pool.push("hello");
        pool.push("world");

        let substr = "nonexistent";
        let result = pool.par_search_substr(substr);
        assert!(result.is_empty());
    }

    #[test]
    fn test_par_search_subslice_boundary() {
        let mut pool = NamePool::new();
        // Let's construct the pool first, then calculate chunk size.
        pool.push(&"a".repeat(1000));
        pool.push("b");
        pool.push(&"c".repeat(1000));

        let pool_len = pool.pool.len();
        let chunk_size = (pool_len / get_parallelism()).max(1024);

        // We need to place a name across a chunk boundary.
        // Let's clear and rebuild the pool.
        pool = NamePool::new();
        let prefix_len = chunk_size - 50;
        let name_to_find = "b".repeat(100);

        pool.push(&"a".repeat(prefix_len));
        let name_start_pos = pool.pool.len();
        pool.push(&name_to_find);
        pool.push(&"c".repeat(1000));

        // Check if the name is actually across the boundary
        assert!(
            name_start_pos < chunk_size,
            "Test setup failed: name does not start before chunk boundary"
        );
        assert!(
            name_start_pos + name_to_find.len() > chunk_size,
            "Test setup failed: name does not cross chunk boundary"
        );

        let result = pool.par_search_subslice(name_to_find.as_bytes());
        assert_eq!(result.len(), 1, "The name at the boundary was not found");
        assert_eq!(result[0], name_to_find);
    }

    #[test]
    fn test_par_search_suffix_boundary() {
        let mut pool = NamePool::new();
        // Let's construct the pool first, then calculate chunk size.
        pool.push(&"a".repeat(1000));
        pool.push("b");
        pool.push(&"c".repeat(1000));

        let pool_len = pool.pool.len();
        let chunk_size = (pool_len / get_parallelism()).max(1024);

        // We need to place a name across a chunk boundary.
        // Let's clear and rebuild the pool.
        pool = NamePool::new();
        let suffix_to_find = "b".repeat(100);
        let full_name = "a".repeat(50) + &suffix_to_find;

        // Place the name such that the suffix crosses the chunk boundary
        let before_name_len = chunk_size - 100; // 50 for prefix, 50 for first part of suffix
        pool.push(&"c".repeat(before_name_len));
        let name_start_pos = pool.pool.len();
        pool.push(&full_name);
        pool.push(&"d".repeat(1000));

        let suffix_start_pos = name_start_pos + 50;

        // Check if the suffix is actually across the boundary
        assert!(
            suffix_start_pos < chunk_size,
            "Test setup failed: suffix does not start before chunk boundary"
        );
        assert!(
            suffix_start_pos + suffix_to_find.len() > chunk_size,
            "Test setup failed: suffix does not cross chunk boundary"
        );

        let suffix_cstring = std::ffi::CString::new(suffix_to_find.as_bytes()).unwrap();
        let result = pool.par_search_suffix(&suffix_cstring);
        assert_eq!(result.len(), 1, "The name at the boundary was not found");
        assert_eq!(result[0], full_name);
    }

    #[test]
    fn test_par_search_prefix_boundary() {
        let mut pool = NamePool::new();
        // Let's construct the pool first, then calculate chunk size.
        pool.push(&"a".repeat(1000));
        pool.push("b");
        pool.push(&"c".repeat(1000));

        let pool_len = pool.pool.len();
        let chunk_size = (pool_len / get_parallelism()).max(1024);

        // We need to place a name across a chunk boundary.
        // Let's clear and rebuild the pool.
        pool = NamePool::new();
        let prefix_to_find = "b".repeat(100);
        let full_name = prefix_to_find.clone() + &"a".repeat(50);

        // Place the name such that the prefix crosses the chunk boundary
        let before_name_len = chunk_size - 50;
        pool.push(&"c".repeat(before_name_len));
        let name_start_pos = pool.pool.len();
        pool.push(&full_name);
        pool.push(&"d".repeat(1000));

        // The search is for \0prefix. The \0 is at name_start_pos - 1.
        // We want the prefix part to cross the boundary.
        assert!(
            name_start_pos < chunk_size,
            "Test setup failed: prefix does not start before chunk boundary"
        );
        assert!(
            name_start_pos + prefix_to_find.len() > chunk_size,
            "Test setup failed: prefix does not cross chunk boundary"
        );

        let prefix_bytes = [b"\0".as_slice(), prefix_to_find.as_bytes()].concat();
        let result = pool.par_search_prefix(&prefix_bytes);
        assert_eq!(result.len(), 1, "The name at the boundary was not found");
        assert_eq!(result[0], full_name);
    }

    #[test]
    fn test_par_search_exact_boundary() {
        let mut pool = NamePool::new();
        // Let's construct the pool first, then calculate chunk size.
        pool.push(&"a".repeat(1000));
        pool.push("b");
        pool.push(&"c".repeat(1000));

        let pool_len = pool.pool.len();
        let chunk_size = (pool_len / get_parallelism()).max(1024);

        // We need to place a name across a chunk boundary.
        // Let's clear and rebuild the pool.
        pool = NamePool::new();
        let name_to_find = "b".repeat(100);

        // Place the name such that it crosses the chunk boundary
        let before_name_len = chunk_size - 50;
        pool.push(&"c".repeat(before_name_len));
        let name_start_pos = pool.pool.len();
        pool.push(&name_to_find);
        pool.push(&"d".repeat(1000));

        // The search is for \0exact\0. The \0 is at name_start_pos - 1.
        // We want the exact part to cross the boundary.
        assert!(
            name_start_pos < chunk_size,
            "Test setup failed: name does not start before chunk boundary"
        );
        assert!(
            name_start_pos + name_to_find.len() > chunk_size,
            "Test setup failed: name does not cross chunk boundary"
        );

        let exact_bytes = [b"\0".as_slice(), name_to_find.as_bytes(), b"\0".as_slice()].concat();
        let result = pool.par_search_exact(&exact_bytes);
        assert_eq!(result.len(), 1, "The name at the boundary was not found");
        assert_eq!(result[0], name_to_find);
    }
}
