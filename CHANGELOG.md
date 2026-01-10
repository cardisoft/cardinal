# Changelog

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
