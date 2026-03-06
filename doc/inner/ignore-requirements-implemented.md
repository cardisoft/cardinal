# Ignore Requirements Implemented

This document summarizes the final ignore-pattern requirements implemented in this branch and reflected in current git changes.

## High-Level Requirements

1. `.gitignore`-style matching semantics were implemented for Preferences ignore rules.
2. Negation (`!`) is supported with the same practical limitation as `.gitignore`:
   - A file cannot be re-included if an ancestor directory was ignored and pruned.
3. Ignore matching is rooted to the configured `watchRoot` and remains consistent across:
   - Initial full indexing
   - Full rescans
   - Incremental (FSEvents-driven) rescans
4. Ignore evaluation is performance-oriented:
   - Ignored directories are pruned early (no subtree walk/read_dir for descendants).
   - Compiled ignore matcher state is reused instead of rebuilt on each incremental rescan.

## Technical Implementation Details

### Matching Engine and Semantics

- Switched ignore evaluation to `ignore::gitignore` rules and matching behavior.
- Rules are matched against paths relative to watch root.
- Matching checks both the path and its parents.

Relevant files:
- `fswalk/src/lib.rs`
- `fswalk/Cargo.toml`

### Negation Behavior and Pruned-Parent Limitation

- Negation rules (`!`) are supported.
- Incremental rescans now enforce the same pruned-parent behavior as full scans:
  - If an ancestor directory is ignored, descendant paths are not indexed even if a child rule negates them.

Relevant files:
- `search-cache/src/cache.rs`
- `fswalk/tests/deep_walk.rs`
- `search-cache/tests/fsevent_incremental_tests.rs`

### Compile Once, Reuse Always

- Added reusable compiled matcher plumbing via `Arc<IgnoreMatcher>`.
- `SearchCache` stores a compiled matcher and reuses it for:
  - Incremental subtree scans
  - Full rescans with reused `WalkData`

Relevant files:
- `fswalk/src/lib.rs`
- `search-cache/src/cache.rs`

### Early Directory Pruning

- The walker checks ignore status with directory/file context and prunes ignored directories before traversal.
- This avoids unnecessary `read_dir` and descendant work for ignored subtrees.

Relevant files:
- `fswalk/src/lib.rs`

### Root-Consistent Matching in Incremental Flow

- Incremental rescans use matcher state rooted at watch root.
- Added protection to skip incremental scan paths outside watch root.

Relevant files:
- `search-cache/src/cache.rs`

### UI/Input Pipeline Alignment

- Ignore lines are preserved as raw `.gitignore`-style lines (non-empty), not auto-expanded into custom glob wrappers.
- Validation/help text now aligns with `.gitignore` expectations.

Relevant files:
- `cardinal/src-tauri/src/commands.rs`
- `cardinal/src/utils/watchRoot.ts`
- `cardinal/src/components/PreferencesOverlay.tsx`
- `cardinal/src/hooks/useIgnorePaths.ts`
- `cardinal/src/i18n/resources/en-US.json`

### Verification in Current Git Changes

The current branch includes tests that lock in the implemented requirements:

- `.gitignore` semantics and glob behavior:
  - `fswalk/tests/deep_walk.rs`
- Incremental rescan behavior with ignores/negation:
  - `search-cache/tests/fsevent_incremental_tests.rs`

The implementation and tests are present in modified tracked files visible in `git status`.
