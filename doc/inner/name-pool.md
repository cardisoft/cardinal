# NamePool (String Interning & Name Search)

`namepool/` is the small interning crate behind `search_cache::NAME_POOL`.

## Core idea
`NamePool` stores unique names in `Mutex<BTreeSet<Box<str>>>`. Every distinct string is kept once, and callers can retrieve stable borrowed `&str` references backed by that set.

In Cardinal, the pool lives behind a process-global `LazyLock`, so interned names effectively stay valid for the lifetime of the app.

## `push`
```rust
pub fn push<'c>(&'c self, name: &str) -> &'c str
```

Behavior:
- insert the name if it is not already present
- look up the stored `Box<str>`
- rebuild a borrowed `&str` with `str::from_raw_parts(...)`

This is what lets `SearchCache` keep `SlabNode` names and `NameIndex` keys as cheap shared references instead of duplicating strings in every node.

## Search helpers
Available helpers:
- `search_substr(...)`
- `search_suffix(...)`
- `search_prefix(...)`
- `search_regex(...)`
- `search_exact(...)`

All of them:
- iterate the interned set in sorted order
- return `Option<BTreeSet<&str>>`
- use `CancellationToken::is_cancelled_sparse(...)` so large scans remain abortable

`None` means the search was cancelled. `Some(set)` means the scan completed, even if the set is empty.

## Integration with Cardinal
- `search-cache` interns every basename through `NAME_POOL.push(...)`.
- `NameIndex` maps each interned name to slab indices sorted by full path.
- Query evaluation uses the pool as the fast first step for basename matching before path hierarchy constraints are applied.

## Practical notes
- The pool is optimized for relatively infrequent inserts and very cheap repeated lookups.
- `Debug` only prints the pool size, which keeps logs readable even when millions of names are interned.
