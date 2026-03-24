# FSWalk (Filesystem Walker)

`fswalk/` builds the in-memory tree that `SearchCache` turns into its slab.

## Core types
`Node`:
```text
Node {
  children: Vec<Node>,
  name: Box<str>,
  metadata: Option<NodeMetadata>,
}
```

`NodeMetadata`:
- `type: NodeFileType`
- `size: u64`
- `ctime: Option<NonZeroU64>`
- `mtime: Option<NonZeroU64>`

`NodeFileType` is a compact `repr(u8)` enum: `File`, `Dir`, `Symlink`, `Unknown`.

## WalkData
`WalkData<'w, F>` carries both configuration and counters:
- `num_files`
- `num_dirs`
- `root_path`
- `ignore_directories`
- `need_metadata`
- `cancel: F` where `F: Fn() -> bool`

Constructors:
- `WalkData::simple(root_path, need_metadata)`
- `WalkData::new(root_path, ignore_directories, need_metadata, cancel)`

This is why callers in `search-cache` can use either `CancellationToken`-aware scans or simple non-cancellable walks without a different API surface.

## Entry points
- `walk_it_without_root_chain(...)` returns a tree rooted exactly at `root_path`.
- `walk_it(...)` wraps that tree with the full parent chain back to `/`. Cardinal uses this form for the initial cache build so absolute paths can be reconstructed cheaply from parent pointers.

Both functions return:
- `Some(Node)` for success
- `None` when the cancellation closure fires

If the root is missing or inaccessible, the walker still returns `Some(Node)` with empty children and `metadata = None`.

## Traversal behavior
For each visited path:
1. `symlink_metadata()` probes the node without following symlinks.
2. If the node is a directory:
   - increment `num_dirs`
   - enumerate children with `read_dir`
   - process entries in parallel via Rayon `par_bridge()`
3. For each child entry:
   - abort the whole branch if `cancel()` returns true
   - skip ignored descendants with `should_ignore_path(...)`
   - use `DirEntry::file_type()` to avoid extra metadata calls when deciding whether to recurse
   - recurse only into real directories
   - treat non-directories, including symlink entries, as leaf nodes
4. Sort children by name before returning the `Node`

When `need_metadata` is `true`, leaf entries fetch `entry.metadata()` and directory/root nodes reuse `symlink_metadata()`-derived information. When it is `false`, many leaves keep `metadata = None` for later lazy hydration.

## Error handling
- `metadata_of_path()` retries only `ErrorKind::Interrupted`.
- `NotFound` becomes `None`.
- Other metadata failures keep the node but drop metadata.
- `read_dir` failures currently produce an empty child list rather than aborting the full walk.

## Integration notes
- Initial full builds typically use `need_metadata = false` for speed.
- Incremental subtree rescans in `SearchCache::scan_path_recursive(...)` use `walk_it_without_root_chain(...)` with `need_metadata = true`.
- `num_files` and `num_dirs` are polled by the Tauri backend to emit `status_bar_update` progress during long scans.
