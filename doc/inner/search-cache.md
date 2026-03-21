# SearchCache Deep Dive

`search-cache/` is Cardinal's in-process search and indexing engine. It stores the watched filesystem in a compact slab, maintains a name index for fast lookup, and applies FSEvent-driven subtree rescans.

## Core structures
```text
SearchCache
├─ file_nodes: FileNodes
│  ├─ path: PathBuf
│  ├─ ignore_paths: Vec<PathBuf>
│  ├─ root: SlabIndex
│  └─ slab: ThinSlab<SlabNode>
├─ name_index: NameIndex
│  └─ BTreeMap<&'static str, SortedSlabIndices>
├─ last_event_id: u64
├─ rescan_count: u64
└─ stop: &'static AtomicBool
```

`SlabNode` stores:
- `NameAndParent`: interned name pointer + length + parent index
- `children: ThinVec<SlabIndex>`
- `metadata: SlabNodeMetadataCompact`

`SearchResultNode` is the lightweight expansion type returned to the Tauri layer:
- `path: PathBuf`
- `metadata: SlabNodeMetadataCompact`

## Memory model
- `SlabIndex` is a 32-bit wrapper.
- Names are interned through the process-global `NAME_POOL: LazyLock<NamePool>`.
- `NameIndex` stores one entry per unique basename, each mapping to slab indices sorted by full path.
- `StateTypeSize` packs node state, file type, and size into a single `u64`.
- Directory sizes are exposed as `-1` through `StateTypeSize::size()`, which is mainly useful for backend sorting.

## Build and persistence
1. `walk_fs_with_walk_data(...)` captures `current_event_id()`.
2. `fswalk::walk_it(...)` builds a sorted `Node` tree.
3. `construct_node_slab_name_index(...)` converts that tree into `ThinSlab<SlabNode>` plus `NameIndex`.
4. The cache starts with `rescan_count = 0`.

Persistence uses `PersistentStorage` in `persistent.rs`:
- encoded with `postcard`
- compressed with `zstd`
- written atomically via `path.with_extension(".sctmp")`
- currently versioned as `5`

Persisted fields:
- watch root
- ignore paths
- slab root and slab contents
- name index
- `last_event_id`
- `rescan_count`

`NamePool` itself is not persisted. `try_read_persistent_cache(...)` rebuilds it from persisted name-index keys.

## Query pipeline
```text
raw query
  -> cardinal_syntax::parse_query
  -> expand_query_home_dirs
  -> strip_query_quotes
  -> highlight::derive_highlight_terms
  -> cardinal_syntax::optimize_query
  -> SearchCache::evaluate_expr
```

Two important details:
- Search cancellation is represented as `SearchOutcome { nodes: None, .. }`.
- If the input is Unicode-normalization-sensitive, `search_with_options(...)` runs one alternate NFC/NFD query and merges both result sets and highlight terms. This is a pragmatic APFS workaround, not a fully normalization-aware index.

## Matching model
- Slash-delimited search text is segmented by `query-segmentation`.
- Plain case-sensitive segments stay as cheap string operations.
- Case-insensitive or wildcard segments are compiled into regex matchers.
- `GlobStar` (`**`) and `Star` (`*`) are handled explicitly so descendant scans and direct-child scans stay separate.
- Empty query returns `NameIndex::all_indices(...)` in name/path order.

## Supported filters
Current `evaluate_filter(...)` support includes:
- `file:`, `folder:`
- `ext:`
- `parent:`, `infolder:`, `nosubfolders:`
- `type:`, plus the type macros `audio:`, `video:`, `doc:`, `exe:`
- `size:`
- `dm:` and `dc:` date filters
- `content:`
- `tag:`

Notable implementation details:
- `ext:` is lowercase-normalized and only matches file nodes.
- `parent:` intersects against the target folder's direct children.
- `infolder:` intersects against the full descendant set.
- `nosubfolders:` keeps the folder itself plus non-directory direct children only.
- `content:` scans files in `64 KiB` windows and supports ASCII case-insensitive matching by lowercasing the read chunk.
- `tag:` uses per-file xattr reads for smaller base sets and switches to `mdfind` when the candidate set exceeds `TAG_FILTER_MDFIND_THRESHOLD` (`10000`).

## Metadata behavior
The initial full walk usually runs with `need_metadata = false`. That means many nodes start as `State::None` and only fetch metadata later.

Metadata is filled lazily by:
- `ensure_metadata(...)` during size/date filtering
- `expand_file_nodes(...)` when the UI asks for row data

Unavailable metadata is cached as `State::Unaccessible` so failed lookups are not retried forever.

## Incremental updates
`handle_fs_events(...)` works in two phases:
1. `scan_paths(...)` reduces an event batch to the minimal set of paths that still covers every changed subtree.
2. Each remaining path is sent through `scan_path_recursive(...)`.

`scan_path_recursive(...)`:
- ignores configured ignored paths
- removes vanished paths from the slab
- ensures the parent chain exists via `create_node_chain(...)`
- removes the stale child subtree if present
- re-walks the changed path with metadata enabled
- re-inserts the rebuilt subtree and updates `NameIndex`

If any incoming event requires a full rebuild (`RootChanged` or a root-level mutation detected by `FsEvent::should_rescan(...)`), the cache increments `rescan_count` and returns `HandleFSEError::Rescan`.

## Expansion and Tauri-facing API
- `search_with_options(...)` returns slab indices plus highlight terms.
- `query_files_with_options(...)` expands those indices into `SearchResultNode` values.
- `expand_file_nodes(...)` preserves input order and always returns one output per requested index; missing nodes degrade to empty path + inaccessible metadata.

## Practical constraints
- Path order matters. `NameIndex` and many set operations assume indices are kept in lexicographic full-path order.
- Cancellation checks are sparse but pervasive. New long-running loops should use `CancellationToken::is_cancelled_sparse(...)`.
- `metadata_cache.rs` exists in the crate, but the current runtime path relies on per-node lazy metadata stored directly in `SlabNodeMetadataCompact`.
