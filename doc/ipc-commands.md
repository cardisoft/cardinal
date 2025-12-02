# IPC Commands

This chapter documents the Tauri commands exposed to the frontend.

---

## Search and data

| Command | Purpose | Used by |
| --- | --- | --- |
| `search(query, options, version)` | Run search with cancellation token; returns `{ results: Vec<SlabIndex>, highlights }` | search bar / main app |
| `get_nodes_info(results)` | Expand slab indices to `{ path, metadata, icon }` using NSWorkspace | `useDataLoader` |
| `update_icon_viewport(id, viewport)` | Notify backend of visible rows for QuickLook icon prefetch | `useIconViewport` |
| `trigger_rescan()` | Force a full rescan | status bar / settings |

---

## Shell integration

| Command | Purpose | Used by |
| --- | --- | --- |
| `open_in_finder(path)` | Reveal file in Finder | context menu |
| `preview_with_quicklook(path)` | Quick Look preview | `Space` keybind |

---

## Lifecycle and window control

| Command | Purpose | Used by |
| --- | --- | --- |
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
