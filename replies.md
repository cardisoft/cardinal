# PR 152 Reviewer Replies

## `discussion_r2881615380` - `fswalk/src/lib.rs`

Thanks. I addressed this in the latest changes. `IgnoreMatcher::new` now logs each `GitignoreBuilder::add_line` failure with the offending pattern and root, and it also logs plus safely falls back when `build()` fails so invalid patterns no longer fail silently.

## `discussion_r2881615414` - `cardinal/src/utils/watchRoot.ts`

I handled this by removing the frontend ignore-pattern validation path instead of trying to duplicate `.gitignore` parsing in TypeScript. The UI now preserves raw lines verbatim, including blank lines and comments, and the Rust matcher build is the single source of truth. Invalid patterns are now surfaced through backend logging when the matcher is constructed.

## `discussion_r2881615425` - `cardinal/src/components/StatusBar.tsx`

Fixed. The export button label is now localized through `t('statusBar.aria.exportListedFiles')` for `title`, `aria-label`, and the screen-reader text.

## `discussion_r2881615437` - `cardinal/src/App.tsx`

Fixed. The export failure alert now uses a localized `t('exportListedFiles.failed')` string. There is not a shared toast or notification surface in this flow yet, so I kept the existing alert behavior but removed the hard-coded English copy.

## `discussion_r2881615448` - `cardinal/src/i18n/resources/en-US.json`

Adjusted. The ignore help text now says `.gitignore-style` and explicitly notes that blank lines and `#` comments are allowed. Since I removed the frontend ignore validation path, the stale "non-empty glob path" error copy is gone as well.

## `discussion_r2881615455` - `cardinal/src-tauri/src/commands.rs`

Fixed. `normalize_watch_config` now rewrites legacy absolute ignore entries that live under a non-`/` watch root into watch-root-relative patterns before they reach the matcher, including negated entries and the built-in default system ignore when it falls under the watch root. I also added unit coverage for user-supplied absolute entries, whole-watch-root ignores, and the `/System` default rewrite case.
