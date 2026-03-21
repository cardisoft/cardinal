# IPC Commands

This chapter documents the Tauri commands exposed from `cardinal/src-tauri/src/commands.rs`.

## Search and data

| Command | Purpose | Notes |
| --- | --- | --- |
| `search(query, options?, version)` | Run a query and return `{ results, highlights }` | Uses `CancellationToken::new(version)`; cancelled searches become empty result lists at the command layer. |
| `get_nodes_info(results, include_icons?)` | Expand slab indices into `{ path, metadata, icon }` | `include_icons` defaults to `true`; icons come from `fs_icon::icon_of_path_ns`. |
| `get_sorted_view(results, sort?)` | Sort the current result set on the backend | Reuses a small `SearchState` cache when the same slab-index slice is sorted repeatedly. |
| `update_icon_viewport(id, viewport)` | Send the visible slab indices for thumbnail prefetch | Fire-and-forget; the backend responds via `icon_update` events. |
| `trigger_rescan()` | Request a full rebuild of the current watch root | Uses `CancellationToken::new_scan()`. |
| `set_watch_config(watch_root, ignore_paths)` | Replace the watch root / ignore list | Input is normalized to absolute paths; `/System/Volumes/Data` is always appended. |

## Quick Look

| Command | Purpose | Notes |
| --- | --- | --- |
| `toggle_quicklook(items)` | Open/close the native `QLPreviewPanel` | `items` may include row screen rects and transition images. |
| `update_quicklook(items)` | Refresh the current panel contents | Used when selection changes while Quick Look stays open. |
| `close_quicklook()` | Close the panel explicitly | Typically called when leaving the files tab or hiding the window. |

## Window, lifecycle, and startup

| Command | Purpose | Notes |
| --- | --- | --- |
| `get_app_status()` | Return the current lifecycle string | Returns `"Initializing"`, `"Updating"`, or `"Ready"`. |
| `start_logic(watch_root, ignore_paths)` | One-shot gate that lets the background thread start | Sent only after Full Disk Access is granted. |
| `hide_main_window()` | Hide the main window | Also nudges the background flush state. |
| `activate_main_window()` | Show, unminimize, and focus the main window | Used by menu and tray actions. |
| `toggle_main_window()` | Toggle visibility/focus | Emits `quick_launch` when showing the window. |
| `set_tray_activation_policy(enabled)` | Switch activation policy between `Regular` and `Accessory` | Also re-activates the main window after the policy change. |

## Shell and system integration

| Command | Purpose | Notes |
| --- | --- | --- |
| `open_in_finder(path)` | Reveal a path in Finder | Runs `open -R <path>`. |
| `open_path(path)` | Open a path with the default macOS handler | Runs `open <path>`. |
| `copy_files_to_clipboard(paths)` | Put file URLs plus a joined path string on the pasteboard | Uses Cocoa `NSPasteboard`, not shell utilities. |

## Events emitted alongside commands
- `status_bar_update`
- `app_lifecycle_state`
- `quick_launch`
- `fs_events_batch`
- `icon_update`
- `quicklook-keydown`

## Guidelines for new commands
- Prefer request/response commands for bounded work and Tauri events for push-style updates.
- Keep long-running work versioned or cancellable.
- Normalize file-system inputs on the Rust side so the UI can stay permissive.
