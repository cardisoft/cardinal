# SearchCache Deep Dive

This chapter covers the Rust search/index engine in `search-cache/`.

---

## Core data structures
```
Walk root (PathBuf)
└── FileNodes (slab of SlabNode)
    ├─ root: SlabIndex
    ├─ slab: ThinSlab<SlabNode>
    │   SlabNode {
    │     name_and_parent: NameAndParent { name: &'static str, parent: Option<SlabIndex> }
    │     metadata: SlabNodeMetadataCompact (type/size/mtime/ctime, compact optional ints)
    │     children: ThinVec<SlabIndex>
    │   }
    └─ helpers: node_path(index) builds absolute paths by climbing parents

NameIndex (BTreeMap<&'static str, SortedSlabIndices>)
└─ maps interned names → sorted list of SlabIndex ordered by full path
   (keeps per-name hits sorted and deduplicated)

NamePool (namepool crate)
└─ interns strings to &'static str so NameIndex keys are stable and cheap to clone
```

---

### In-memory layout overview

```text
SearchCache
├─ file_nodes: FileNodes
│  ├─ path: PathBuf       (watch root)
│  ├─ root: SlabIndex     (root node in slab)
│  └─ slab: ThinSlab<SlabNode>
│      SlabNode {
│        name_and_parent: NameAndParent { ptr, len, parent: OptionSlabIndex }
│        children: ThinVec<SlabIndex>
│        metadata: SlabNodeMetadataCompact
│      }
├─ name_index: NameIndex
│  └─ BTreeMap<&'static str, SortedSlabIndices>
│        (indices sorted by full path)
└─ last_event_id: u64
```

## Memory layout and compression

To keep the in-memory mirror compact while still indexing millions of files, `SearchCache` uses several layout tricks:

- `SlabIndex` is a 32-bit wrapper (`u32`), supporting up to ~4 billion nodes while halving index storage compared to `u64`.
- `NameAndParent` stores:
  - A pointer to an interned filename (`&'static str` from `NAME_POOL`),
  - A 32-bit length (macOS/Unix filenames are limited to 255 characters),
  - An `OptionSlabIndex` parent.
  This replaces a full `&str` (pointer + `usize`) plus a separate parent field.
- `SlabNodeMetadataCompact` packs:
  - State (`None`/`Some`/`Unaccessible`) into 2 bits,
  - File type (file/dir/symlink/unknown) into 2 bits,
  - Size into 60 bits (saturating at `(1<<60)-1`, sufficient for multi‑TB volumes),
  - `ctime`/`mtime` into `u32` seconds since Unix epoch.
- `ThinVec<SlabIndex>` is used for `children` instead of `Vec<SlabIndex>`, so leaf nodes (the common case) pay only for a null pointer instead of a full `(ptr,len,cap)` triple.

In combination, these choices roughly halve the memory footprint of the slab compared to a naive `String`/`Vec`/`u64` implementation, while keeping access patterns cache-friendly.

---

## Lifecycle
1. **Initial build** (`walk_fs*`): `fswalk::walk_it` produces a tree of `Node` with metadata; we then allocate a slab and `NameIndex` in one pass (`construct_node_slab_name_index`). The last FSEvent ID at build time is recorded for incremental updates.
2. **Persistence**: `persistent::{write_cache_to_file, read_cache_from_file}` snapshot `{ path, slab_root, slab, name_index, last_event_id }`. `NamePool` is *not* persisted; it is reconstructed from `name_index` on load because interning is fast.
3. **Incremental updates**:
   - FSEvents come from `cardinal_sdk::EventWatcher` with `FsEvent { path, flag, id }`.
   - Adds/removes/renames call into `scan_path_recursive` (re-walk subtree) or `remove_node_path`.
   - `ignore_paths` are honored both in initial walk and rescans.
   - On error conditions (e.g., `HandleFSEError::Rescan`) the entire cache is rebuilt via `rescan_with_walk_data`.

```
FSEvents -> handle_fs_events -> {remove | create_node_chain | scan_path_recursive}
         -> update FileNodes + NameIndex
         -> last_event_id advanced
```

---

## Query path
```
UI query string
   ↓ parse (cardinal-syntax::parse_query)
   ↓ normalize paths (search-cache::expand_query_home_dirs)
   ↓ optimize (cardinal-syntax::optimize_query)
   ↓ highlight terms (highlight::derive_highlight_terms)
   ↓ evaluate_expr (SearchCache)
        - uses NameIndex for fast name term expansion
        - uses type/size/time filters via metadata cache
        - path segments via query-segmentation
        - cancellation checks every CANCEL_CHECK_INTERVAL
   ↓ SearchOutcome { nodes: Option<Vec<SlabIndex>>, highlights }
```

- Cancellation uses `search-cancel::CancellationToken` (versioned per request). When cancelled, `nodes` becomes `None`.
- Empty query uses `NameIndex::all_indices` to return every node in path order with cancellation checks.

---

## Metadata and type filters
- Metadata is compacted into `SlabNodeMetadataCompact` for memory density (see above).
- `type_and_size` (`StateTypeSize`) encodes state, type, and size together and exposes helpers to classify node type (file/dir/other) and obtain sizes.
- Initial full scans are run without per-file metadata (`WalkData::new(..., need_metadata = false, ...)`) to avoid slow `lstat` calls on APFS; the cache lazily populates metadata when filters (size/date/type) require it.
- `metadata_cache` and `ensure_metadata` handle this lazy loading, updating `SlabNodeMetadataCompact` in-place the first time a node’s metadata is needed.

---

## Rescan logic
```
rescan_with_walk_data:
  new_cache = walk_fs_with_walk_data(...)
  if cancelled -> None (caller keeps old cache)
  else replace self with new_cache
```

- Long rescans stream progress via `walk_data.num_dirs/num_files` (used by the background loop to emit status updates).

---

## Stored vs computed
- **Stored**: slab (tree), `NameIndex` (name → sorted indices), `last_event_id`.
- **Computed on demand**: absolute paths (`node_path`), subtrees (`all_subnodes`), metadata lookups for filters (when not already cached).

---

## Extension tips
- To add new query operators, update `cardinal-syntax` and ensure `highlight::derive_highlight_terms` covers them.
- Keep `CANCEL_CHECK_INTERVAL` low enough for responsive cancels; avoid heavy work outside cancellable loops.
- Slab indices are 32-bit; stay safely below `u32::MAX` nodes for a given cache.
