# Repository Guidelines

## Project Structure & Module Organization
- Desktop app lives in `cardinal/` (React UI in `src/`, Tauri/native glue in `src-tauri/`, build output in `cardinal/dist/`).
- Workspace crates (root `Cargo.toml`): `lsf/` (CLI), `cardinal-sdk/` (shared types), `fswalk/`, `fs-icon/`, `namepool/`, `query-segmentation/`, `search-cache/`, `search-cancel/`, `cardinal-syntax/`, `slab-mmap/` (mmap-backed slab for the cache), `was/` (CLI that streams FSEvents via the SDK).
- Tests sit next to code; cross-crate cases belong in each crate’s `tests/` directory. Generated outputs (`target/`, `cardinal/dist/`, vendor bundles) stay out of commits.
- Toolchain pinned via `rust-toolchain.toml` (`nightly-2025-05-09`); install with `rustup toolchain install nightly-2025-05-09`.

## Build, Test, and Development Commands
- `cargo check --workspace` — fast compile validation for all crates.
- `cargo test --workspace` or `cargo test -p <crate>` — run full or targeted suites.
- `cargo clippy --workspace --all-targets` — lint; fix or explain warnings. `cargo fmt --all` — enforce workspace rustfmt settings.
- Frontend: `cd cardinal && npm ci` (install), `npm run dev` (Vite), `npm run tauri dev -- --release --features dev` (desktop shell), `npm run build` (static bundle), `npm run tauri build` (release binaries).

## Coding Style & Naming Conventions
- Rust: grouped imports, 4-space indent, `snake_case` for modules/functions, `PascalCase` for types/traits. Prefer explicit modules, return `anyhow::Result` from fallible APIs, and use `tracing` for structured logs.
- React: components in `cardinal/src/components` use `PascalCase.tsx`; hooks/utilities export camelCase from kebab-case folders. Run `npm run format` or `npm run format:check` (Prettier). Keep Vite and `tsconfig*.json` settings intact.

## Testing Guidelines
- Co-locate unit tests; add crate-level integration tests for cross-cutting behaviors.
- Run `cargo test --workspace` after shared-crate changes; target `cargo test -p lsf` for query/indexing paths. Name tests for the behavior under test and include edge cases (search latency, indexing throughput, icon extraction).
- UI/performance: per `TESTING.md`, `npm run build`, then profile in Chrome DevTools/Safari and monitor FPS/memory regressions.

## Commit & Pull Request Guidelines
- Use Conventional Commits (`feat:`, `fix:`, `chore:`) with scopes when helpful (e.g., `feat(fs-icon): cache lookups`); squash WIP commits.
- PRs should list cargo/npm commands run, link issues, include UI screenshots when relevant, and call out risks (indexing throughput, search latency, icon extraction).
- Avoid committing generated or vendor outputs; note any intentional lint allowlists or config deviations in the PR.
