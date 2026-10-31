# Changelog

## Unreleased
- Fix the ignore-path migration never running: it used a single "user edited" flag that a Preferences save set, so `/Volumes` was never added and the app indexed every NFS/SMB share (2.9M files and an endless "Updating"). Defaults are now tracked individually, so a new default is added once and a later deletion stays deleted.
- Detect `HistoryDone` with `contains` instead of exact equality, so the lifecycle can reach `Ready` even when the replay's final event carries other flag bits.
- Keep the background thread alive: a panicking search is converted into an error result, event-batch handling is guarded, and channel sends/emits no longer panic. Previously one panic killed the thread and every later search failed with `RecvError`.
- Write panics to `~/Library/Logs/Cardinal/panic.log` (a Finder-launched bundle has no stderr, so crashes left no trace).
- Report a stopped background thread as an actionable message instead of `RecvError`.
- Add "Open With" to the file context menu: lists every application LaunchServices can use for the selection (default handler first) and can launch any of them by bundle id, falling back to the bundle path.
- Ignore the nested `winnfsd/` checkout so its files can never be committed here.
- Reopening a hidden window (Dock click, tray, Cmd+Shift+Space) now shows it on the Desktop you are currently on instead of yanking you back to the Desktop where it was last visible.
- Bookmarks window follows the theme, font size and language immediately: UI preferences are now broadcast between the two windows instead of only being read when a window boots.
- The bookmarks manager (Cmd+Shift+B) is now its own window: drag it anywhere on screen, including outside the Cardinal window, so it no longer covers the result list.
- Bookmarks changed in one window (Cmd+S, CSV import) appear immediately in the other.
- Fix: "search in new tab" opened one tab per re-render. The bookmark-open effect depended on a callback that `useSearchTabs` rebuilt every render, and because `listen()` resolves asynchronously the cleanup could not unsubscribe — each render leaked a listener.
- Folder scope is now per search tab: typing a scope in one tab no longer narrows every other tab (including tabs opened from a bookmark).
- Tiled view fills each name line greedily: a name like `白咲碧122614_0...` used to leave `白咲碧` alone on line 1 because `overflow-wrap: anywhere` moves a whole word down before splitting it.
- Fix imported bookmark lists collapsing into 2px lines: rows are flex items inside a scroll container, so `overflow: hidden` zeroed their automatic minimum size and let them shrink.
- Tiled view: names wrap to at most three lines of six CJK characters with an ellipsis. The icon box and the label box are both fixed height, so icons never shift between rows and the last text line of every tile shares one baseline.
- Tiled view: square document/folder icons (which fill the icon box) get two text lines; 16:9 video covers keep three.
- Merge newly added default ignore paths into lists saved by older releases. Without this, upgrades kept indexing `/Volumes` (network mounts), which made the first scan run for hours.
- Persist folder-scope indexes (`scopes/scope-<hash>.db`) and reuse them for 30 minutes, so searching a slow network folder is instant on the next visit instead of re-walking it.
- Load and build the index on a worker thread so searches answer immediately instead of queueing behind the initial walk.
- Show index-build progress in the results pane, with a hint that a full folder path can be searched right away.
- Allow searching a literal folder path through the folder scope box while the index is still building (temporary per-folder index).
- Re-run the current query once the full index is ready so result rows always belong to the active cache.
- Never persist a noop or temporary folder-scope index over the real cache snapshot.
- Persist the first complete index right after it is built, so quitting before the app reaches `Ready` no longer forces a full re-walk on the next launch.

## 0.1.23 — 2026-03-25
- Reduce power consumption by expanding the default ignored paths to cover more macOS cache, log, metadata, and runtime directories.
- Further reduce background work by making the filesystem event watcher honor ignored paths.
- Improve Unicode path matching by searching both NFC and NFD-equivalent query forms.

## 0.1.22 — 2026-03-08
- Add `~/Library/CloudStorage` to default ignored paths to avoid initial indexing delays
- Make index building cancellable
- Fix unexpected indexing of files in ignored directories
- Fix stale selection affecting context menu targets
- Fix lost sort state and incorrect tooltip when search results are empty
- Speed up scanning/search by removing redundant index-map and ignore-path checks

## 0.1.21 — 2026-01-23
- Add shortcut `cmd+shift+c` for copy-paths
- Allow partial-quoted queries: `"Application Support"/**`
- Add drag-and-drop support for search input
- Faster wildcard search.
- Exclude more cloud storage paths from icon generation for better performance.
- Better experience for non-root monitor path
- Better experience for panel switching
- Fix duplicate results of globstar `**` search

## 0.1.20 — 2026-01-11
- Improve context menu for multiple selections.
- Add copy file(s) to clipboard support.
- Add shorthand aliases for `tag` and `infolder` filters.
- Hide the dock icon when the tray icon is enabled.

## 0.1.19 — 2026-01-10
- Add watch roots and ignore paths to settings.
- Avoid unnecessary rescan on FSEvent::Rescan.
- Handle Enter key in search input.

## 0.1.18 — 2025-12-15
- Add option for tray icon and defaults to disable
- Persistent cache when idle or enter background.
- Improve i18n locale detection and add zh-TW translation.
- Make tags filtering accepts multiple value.
- Supports `shift+arrow` to select multiple rows.
- Various UI/UX improvements and bug fixes.

## 0.1.17 — 2025-12-10
- Add `tag:` filter support so Finder tags can scope searches.
- Better i18n support.
- Improve the column resizing experience
- Improve the sorting experience

## 0.1.16 — 2025-12-08
- Improve sort order so directories are prioritized and folder size ranking stays stable.
- Refine selection handling for smoother keyboard and pointer interactions.
- Add an event column to the events panel for better debugging context.
- Fix the cursor state resetting incorrectly after Quick Look opens.

## 0.1.15 — 2025-12-06
- Implement double asterisk `**` in glob search.
- Implement history navigation with `ArrowUp`/`ArrowDown` for search bar.
- Add `Cmd+O` shortcut for file opening.
- Refined QuickLook animation positioning logic to better handle multiple monitor setups.
- Refined file row selection handling.

## 0.1.14 — 2025-12-03
- Make the results sortable(by name, path, size, create_time, modify_time)

## 0.1.13 — 2025-12-01
- 30% lower memory usage
- Quick Look is now fully native, with multi-file previews, smoother animations, and better alignment with macOS expectations. Thanks for [@Denis Stoyanov](https://github.com/xgrommx) for the help!
- Support `~` expansion in query path and filter
- Fix the database path so cache files land in the correct app config directory.

## 0.1.12 — 2025-11-27
- Allow double-clicking a result row to open the file immediately.
- Support wildcards in multi-path-segment queries for more flexible searches.

## 0.1.11 — 2025-11-25
- Implement `content:`, `nosubfolders:`filter
- Improve file selection and drag-drop support
- Cleaner app menu and context menu
- make ESC hide main window

## 0.1.10 — 2025-11-19
- Added new metadata filters (`dm:`, `dc:`, `type:`, `audio:`, `video:`, `doc:`, `exe:`, `size:`) for more precise searches.
- Reworked the parser/optimizer pipeline to flatten redundant AND/OR groups, collapse empty expressions, and reorder metadata filters for faster searching.
- Use the native context menu on right-click for a more consistent feel on macOS.

## 0.1.9 — 2025-11-17
- Speedup `parent:` and `infolder:` filters.

## 0.1.8 — 2025-11-16
- Cardinal now fully supports the "Everything syntax"(AND/OR/NOT, parentheses, quoted phrases, wildcards).
- Removed the legacy regex toggle and unified the search bar, hooks, and IPC payloads around the new parser pipeline.
- Highlight of search results was improved.
- Enhance show/hide shortcut.

## 0.1.7 — 2025-11-12
- Added a cancellable search pipeline for a more responsive search experience.
- Refined selected row styling with dedicated text color tokens for better contrast in both themes.

## 0.1.6 — 2025-11-11
- Further optimized search execution and reorganized the search cache for faster lookups.

## 0.1.5 — 2025-11-09
- search-cache: widen `NameAndParent` filename length tracking from `u8` to `u32` to handle very long paths without truncation or panic.

## 0.1.4 — 2025-11-09
- Fix i18n support for tray menu.
- Rescans now clickable while updating cache.

## 0.1.3 — 2025-11-08
- Added keyboard shortcuts for Quick Look (Space), Finder reveal (Cmd+R), copy path (Cmd+C), and refocusing search (Cmd+F).
- The search field auto-focuses after launch and whenever the quick-launch shortcut summons Cardinal.
- i18n: add Ukrainian language support and translations

## 0.1.2 — 2025-11-07
- feat(shortcut&tray): support global shortcut to toggle cardinal by [@Binlogo](https://github.com/Binlogo)
- feat(theme): implement theme switching functionality with user preferences
- feat(context-menu): add “copy filename” option and update translations
- feat(i18n): add Russian language support and translations

## 0.1.1 — 2025-11-07
- Fixes iCloud download triggered by thumbnail generation.

## 0.1.0 — 2025-11-07
