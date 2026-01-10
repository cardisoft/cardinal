# FSWalk (Filesystem Walker)

This chapter documents the `fswalk/` crate, which builds the initial directory tree Cardinal indexes.

---

## Node and metadata

`fswalk::Node` is a simple recursive tree:
```text
Node {
  name: Box<str>,
  metadata: Option<NodeMetadata>,
  children: Vec<Node>,
}
```

Example tree:
```text
/Users/demo
├─ Projects
│  ├─ cardinal
│  │  ├─ Cargo.toml
│  │  └─ README.md
│  └─ sandbox
└─ Downloads
   └─ archive.zip
```

`NodeMetadata` is a compact snapshot of filesystem attributes:
- `r#type: NodeFileType` — file, directory, symlink, or unknown.
- `size: u64` — byte size (from `Metadata::size()`).
- `ctime: Option<NonZeroU64>` — creation time seconds since UNIX epoch.
- `mtime: Option<NonZeroU64>` — modification time seconds since UNIX epoch.

`NodeFileType` is a `repr(u8)` enum so it compresses well when serialized.

---

## WalkData configuration

`WalkData<'w>` holds traversal state and configuration:
- `num_files: AtomicUsize` — total files visited.
- `num_dirs: AtomicUsize` — total directories visited.
- `cancel: Option<&'w AtomicBool>` — optional cancellation flag.
- `root_path: &'w Path` — root directory for the walk.
- `ignore_directories: &'w [PathBuf]` — directories to skip.
- `need_metadata: bool` — whether to gather per-file `Metadata`.

Constructors:
- `WalkData::simple(root_path, need_metadata)` — minimal config, no ignore list or cancellation.
- `WalkData::new(root_path, ignore_directories, need_metadata, cancel)` — full control.

`SearchCache` uses `WalkData` to drive progress bars, cancellation, and ignore lists.

---

## Traversal algorithm

Entry point: `walk_it(walk_data: &WalkData) -> Option<Node>`.

High-level steps:
1. Check `ignore_directories`; abort traversal under ignored roots.
2. Fetch `symlink_metadata` for the current path:
   - `NotFound` → skip entirely.
   - Other errors → optionally retry via `handle_error_and_retry`.
3. If metadata reports a directory:
   - Increment `num_dirs`.
   - Call `read_dir` and process entries in parallel using `rayon::ParallelBridge`.
   - For each entry:
     - Check `cancel` flag periodically; abort current branch if set.
     - Use `entry.file_type()` (backed by `dirent.d_type`) to distinguish files vs directories without extra `lstat` calls.
     - Skip symlinks; recurse into subdirectories with `walk`.
     - For files:
       - Increment `num_files`.
       - Collect `NodeMetadata` only when `need_metadata` is `true`.
4. If not a directory:
   - Treat as a file and increment `num_files`.
5. After collecting children, sort them by `name` for deterministic ordering.
6. Build the root `Node` with the current path’s file name and optional metadata.

Cancellation:
- At several points, `cancel.load(Ordering::Relaxed)` is checked.
- If cancelled, `walk` returns `None`, signalling the caller to abandon this traversal.

---

## Error handling

`handle_error_and_retry` currently retries only on `ErrorKind::Interrupted`, mirroring POSIX “try again” semantics.

- For `read_dir` errors, a retry falls back to a recursive `walk` on the same path.
- For per-entry errors, retry may re-enter `walk` on the parent path.

All other unrecoverable errors cause that branch to be recorded as a node with minimal or missing metadata, rather than aborting the entire walk.

---

## Integration notes

- `SearchCache` consumes the `Node` tree to construct `FileNodes` and the slab.
- `WalkData::num_files` and `num_dirs` are used by the background event loop to emit `status_bar_update` progress during scans and rescans.
- Initial full scans typically run with `need_metadata = false` so traversal can avoid `lstat` for leaf files; metadata is lazily fetched later by the cache when filters require it.
- Ignore lists are expressed as full paths; make sure they are canonicalized consistently with the watch root.
