# Search Cancellation

This chapter describes the `search-cancel/` crate and how it keeps long-running searches responsive.

---

## CancellationToken

`CancellationToken` is a lightweight handle used by loops in `search-cache`, `namepool`, and related crates to decide when to abort work.

Key pieces:
- `ACTIVE_SEARCH_VERSION: AtomicU64` — global active search version.
- `CANCEL_CHECK_INTERVAL: usize` — how often loops check for cancellation.
- `CancellationToken::new(version)`:
  - Stores `version` into `ACTIVE_SEARCH_VERSION` (SeqCst).
  - Returns a token capturing that version.
- `CancellationToken::is_cancelled()`:
  - Compares the captured version to `ACTIVE_SEARCH_VERSION` with relaxed loads.
  - Returns `Some(())` while the token remains active and `None` once a newer search starts, so callers can
    simply use `token.is_cancelled()?;` to propagate cancellation.
- `CancellationToken::is_cancelled_sparse(counter)`:
  - Checks cancellation only every `CANCEL_CHECK_INTERVAL` iterations to amortize the cost, returning
    `None` when the token has been cancelled at one of those checkpoints.

`CancellationToken::noop()`:
- Uses a private, static `AtomicU64` that never changes.
- Suitable for tests or paths that should never cancel.

---

## How it is used

Typical pattern inside loops:
```rust
for (i, item) in items.iter().enumerate() {
    token.is_cancelled_sparse(i)?;
    // ... work ...
}
```

Examples:
- `SearchCache::all_subnodes_recursive` checks the token while traversing the subtree.
- `NameIndex::all_indices` and `NamePool` search methods bail early when a new search supersedes the current one.

---

## Backend/Frontend coordination

- The frontend includes a monotonically increasing `version` in each `search` request.
- The Tauri backend creates a `CancellationToken::new(version)` for each search and passes it into `SearchCache::search_with_options`.
- If a new search starts with a higher version:
- The prior token becomes cancelled.
- Long-running loops periodically hit an `is_cancelled()` / `is_cancelled_sparse()` checkpoint that yields
  `None` and exit, returning `SearchOutcome { nodes: None, .. }` inside the engine.
  - The Tauri command handler converts this to an empty `results` list, and the React side also uses its own `searchVersionRef` to discard responses for older versions.

---

## Extension tips

- When you add new long-running loops (e.g., content scanning or complex sorting), integrate `CancellationToken` checks using `CANCEL_CHECK_INTERVAL` as a guide.
- Avoid global state beyond the single `ACTIVE_SEARCH_VERSION`; tokens should be passed explicitly where needed.
- Prefer returning `Option`/`Result` that can encode cancellation distinctly from other failures so callers can distinguish “stale search” from “real error”.
