# Search Cancellation

`search-cancel/` provides the versioned cancellation token used by both query evaluation and long-running scans.

## Global versions
- `ACTIVE_SEARCH_VERSION: AtomicU64`
- `ACTIVE_SCAN_VERSION: AtomicU64`
- `CANCEL_CHECK_INTERVAL: usize = 0x10000`

Search and scan cancellation are intentionally separate so a new rescan does not cancel an in-flight search token and vice versa.

## CancellationToken
`CancellationToken` stores:
- a pointer to the relevant global atomic
- the version captured at creation time

Constructors:
- `CancellationToken::new(version)` -> marks that search version as current
- `CancellationToken::new_scan()` -> increments and captures the scan version
- `CancellationToken::noop()` -> token backed by a private static atomic that never changes

The naming is slightly counterintuitive:
- `is_cancelled()` returns `Some(())` while the token is still active
- it returns `None` once the token has been superseded
- `is_cancelled_sparse(counter)` only checks every `CANCEL_CHECK_INTERVAL` iterations

That shape lets callers write:
```rust
token.is_cancelled_sparse(i)?;
```
and naturally bubble cancellation through `Option`.

## Where Cardinal uses it
- Frontend search requests pass an increasing `version` to the `search` Tauri command.
- `commands.rs` turns that into `CancellationToken::new(version)`.
- `SearchCache`, `NamePool`, and other loops periodically check the token and return `None` when a newer search has superseded them.
- `trigger_rescan()` and `set_watch_config(...)` use `CancellationToken::new_scan()` to cancel older rebuilds.

## Behavioral contract
- Search cancellation surfaces as `SearchOutcome { nodes: None, .. }`.
- The Tauri command handler translates that to an empty result list for the frontend.
- React still keeps its own `searchVersionRef` guard, because the UI can receive stale responses even if the engine already did the right thing.

## Guidance for new code
- Use `new_scan()` for scan-like work and `new(version)` for search-like work.
- Add sparse checks inside any loop that can touch many nodes, files, or bytes.
- Prefer APIs that can distinguish cancellation from "no result" or "hard error"; `Option` or a dedicated enum both work.
