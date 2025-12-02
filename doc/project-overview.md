# Cardinal Project Overview

Cardinal is a macOS desktop search app built with a React/Tauri frontend and a Rust backend. This document explains how the pieces fit together so contributors can navigate, extend, and debug the codebase quickly.

## High-level architecture
- **Frontend (cardinal/)**: React + Vite UI. Talks to Tauri commands for search, metadata, window control, and previews. Initializes menu/tray, global shortcuts, and theme preference.
- **Desktop shell (cardinal/src-tauri/)**: Tauri entrypoint. Registers plugins (global shortcuts, window state, opener, macOS permissions, prevent-default in prod), wires commands, owns app lifecycle, and spawns the background logic thread.
- **Search engine (search-cache/)**: Maintains an in-memory index of the filesystem (slab-based storage with compact nodes, name index backed by an interned name pool, lazy metadata cache), persists to disk, and serves queries with highlighting and cancellation support.
- **Filesystem events (cardinal-sdk/)**: Thin wrapper over macOS FSEvents providing `EventWatcher` and event flags; used to keep the index in sync.
- **Icon extraction (fs-icon/)**: macOS icon retrieval via Quick Look / NSWorkspace, returning base64-encoded PNGs.

Overall architecture:
```text
 ┌───────────────────────────────────────────────────────┐
 │                    React frontend                     │
 │  - Search bar / filters                               │
 │  - VirtualList results                                │
 │  - Status bar / overlays                              │
 └───────────────▲─────────────────────┬─────────────────┘
                 │ invoke()            │ listen()
                 │                     │ (status_bar_update,
                 │                     │  app_lifecycle_state,
                 │                     │  icon_update, quick_launch, ...)
 ┌───────────────┴─────────────────────▼─────────────────┐
 │                    Tauri shell                        │
 │  - Commands: search, get_nodes_info,                  │
 │    update_icon_viewport, open_in_finder, ...          │
 │  - Plugins: global-shortcut, window-state,            │
 │    opener, macOS permissions, prevent-default         │
 └───────────────▲─────────────────────┬─────────────────┘
                 │ crossbeam channels  │
                 │ (search_tx, node_info_tx,             │
                 │  icon_viewport_tx, rescan_tx, ...)    │
 ┌───────────────┴─────────────────────▼─────────────────┐
 │               Background logic thread                  │
 │  - SearchCache (slab + name index + metadata)         │
 │  - EventWatcher (FSEvents)                            │
 │  - fswalk (initial walk / rescans)                    │
 │  - fs-icon (NSWorkspace + QuickLook icons)            │
 └───────────────────────────────────────────────────────┘
```

## Key data flow
1) **Startup**: `cardinal/src-tauri/src/lib.rs` builds the Tauri app, registers plugins, and constructs channels for search, node info, icon viewport, rescans, and shutdown. It spawns a background thread via `run_background_event_loop`.
2) **Index hydration**: The background loop loads or walks the filesystem (`search_cache::SearchCache::walk_fs_with_ignore` and persistence helpers). It emits status updates to the UI while scanning.
3) **Live updates**: `EventWatcher` streams FSEvents. The background loop feeds them to the cache; a rescan is triggered on error conditions or when flags/paths suggest the index may be stale. New events are batched to the frontend for recent-activity views.
4) **Queries**: UI sends the `search` command with options and a cancellation token version. The background loop runs `cache.search_with_options`, returning result slab indices and highlights. `update_icon_viewport` prompts icon loads for visible rows; icons are emitted back over an event channel.
5) **Metadata & icons**: `get_nodes_info` expands slab indices into paths/metadata and attaches icons via `fs_icon::icon_of_path_ns`. For grid/list views, additional icons are fetched with Quick Look (`icon_of_path_ql`) on background threads.
6) **Window control & UX**: Commands (`activate_main_window`, `toggle_main_window`, `hide_main_window`) manage visibility. Quick Look (`toggle_quicklook`/`update_quicklook`/`close_quicklook`) talks directly to `QLPreviewPanel` via `quicklook.rs`, while Finder/open actions (`open_in_finder`, `open_path`) use the macOS `open` binary. The single global shortcut (`Cmd+Shift+Space`) is registered through `@tauri-apps/plugin-global-shortcut`.

## Frontend layout (cardinal/)
- `src/main.tsx`: Bootstraps theme, tray, app menu, and global shortcuts; renders `<App />`.
- `src/menu.ts` / `src/tray.ts`: Build native menu and status bar/tray; menu reacts to locale changes.
- `src/utils/globalShortcuts.ts`: Registers the quick-launch accelerator and logs registration failures.
- `src/components/`: UI building blocks (virtualized list, status bar, search controls, etc.).
- `src/hooks/`: Client-side hooks (e.g., context menu, icon viewport tracking).
- `src/i18n/`: Localization setup.
- `public/` and Vite config live under `cardinal/` per standard Vite/Tauri layout.

## Backend layout (Rust workspace)
- `cardinal/src-tauri/src/lib.rs`: Tauri bootstrap and plugin wiring.
- `cardinal/src-tauri/src/commands.rs`: Tauri command handlers (search, node info, rescan, window ops, Quick Look/Finder).
- `cardinal/src-tauri/src/background.rs`: Background event loop for queries, FSEvents ingestion, rescans, icon loading, and status updates.
- `cardinal/src-tauri/src/lifecycle.rs`: Tracks app lifecycle state and persistence of readiness.
- `cardinal/src-tauri/src/window_controls.rs`: Abstractions for showing/hiding/activating the main window.
- `cardinal/src-tauri/src/quicklook.rs`: Owns the native `QLPreviewPanel` bridge used by `toggle_quicklook`/`update_quicklook`/`close_quicklook`.
- `search-cache/`: Core index, query engine, persistence, highlighting, and slab management.
- `fswalk/`: Filesystem walker used by the cache to build initial state.
- `cardinal-sdk/`: FSEvents bindings and helpers.
- `fs-icon/`: Icon extraction via macOS APIs.
- `query-segmentation/`: Parses slash-delimited search tokens into prefix/suffix/exact/substr segments.
- `cardinal-syntax/`: Everything-style query parser (operators, filters, grouping).
- `search-cancel/`: Cancellation token with versioning for aborting stale searches.
- `namepool/`: Process-wide string interner feeding the slab + name index.
- `slab-mmap/`: Memory-mapped slab allocator backing `search-cache`'s `ThinSlab`.
- `lsf/`: CLI utility that exercises `SearchCache` for manual indexing/search experiments.
- `was/`: CLI that streams macOS FSEvents via `cardinal-sdk`, useful for debugging watcher behavior.

## Runtime behavior and UX notes
- **Search semantics**: Combines Everything-like filters (extensions, size, content, boolean) with path-segmentation support (leading/trailing slashes enforce prefix/suffix/exact). Highlights returned with results guide UI rendering.
- **Performance**: Indexing runs in the background and reports progress. The initial walk avoids per-file `lstat` calls and defers metadata until filters need it; FSEvents keep the index current, with targeted rescans when necessary. Icons are lazy-loaded for visible rows and throttled via viewport requests.
- **Permissions**: macOS permissions plugin is initialized; prevent-default is enabled in non-dev builds to keep the app resident. Quick Look uses the native `QLPreviewPanel` integration; Finder/open actions still shell out to `open`.
- **Global shortcuts**: Primary `Cmd+Shift+Space`; failures are logged (there is no automatic fallback accelerator today).

## Development workflow
- **Rust**: `cargo check --workspace`, `cargo test --workspace`, `cargo clippy --workspace --all-targets`, `cargo fmt --all`. Toolchain pinned via `rust-toolchain.toml` (`nightly-2025-05-09`).
- **Frontend**: `cd cardinal && npm ci`; `npm run dev` (Vite), `npm run tauri dev -- --release --features dev`; `npm run build` or `npm run tauri build` for production.
 - **Testing strategy**: Unit tests live beside code; cross-crate tests in each crate’s `tests/`. Frontend uses Vitest/JSDOM. Performance and UI regressions should be checked after `npm run build`.

## Debugging tips
- Watch Tauri logs (tracing) for lifecycle, search, and rescan events.
- Conflicts on global shortcuts manifest as registration failures logged by the UI; there is no automatic fallback shortcut today.
- Icon loading failures won’t block search; they are best-effort and logged per item.

## Release considerations
- Avoid committing generated assets (`target/`, `cardinal/dist/`, vendor bundles).
- Follow Conventional Commits and include executed cargo/npm commands in PRs.
- Capture UI changes with screenshots and call out risks around indexing throughput, search latency, and icon extraction.
