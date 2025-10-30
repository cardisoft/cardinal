# Repository Guidelines

## Project Structure & Module Organization
- `Cargo.toml` defines a Rust workspace spanning `search-cache/`, `fswalk/`, `cardinal-sdk/`, `fs-icon/`, `namepool/`, `query-segmentation/`, and the CLI entry `lsf/`.
- `cardinal/` hosts the Tauri + React desktop app; web assets live under `src/` and native glue in `src-tauri/`.
- Persistent caches and build artifacts stay in `target/` and `cardinal/dist/`; keep them out of commits. Core documentation lives under `doc/`.

## Build, Test, and Development Commands
- `cargo check --workspace` for a fast validation pass before pushing.
- `cargo test --workspace` to execute the full Rust suite; target crates with `-p search-cache`.
- `cargo clippy --workspace --all-targets -D warnings` keeps lint debt out.
- Front-end flows use `npm run dev` for Vite, `npm run tauri dev -- --release --features dev` for the desktop shell, and `npm run tauri build` when packaging.

## Coding Style & Naming Conventions
- Run `cargo fmt --all` to apply the pinned `rustfmt.toml` (4-space indent, grouped imports). Stick to `snake_case` modules/functions and `PascalCase` types.
- Prefer explicit module paths to glob imports; use `tracing` instead of `println!` for diagnostics, return `anyhow::Result` in fallible APIs.
- React components live in `cardinal/src/components` and follow `PascalCase.tsx`; hooks/utilities keep `camelCase` exports inside `kebab-case` directories.
- Execute `npm run format` prior to committing UI changes; keep CSS variables scoped in `cardinal/src/styles`.

## Testing Guidelines
- Co-locate Rust unit tests with implementations and reserve `tests/` directories for integration coverage.
- Run `cargo test --workspace` after touching shared crates, plus targeted cases (`cargo test -p lsf`) for query/indexing changes.
- Follow `doc/testing.md` for manual UI and performance checks; record FPS captures when tweaking virtualization or icon loading.

## Commit & Pull Request Guidelines
- Follow Conventional Commits (`feat:`, `fix:`, `chore:`). Include crate scopes (`feat(search-cache): ...`) when helpful.
- Squash exploratory commits before review. Reference related issues, note impacted crates, and attach screenshots or logs for UI and perf work.
- Document the commands you executed (cargo/npm) in the PR description and call out risk areas around indexing throughput or Tauri packaging.
