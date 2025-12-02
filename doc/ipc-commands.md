# IPC Commands

This chapter documents the Tauri commands exposed to the frontend.

---

## Search and data

| Command | Purpose | Used by |
| --- | --- | --- |
| `search(query, options, version)` | Run search with cancellation token; returns `{ results: Vec<SlabIndex>, highlights }` | search bar / main app |
| `get_nodes_info(results, include_icons?)` | Expand slab indices to `{ path, metadata, icon }` with optional icon hydration | `useDataLoader` |
| `get_sorted_view(results, sort)` | Sort a slice of slab indices on the backend so the UI can render column sorts without moving data client-side | remote sort controls |
| `update_icon_viewport(id, viewport)` | Notify backend of visible rows for Quick Look icon prefetch | `useIconViewport` |
| `trigger_rescan()` | Force a full rescan and reset lifecycle state | status bar / settings |

---

## Shell integration

| Command | Purpose | Used by |
| --- | --- | --- |
| `open_in_finder(path)` | Reveal file in Finder | context menu |
| `open_path(path)` | Launch the path with the default handler (`open <path>`) | context menu / double-click |

---

## Quick Look

| Command | Purpose | Used by |
| --- | --- | --- |
| `toggle_quicklook(items)` | Open/close the native `QLPreviewPanel` for the current selection | `useQuickLook` + `Space` keybind |
| `update_quicklook(items)` | Refresh the panel contents when selection changes | `useQuickLook` |
| `close_quicklook()` | Close the panel explicitly (e.g., when the window hides) | `useQuickLook` |

---

## Lifecycle and window control

| Command | Purpose | Used by |
| --- | --- | --- |
| `request_app_exit()` | Exit app (sets `EXIT_REQUESTED`) | UI quit action |
| `hide_main_window()` | Hide window | Escape/menu |
| `activate_main_window()` | Show + focus | menu |
| `toggle_main_window()` | Toggle visibility and emit `quick_launch` | global shortcut |
| `get_app_status()` | Read lifecycle state | startup |
| `start_logic()` | Unblocks logic thread once permissions/UI are ready | startup |

---

## Guidelines for new commands
- Prefer idempotent commands where possible; side effects should be well-documented.
- Use structured payloads and return types so the TypeScript side can model them precisely.
- For streaming-style outputs, consider using Tauri events (like `icon_update`) instead of polling.
- Maintain versioning or token IDs for long-running work so the UI can discard stale responses.
