# NamePool (String Interning & Name Search)

This chapter documents the `namepool/` crate, which interns path segments and supports name-level search.

---

## Design

`NamePool` wraps a `Mutex<BTreeSet<Box<str>>>`:
- Each unique string is stored exactly once.
- Callers receive `&'static str`-like references via `push`, which remain stable for the process lifetime.

`Debug` is implemented to show only the pool size, which is useful when logging large caches.

---

## push: interning names

```rust
pub fn push<'c>(&'c self, name: &str) -> &'c str
```

- If `name` is not present, it is inserted into the `BTreeSet`.
- Returns a `&str` pointing to the stored `Box<str>` using `str::from_raw_parts`.
- Subsequent `push` calls with the same contents return the same address.

This interning is used heavily by `search-cache` to:
- Deduplicate path segments stored in the slab.
- Provide stable keys for `NameIndex` (BTreeMap<&'static str, SortedSlabIndices>).

---

## Name-level search helpers

NamePool supports several search modes, each taking a `CancellationToken`:

- `search_substr(substr, token)` — names containing `substr`.
- `search_suffix(suffix, token)` — names ending with `suffix`.
- `search_prefix(prefix, token)` — names starting with `prefix`.
- `search_regex(pattern, token)` — names matching a `Regex`.
- `search_exact(exact, token)` — names equal to `exact`.

Shared behavior:
- Results are returned as `Option<BTreeSet<&str>>`.
  - `None` means the operation was cancelled.
  - `Some(set)` contains borrowed references into the pool.
- Each method iterates the pool and calls `token.is_cancelled_sparse(i)?;` so cancellation naturally propagates via `?`.

---

## Integration notes

- The search engine uses NamePool as a building block for higher-level query evaluation:
  - `NameIndex` stores interned names for direct indexing.
  - Path segmentation and Everything-style filters often reduce to name-level queries first.
- Cancellation is shared with `search-cache` and other crates via `search-cancel` so the engine can abort work quickly when the user types a new query.
