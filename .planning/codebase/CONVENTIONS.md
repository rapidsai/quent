# Coding Conventions

**Analysis Date:** 2026-07-08

## Naming Patterns

**Files:**
- Rust: `snake_case.rs` modules (e.g. `crates/instrumentation/src/observer.rs`, `crates/analyzer/src/resource/tree.rs`)
- TypeScript React components: `PascalCase.tsx` (e.g. `ui/src/components/QueryResourceTree.tsx`, `ui/src/components/ThemeToggle.tsx`)
- TypeScript utilities: `camelCase.ts` (e.g. `ui/packages/@quent/utils/src/parseJsonWithBigInt.ts`, `ui/src/atoms/resourceTree.ts`)
- React hooks: `useXxx.ts` (e.g. `ui/src/hooks/useExpandedIds.ts`, `ui/src/hooks/useQueryPlanVisualization.ts`)
- UI package directories: `kebab-case` (e.g. `ui/packages/@quent/components/src/pivot-table/`, `.../operator-timeline/`)
- Python tests: `test_*.py` (e.g. `domains/query_engine/tests/python/test_query_engine.py`)

**Crates:**
- All library crates prefixed `quent-` with kebab-case names (`quent-instrumentation`, `quent-exporter`, `quent-model-macros`); import paths use snake_case (`quent_events`, `quent_exporter_types`)
- Application-agnostic crates live in `crates/`; domain-specific crates in `domains/query_engine/`; examples in `examples/`

**Functions:**
- Rust: `snake_case`; fallible constructors named `try_new` (see `crates/exporter/ndjson/src/lib.rs`, `crates/instrumentation/src/context.rs`)
- TypeScript: `camelCase` (e.g. `formatDuration`, `renderWithRouter`); helper factories prefixed `create` (e.g. `createTestQueryClient` in `ui/src/test/test-utils.tsx`)

**Types:**
- Rust: `PascalCase` structs/enums/traits; options structs suffixed `Options` (`NdjsonExporterOptions`, `FileSystemExporterOptions`); error enums suffixed `Error`; result aliases suffixed `Result` (`ExporterResult`, `ImporterResult` in `crates/exporter/types/src/lib.rs`)
- Rust constants: `SCREAMING_SNAKE_CASE` (e.g. `const EXTENSION: &str = "ndjson"`)
- TypeScript: `PascalCase` interfaces/types (e.g. `RenderWithRouterOptions`, `QuantitySpec`)

## License Headers (mandatory)

Every source file (Rust, TS, Python, config JS/TS) starts with the SPDX header (enforced by `.github/workflows/license-check.yml`):

```text
// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0
```

Use `#` comment style for Python/TOML/YAML. See `CONTRIBUTING.md` for details.

## Code Style

**Rust Formatting:**
- `rustfmt` with default settings — no `rustfmt.toml` present anywhere in the repo
- Edition 2024, workspace resolver 3 (root `Cargo.toml`)
- Run: `pixi run cargo fmt --all` (CI checks with `-- --check`)

**Rust Linting:**
- `cargo clippy` with `-D warnings` — zero warnings allowed (CI: `.github/workflows/rust.yml`)
- Run: `pixi run cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
- `cargo deny check` for license/security audit (config: `deny.toml`)

**TypeScript Formatting (Prettier, `ui/.prettierrc`):**
- `semi: true`, `singleQuote: true`, `printWidth: 100`, `tabWidth: 2`, `trailingComma: "es5"`, `arrowParens: "avoid"`, `endOfLine: "lf"`
- Run: `pnpm format` / `pnpm format:check` from `ui/`

**TypeScript Linting (ESLint flat config, `ui/eslint.config.js`):**
- `@eslint/js` recommended + `typescript-eslint` recommended + `eslint-plugin-react-hooks` + `eslint-plugin-react-refresh` + prettier plugin
- `no-console: error` — only `console.warn` and `console.error` allowed
- `@typescript-eslint/no-unused-vars: error` — prefix intentionally unused args/vars with `_`
- Ignores: `dist`, `src/routeTree.gen.ts` (generated), `examples/**` (self-contained apps with own toolchains)

**Markdown:**
- `rumdl` lint runs in CI (`.github/workflows/markdown.yml`)

## Import Organization

**Rust order (observed, e.g. `crates/exporter/ndjson/src/lib.rs`):**
1. `std` imports
2. External + workspace crates (`quent_*` crates imported like any dependency)
3. Nested/grouped braces style: `use std::{io::{BufRead, BufReader}, path::PathBuf};`

**TypeScript:**
- External packages first, then internal aliases/relative imports
- Path alias: `@/` → `ui/src/` (configured in `ui/vitest.config.ts` and `ui/tsconfig.json`)
- Workspace packages imported as `@quent/client`, `@quent/components`, `@quent/hooks`, `@quent/utils` (pnpm workspace, `ui/pnpm-workspace.yaml`)

## Error Handling

**Rust:**
- `thiserror`-derived error enums per crate (workspace dep `thiserror = "2.0.17"`)
- Canonical pattern in `crates/exporter/types/src/lib.rs`: enum with specific variants plus `#[error(transparent)] Other(#[from] Box<dyn std::error::Error + Send + Sync>)` and an `other()` helper for wrapping
- Per-crate `Result` type aliases: `pub type ExporterResult<T> = std::result::Result<T, ExporterError>;`
- Fallible constructors return `Result` and are named `try_new`
- `let Some(x) = ... else { return Ok(()) };` let-else for early returns
- Document error cases with `/// # Errors` doc sections
- In iterators/streams where errors cannot propagate, log with `tracing::error!` and return `None` (see `NdjsonImporter::next` in `crates/exporter/ndjson/src/lib.rs`)

**TypeScript:**
- TanStack Query handles fetch errors; route errors rendered by `ui/src/components/RouteError.tsx`

## Logging

**Rust framework:** `tracing` (workspace dep; 11 crates use `use tracing::...`, zero use `log::` directly)

**Patterns:**
- `debug!` for operational detail (e.g. `debug!("exporting to \"{}\"", path.display())`)
- `error!` for recoverable failures (e.g. `error!("failed to parse ndjson line: {e}")`)
- Inline format captures preferred: `{e}` not `{}`, e

**TypeScript:** no `console.log` (ESLint error); only `console.warn`/`console.error` permitted

## Comments

**When to Comment:**
- Explain "why", not "what" — e.g. field-level invariants: `/// \`None\` once [\`shutdown\`](Exporter::shutdown) has flushed and released it.` (`crates/exporter/ndjson/src/lib.rs`)
- Avoid committing commented-out code (`CONTRIBUTING.md`)
- Workspace `Cargo.toml` documents non-obvious member/default-member decisions inline

**Rustdoc:**
- Crate-level `//!` doc comments on every `lib.rs` (e.g. `crates/instrumentation/src/lib.rs`)
- `///` on public items; intra-doc links used (`[\`Self::push\`]`, `[\`ExporterError::Other\`]`)
- `/// # Errors` sections on fallible public functions

**TSDoc:** JSDoc blocks on exported helpers (see `ui/src/test/test-utils.tsx`)

## Function Design

**Rust:**
- Async-first for I/O: `tokio` runtime, `#[async_trait::async_trait]` for async traits (`Exporter` trait in `crates/exporter/types/src/lib.rs`)
- Generic bounds spelled in `where` clauses when non-trivial
- Options structs passed to constructors instead of long parameter lists (`NdjsonExporterOptions { dir }`)

**TypeScript:**
- Function components (no classes); hooks for shared logic
- Options objects with defaults: `function renderWithRouter(options: RenderWithRouterOptions = {})`

## Module Design

**Rust exports:**
- Small `lib.rs` files that declare private modules and re-export the public surface: `mod context; mod observer; pub use context::Context;` (`crates/instrumentation/src/lib.rs`)
- Feature-gated derives: `#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]`
- Zero-cost guarantee: crates activating `quent-time/__test-clock-override` are excluded from `default-members` in root `Cargo.toml`

**TypeScript exports:**
- Named exports preferred; test utils barrel re-exports (`export * from '@testing-library/react'` in `ui/src/test/test-utils.tsx`)
- Generated files never edited by hand: `ui/src/routeTree.gen.ts` (TanStack Router, `tsr generate`)

## PR / Commit Conventions

- PR titles must follow Conventional Commits (`feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert`, optional scope) — CI-enforced (`.github/workflows/pr-title.yml`)
- Every commit needs a DCO sign-off line (`git commit -s`)
- One concern per PR; new components must include tests (`CONTRIBUTING.md`)

---

*Convention analysis: 2026-07-08*
