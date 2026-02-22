# Ignore Patterns (`.gitignore` Semantics)

This document describes how Cardinal interprets ignore patterns from Preferences.

The behavior is intended to follow `.gitignore` pattern semantics for matching rules.

---

## Root Definition

Ignore patterns are evaluated relative to the configured **Monitor root path** (`watchRoot`).

Think of it as:
- One virtual `.gitignore` file located at the watch root.
- All preferences ignore lines are entries in that one file.

Examples:
- If watch root is `/`, pattern `/Volumes` matches `/Volumes`.
- If watch root is `/Users/madda`, pattern `/Library` matches `/Users/madda/Library` (not system `/Library`).

---

## Input Source

Ignore rules come from the Preferences textarea, one line per rule.

Current behavior:
- Empty/whitespace-only lines are ignored.
- Non-empty lines are preserved and interpreted using `.gitignore` rule syntax.

---

## Matching Rules

The following semantics apply:

1. Order matters: last matching rule wins.
2. `#` starts a comment.
3. `!` negates (unignores) a previous ignore match.
4. Leading `/` anchors to the watch root.
5. Trailing `/` means directory-only match.
6. No `/` in a pattern means basename-style match anywhere under the watch root.
7. `*`, `?`, `[abc]`, and `**` behave like `.gitignore`.

Important example:
- `**/Xcode.app/**` ignores descendants of `Xcode.app`, but not `Xcode.app` itself.
- `/Xcode.app/` ignores `Xcode.app` and everything inside it (when anchored at root).
- `Xcode.app` (no slash) matches `Xcode.app` directories/files by basename anywhere.

---

## Common Pattern Examples

1. Ignore any `node_modules` directory and its contents:
`node_modules`

2. Ignore descendants inside any `Xcode.app` bundle, but keep the bundle directory node:
`**/Xcode.app/**`

3. Ignore only root-level `build` under watch root:
`/build/`

4. Ignore all `.pyc` files:
`*.pyc`

5. Ignore all `.DS_Store` files:
`.DS_Store`

6. Ignore everything in `.cache`, then re-include one file:
`.cache/`
`!.cache/keep.me`

---

## Notes on Scope

Cardinal currently uses a single global ruleset (from Preferences) rooted at `watchRoot`.

This means:
- Pattern syntax matches `.gitignore` semantics.
- Cardinal does **not** recursively read `.gitignore` files from each directory in the filesystem tree.

---

## Built-in Default Rules

Cardinal ships with the following default ignore entries:

```gitignore
# Root-anchored system paths
/Volumes/
/cores/
/dev/
/private/
/System/Applications/**/Contents/Resources/
/System/Volumes/
/usr/share/
/xarts/

# Common project/build caches
node_modules/
.next/
.bun/
.pnpm/
**/.local/fsindex*

# Application-specific heavy caches
**/com.docker.docker/Data/
**/Firefox/Profiles/**/sessionstore-backups/
**/Firefox/Profiles/**/storage/default/
**/Firefox/Profiles/**/storage/permanent/
**/Google/Chrome*/Cache/
**/Google/Chrome*/leveldb/
**/IconJar*/Backups/
**/Sublime Text */Index/
**/var/postgres/base/
**/var/postgres/pg_stat_tmp/
**/var/postgres/pg_wal/
**/Spotify/Users/*/pending-messages*

# Root user-library indexing data
/Library/Biome/
/Library/DuetExpertCenter/

# Basename folders to ignore anywhere
.cache/
.cocoapods/
.git/
.opam/
__pycache__/
Cache/
Caches/
doc/
Xcode.app/
wharf/
Index.noindex/
TextIndex/
io.tailscale.ipn.macos/
.stversions/

# File patterns
*.com.google.Chrome
*.pyc
.dat.nosync*
webappsstore.sqlite-wal
.DS_Store
```

These defaults are used for first run and reset behavior.

---

## Practical Guidance

1. Prefer basename rules for common folders:
`node_modules`, `.git`, `__pycache__`, `.cache`

2. Use `/...` when you want strict root anchoring.

3. Use `**/.../**` when you specifically want descendants but not the parent directory node.

4. Use negation (`!`) carefully: if a parent directory is excluded, you may need to unignore parent paths as well, same as `.gitignore`.
