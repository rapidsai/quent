<!-- GSD:project-start source:PROJECT.md -->
## Project

**Quent NVTX Consumer**

An NVTX ingestion pipeline for Quent: a Rust injection library that captures NVTX events (ranges, marks, domains, registered strings, resources, and payload-extension data) from instrumented applications and turns them into Quent events, plus the Quent model, analyzer, API endpoint, and UI rendering needed to see those ranges as traces. It exists so Quent can observe GPU-accelerated data processing libraries (libcudf, cuCascade, and similar) that already emit NVTX, and it includes a fan-out mediator so Quent can coexist with other NVTX consumers like Nsight Systems and AON in the same process.

**Core Value:** An application emitting NVTX ranges can be observed by Quent end-to-end — events captured, reconstructed into a model, and visible in the Quent UI — without breaking that application's ability to also be profiled by NSys/AON.

### Constraints

- **Platform**: Linux (and possibly macOS) 64-bit only — NVTX injection relies on weak-symbol override / `NVTX_INJECTION64_PATH`, which excludes Windows
- **Language**: Ingestion library in Rust — explicit preference from Johan ("do not want to write anything new in C/C++ if I can help it"); only minimal C shims where the linker mechanism demands it
- **Architecture**: Follow the established repo layering — application-agnostic capture crates, domain model/analyzer/server/ui split, `quent-*` naming, workspace `members`+`default-members` registration
- **Separability**: The injection crate must stay cleanly separable from Quent so it can be offered upstream to NVIDIA/NVTX later
- **NVTX semantics**: One injection slot per process is an NVTX invariant we must design around (fan-out), not something we can change
- **Compatibility**: External NVTX consumers (nsys via `NVTX_INJECTION64_PATH`) must keep working unmodified when the fan-out mediator is in place
- **CI**: End-to-end validation must be runnable without GPU hardware (deterministic in-repo NVTX test app); GPU-library validation is manual
<!-- GSD:project-end -->

<!-- GSD:stack-start source:codebase/STACK.md -->
## Technology Stack

## Languages
- Rust (edition 2024, toolchain >= 1.93 per `README.md`; pixi pins `rust >= 1.96`, Docker builds with `rust:1.91-trixie`) - All backend crates under `crates/`, `domains/`, `examples/`, `proto/`
- TypeScript ~5.9 - Web UI under `ui/` (React app + workspace packages in `ui/packages/@quent/`)
- C++ - Generated instrumentation API consumers: `examples/cpp-integration/`, `domains/query_engine/tests/cpp/` (built via `cxx`/`cxx-build` and CMake)
- Python >= 3.11 - PyO3 extension module consumers: `examples/python-integration/` (maturin, `pyproject.toml`), `domains/query_engine/tests/python/`
- Protocol Buffers (proto3) - `proto/quent/collector/v1/collector.proto`
## Runtime
- Rust binaries (Tokio multi-threaded async runtime for servers/exporters)
- Node.js >= 24.11.0 (pinned via `ui/package.json` `engines` and `volta`; pixi provides `nodejs >= 24.11.0`)
- Browser (React 19 SPA built with Vite)
- Python >= 3.11, < 3.15 for the PyO3 bridge (`abi3-py311`)
- Cargo (workspace resolver "3", ~60 member crates in root `Cargo.toml`)
- pnpm 11.6.0 (`packageManager` field; `preinstall` enforces pnpm via `only-allow`)
- pixi (conda-forge) for toolchain management: `pixi.toml` / `pixi.lock` provision rust, nodejs, pnpm, libprotobuf, cmake, cxx-compiler, python, maturin, pytest, ty, uv
## Frameworks
- Tokio 1.48 - async runtime for all servers and the collector client
- Tonic 0.14.2 + Prost 0.14.1 - gRPC collector service (`crates/collector/`, `proto/`)
- Axum 0.8.7 - HTTP analysis API servers (`domains/query_engine/server/`, `examples/simulator/server/`)
- tower-http 0.7 (`cors` feature) - CORS layer for the HTTP API
- Clap 4.5 (`derive`, `env`) - CLI argument parsing for binaries
- PyO3 0.29 - Python extension module bridges
- cxx 1 - C++ bridge (`examples/cpp-integration/bridge/`)
- React 19 + react-dom 19 - `ui/src/main.tsx`
- TanStack Router 1.x (file-based routes generated via `tsr generate`; config `ui/tsr.config.json`) - `ui/src/routes/`
- TanStack Query 5 - server state (`ui/src/lib/queryClient.ts`)
- Jotai 2 (+ jotai-family) - client state atoms (`ui/src/atoms/`)
- Tailwind CSS 4 (`@tailwindcss/vite`) + Radix UI primitives + class-variance-authority/clsx/tailwind-merge - styling and components (shadcn-style, `ui/components.json`)
- ECharts 5 (custom tree-shaken build via `@/lib/echarts.ts`) - charts
- @xyflow/react 12 + elkjs 0.11 (bundled `elk.bundled.js` alias) - DAG visualization/layout
- @tanstack/react-table, @tanstack/react-virtual, react-resizable-panels, lucide-react
- Rust: built-in `cargo test` (workspace-wide, `--all-features --all-targets` in CI)
- UI unit: Vitest 4 (+ @vitest/coverage-v8, jsdom, Testing Library, MSW 2 for API mocking) - `ui/vitest.config.ts`, `ui/vitest.workspace.ts`
- UI e2e: Playwright 1.60 - `ui/playwright.config.ts`, `ui/e2e/`
- Python: pytest >= 8 (`domains/query_engine/tests/python/test_query_engine.py`); `ty` for type checking; `uv` for env management
- Vite 7 (`ui/vite.config.ts`) with @vitejs/plugin-react, TanStack router plugin, rollup-plugin-visualizer, manual chunk splitting
- tsdown 0.22 - builds for `ui/packages/@quent/*` workspace packages
- ESLint 9 (flat config `ui/eslint.config.js`, typescript-eslint 8) + Prettier 3
- tonic-prost-build 0.14.2 - proto codegen at build time (`crates/collector/proto/`)
- maturin >= 1.14 - Python wheel builds
- cargo-deny (`deny.toml`) - license/dependency auditing
- Docker (`Dockerfile`, multi-stage `rust:1.91-trixie` -> `debian:trixie`) + `docker-compose.yml`
## Key Dependencies
- serde 1 / serde_json 1 - event serialization backbone across all crates
- prost + tonic + tonic-prost - collector gRPC transport
- postcard 1, rmp-serde 1 (msgpack), ciborium 0.2 (CBOR framing in `crates/collector/client/`) - binary event encodings for exporters (`crates/exporter/{postcard,msgpack,ndjson}/`)
- uuid 1 (v7) - event/context identifiers
- petgraph 0.8 - graph modeling (analyzer/model)
- ts-rs 12 - generates TypeScript bindings consumed by the UI (aliased as `~quent/types` -> `examples/simulator/server/ts-bindings` in `ui/vite.config.ts`)
- moka 0.12 (`future`) - in-process async caching in `domains/query_engine/server/`
- thiserror 2, tracing 0.1 / tracing-subscriber 0.3, log 0.4 (kv)
- smallvec, indexmap, rustc-hash - performance-oriented data structures
- reqwest 0.12 (`rustls-tls-native-roots`, `json`; optional, behind `db` feature) - fetch runs from internal Benchmarking API
- tar/flate2/zstd/zip/tempfile (optional, behind `archive` feature) - open archived telemetry
- dotenvy (optional) - loads `.env` for `QUENT_OPEN_*` vars
- backon 1 - retry; gix-url, cargo-manifest, syn/quote/prettyplease - viewer-crate generation
- rust-embed 8 + mime_guess 2 (optional, `ui` feature) - embed the built webpage into the server binary
- utoipa 5 + utoipa-swagger-ui 9 (optional, `swagger` feature) - OpenAPI docs at `/swagger-ui`
- pyo3 (workspace, `abi3-py311`) - Python bridges
## Configuration
- CLI-first via Clap with env fallbacks (see INTEGRATIONS.md for var list); no committed `.env` files (quent-open optionally loads one at runtime via dotenvy, `crates/open/src/main.rs`)
- Vite dev/preview proxy target: `VITE_API_TARGET` (default `http://localhost:8080`), `ui/vite.config.ts`
- Root `Cargo.toml` - workspace members, `default-members` (excludes crates enabling `quent-time/__test-clock-override` to preserve zero-cost default builds), shared `[workspace.dependencies]`
- `pixi.toml` / `pixi.lock` - toolchain provisioning
- `ui/tsconfig.json` + `tsconfig.base.json` + `tsconfig.node.json`; path alias `@` -> `ui/src`
- `ui/vite.config.ts`, `ui/vitest.config.ts`, `ui/playwright.config.ts`, `ui/eslint.config.js`, `ui/tsr.config.json`
- `deny.toml` - cargo-deny policy
- Cargo feature flags on servers: `ui` (embed static webpage, runs `pnpm install && pnpm build`), `swagger`
## Platform Requirements
- Rust stable >= 1.93, Node >= 24.11, pnpm >= 10, protoc (or just `pixi shell` which provides everything)
- pixi platforms: linux-64, linux-aarch64, osx-arm64, osx-64
- CMake >= 3.24 + C++ compiler for the C++ integration examples/tests
- Self-hosted binaries or Docker; `docker-compose.yml` runs `quent-simulator-server` (HTTP `:8080`, gRPC collector `:7836`) and the `quent-simulator` event generator
- No cloud-provider dependency detected
<!-- GSD:stack-end -->

<!-- GSD:conventions-start source:CONVENTIONS.md -->
## Conventions

## Naming Patterns
- Rust: `snake_case.rs` modules (e.g. `crates/instrumentation/src/observer.rs`, `crates/analyzer/src/resource/tree.rs`)
- TypeScript React components: `PascalCase.tsx` (e.g. `ui/src/components/QueryResourceTree.tsx`, `ui/src/components/ThemeToggle.tsx`)
- TypeScript utilities: `camelCase.ts` (e.g. `ui/packages/@quent/utils/src/parseJsonWithBigInt.ts`, `ui/src/atoms/resourceTree.ts`)
- React hooks: `useXxx.ts` (e.g. `ui/src/hooks/useExpandedIds.ts`, `ui/src/hooks/useQueryPlanVisualization.ts`)
- UI package directories: `kebab-case` (e.g. `ui/packages/@quent/components/src/pivot-table/`, `.../operator-timeline/`)
- Python tests: `test_*.py` (e.g. `domains/query_engine/tests/python/test_query_engine.py`)
- All library crates prefixed `quent-` with kebab-case names (`quent-instrumentation`, `quent-exporter`, `quent-model-macros`); import paths use snake_case (`quent_events`, `quent_exporter_types`)
- Application-agnostic crates live in `crates/`; domain-specific crates in `domains/query_engine/`; examples in `examples/`
- Rust: `snake_case`; fallible constructors named `try_new` (see `crates/exporter/ndjson/src/lib.rs`, `crates/instrumentation/src/context.rs`)
- TypeScript: `camelCase` (e.g. `formatDuration`, `renderWithRouter`); helper factories prefixed `create` (e.g. `createTestQueryClient` in `ui/src/test/test-utils.tsx`)
- Rust: `PascalCase` structs/enums/traits; options structs suffixed `Options` (`NdjsonExporterOptions`, `FileSystemExporterOptions`); error enums suffixed `Error`; result aliases suffixed `Result` (`ExporterResult`, `ImporterResult` in `crates/exporter/types/src/lib.rs`)
- Rust constants: `SCREAMING_SNAKE_CASE` (e.g. `const EXTENSION: &str = "ndjson"`)
- TypeScript: `PascalCase` interfaces/types (e.g. `RenderWithRouterOptions`, `QuantitySpec`)
## License Headers (mandatory)
## Code Style
- `rustfmt` with default settings — no `rustfmt.toml` present anywhere in the repo
- Edition 2024, workspace resolver 3 (root `Cargo.toml`)
- Run: `pixi run cargo fmt --all` (CI checks with `-- --check`)
- `cargo clippy` with `-D warnings` — zero warnings allowed (CI: `.github/workflows/rust.yml`)
- Run: `pixi run cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
- `cargo deny check` for license/security audit (config: `deny.toml`)
- `semi: true`, `singleQuote: true`, `printWidth: 100`, `tabWidth: 2`, `trailingComma: "es5"`, `arrowParens: "avoid"`, `endOfLine: "lf"`
- Run: `pnpm format` / `pnpm format:check` from `ui/`
- `@eslint/js` recommended + `typescript-eslint` recommended + `eslint-plugin-react-hooks` + `eslint-plugin-react-refresh` + prettier plugin
- `no-console: error` — only `console.warn` and `console.error` allowed
- `@typescript-eslint/no-unused-vars: error` — prefix intentionally unused args/vars with `_`
- Ignores: `dist`, `src/routeTree.gen.ts` (generated), `examples/**` (self-contained apps with own toolchains)
- `rumdl` lint runs in CI (`.github/workflows/markdown.yml`)
## Import Organization
- External packages first, then internal aliases/relative imports
- Path alias: `@/` → `ui/src/` (configured in `ui/vitest.config.ts` and `ui/tsconfig.json`)
- Workspace packages imported as `@quent/client`, `@quent/components`, `@quent/hooks`, `@quent/utils` (pnpm workspace, `ui/pnpm-workspace.yaml`)
## Error Handling
- `thiserror`-derived error enums per crate (workspace dep `thiserror = "2.0.17"`)
- Canonical pattern in `crates/exporter/types/src/lib.rs`: enum with specific variants plus `#[error(transparent)] Other(#[from] Box<dyn std::error::Error + Send + Sync>)` and an `other()` helper for wrapping
- Per-crate `Result` type aliases: `pub type ExporterResult<T> = std::result::Result<T, ExporterError>;`
- Fallible constructors return `Result` and are named `try_new`
- `let Some(x) = ... else { return Ok(()) };` let-else for early returns
- Document error cases with `/// # Errors` doc sections
- In iterators/streams where errors cannot propagate, log with `tracing::error!` and return `None` (see `NdjsonImporter::next` in `crates/exporter/ndjson/src/lib.rs`)
- TanStack Query handles fetch errors; route errors rendered by `ui/src/components/RouteError.tsx`
## Logging
- `debug!` for operational detail (e.g. `debug!("exporting to \"{}\"", path.display())`)
- `error!` for recoverable failures (e.g. `error!("failed to parse ndjson line: {e}")`)
- Inline format captures preferred: `{e}` not `{}`, e
## Comments
- Explain "why", not "what" — e.g. field-level invariants: `/// \`None\` once [\`shutdown\`](Exporter::shutdown) has flushed and released it.` (`crates/exporter/ndjson/src/lib.rs`)
- Avoid committing commented-out code (`CONTRIBUTING.md`)
- Workspace `Cargo.toml` documents non-obvious member/default-member decisions inline
- Crate-level `//!` doc comments on every `lib.rs` (e.g. `crates/instrumentation/src/lib.rs`)
- `///` on public items; intra-doc links used (`[\`Self::push\`]`, `[\`ExporterError::Other\`]`)
- `/// # Errors` sections on fallible public functions
## Function Design
- Async-first for I/O: `tokio` runtime, `#[async_trait::async_trait]` for async traits (`Exporter` trait in `crates/exporter/types/src/lib.rs`)
- Generic bounds spelled in `where` clauses when non-trivial
- Options structs passed to constructors instead of long parameter lists (`NdjsonExporterOptions { dir }`)
- Function components (no classes); hooks for shared logic
- Options objects with defaults: `function renderWithRouter(options: RenderWithRouterOptions = {})`
## Module Design
- Small `lib.rs` files that declare private modules and re-export the public surface: `mod context; mod observer; pub use context::Context;` (`crates/instrumentation/src/lib.rs`)
- Feature-gated derives: `#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]`
- Zero-cost guarantee: crates activating `quent-time/__test-clock-override` are excluded from `default-members` in root `Cargo.toml`
- Named exports preferred; test utils barrel re-exports (`export * from '@testing-library/react'` in `ui/src/test/test-utils.tsx`)
- Generated files never edited by hand: `ui/src/routeTree.gen.ts` (TanStack Router, `tsr generate`)
## PR / Commit Conventions
- PR titles must follow Conventional Commits (`feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert`, optional scope) — CI-enforced (`.github/workflows/pr-title.yml`)
- Every commit needs a DCO sign-off line (`git commit -s`)
- One concern per PR; new components must include tests (`CONTRIBUTING.md`)
<!-- GSD:conventions-end -->

<!-- GSD:architecture-start source:ARCHITECTURE.md -->
## Architecture

## System Overview
```text
```
## Component Responsibilities
| Component | Responsibility | File |
|-----------|----------------|------|
| Model core types | FSM/Resource/Entity/Usage definitions, `Model` metadata | `crates/model/src/model.rs` |
| Proc macros | `resource!`, `entity!`, `state!`, `fsm!`, `model!`, `instrumentation!`, `#[derive(Attributes)]` | `crates/model-macros/src/lib.rs` |
| Instrumentation runtime | Sync→async bridge, per-entity event pipelines, `model.qmi` sidecar | `crates/instrumentation/src/{context.rs,observer.rs,sidecar.rs}` |
| Event types | `Event<T>` envelope, `EntityEvent` trait | `crates/events/src/lib.rs` |
| Exporter umbrella | Feature-gated `ExporterOptions` / `ResolvedExporterOptions`, format detection, importer creation | `crates/exporter/src/lib.rs` |
| Exporter traits | `Exporter`, `ExporterProvider`, `Importer` | `crates/exporter/types/` |
| Format exporters | ndjson / msgpack / postcard filesystem writers | `crates/exporter/{ndjson,msgpack,postcard}/` |
| Collector transport | gRPC exporter client + collector service | `crates/exporter/collector/`, `crates/collector/{proto,client,server}/` |
| Domain-agnostic analyzer | `Entity`/`Model`/`Span` traits, FSM/resource runtimes, binned timelines | `crates/analyzer/src/lib.rs` |
| UI view types (Rust) | ts-rs-exported view/timeline types shared with frontend | `crates/ui/src/lib.rs` |
| Query engine model | `Engine`, `Worker`, `Query`, `QueryGroup`, `Plan`, `Operator`, `Port` entities | `domains/query_engine/model/src/lib.rs` |
| Query engine analyzer | Rebuilds `QueryEngineModel` from events; `UiAnalyzer` trait | `domains/query_engine/analyzer/src/lib.rs`, `domains/query_engine/analyzer/src/ui.rs` |
| Query engine server | axum HTTP API + tonic gRPC collector composition, analyzer/timeline caches | `domains/query_engine/server/src/lib.rs` |
| C++/Python codegen | cxx and PyO3 bridge generation from a model | `crates/codegen/src/{cxx_bridge.rs,pyo3_bridge.rs}` |
| Artifact viewer | `quent-open` binary: build+serve a pinned viewer for local artifacts | `crates/open/src/main.rs` |
| Frontend app | Routes, pages, charts, plan DAG visualization | `ui/src/main.tsx`, `ui/src/routes/` |
## Pattern Overview
- Application-agnostic crates live in `crates/`; domain-specific crates in `domains/query_engine/`; runnable wiring in `examples/`.
- Proc macros generate a statically typed instrumentation API and model metadata from declarative definitions (`crates/model-macros`).
- Instrumented code interacts only with the *generated* API; `quent-instrumentation` is an internal backing crate (its docs say so explicitly).
- Transport is pluggable via feature flags on the `quent-exporter` umbrella crate (ndjson, msgpack, postcard, collector, callback).
- Rust↔TypeScript type sharing via `ts-rs` (`crates/ui`, `domains/query_engine/ui` export TS bindings consumed by `ui/`).
## Layers
- Purpose: declare FSMs, Resources, Entities, Usages; generate instrumentation APIs and metadata
- Contains: core types + proc macros; `crates/stdlib` provides reusable resources (`channel`, `memory`, `processor`)
- Depends on: nothing above it
- Used by: instrumentation, analyzer, domain models
- Purpose: host observers on a tokio runtime, forward events to exporters, write the `model.qmi` provenance sidecar
- Key types: `Context` (`crates/instrumentation/src/context.rs`), `Observer`/`EventSender` (`crates/instrumentation/src/observer.rs`), `Event<T>` (`crates/events/src/lib.rs`)
- Depends on: exporter traits
- Used by: generated instrumentation code only
- Purpose: move serialized events to files, a gRPC collector, or a callback
- Proto contract: `proto/quent/collector/v1/collector.proto`, compiled in `crates/collector/proto/build.rs`
- Filesystem layout: `<root>/<context-uuid>/<EntityName>/<uuid>.<ext>` batch files plus `<context-uuid>/model.qmi`
- Purpose: reconstruct in-memory models from event streams; derive FSM state durations, resource utilization, binned timelines
- Contains: `entity/`, `fsm/`, `resource/`, `timeline/binned/`, `trace/` modules in `crates/analyzer/src/`
- Used by: servers and the `quent-ui` view-type layer
- Purpose: expose collector (tonic gRPC) and analyzer (axum HTTP) services
- `analyzer_service_router()` / `collector_service()` in `domains/query_engine/server/src/lib.rs` are the composition helpers; caching in `analyzer_cache.rs` and `timeline_cache.rs`
- Optional features: `ui` (embed built frontend via `build.rs` + `pnpm build`), `swagger` (utoipa Swagger UI)
- Purpose: interactive visualization (plan DAGs via `@xyflow/react`+`elkjs`, timelines via echarts, tables via TanStack Table)
- State: jotai atoms (`ui/src/atoms/`), server state via TanStack Query (`ui/src/lib/queryClient.ts`)
- API access: `ui/packages/@quent/client/src/` (typed fetchers per endpoint)
## Data Flow
### Telemetry Emission Path
### Collection Path (remote apps)
### Analysis/Query Path
- Backend: per-process in-memory caches (`ServiceState` in `domains/query_engine/server/src/state.rs`); event data on the filesystem
- Frontend: jotai for client state, TanStack Query for server state
## Key Abstractions
- Purpose: declarative application performance model
- Examples: `domains/query_engine/model/src/{engine,query,operator,plan,port,worker,query_group}.rs`, extensive walkthrough in `examples/readme/src/lib.rs`
- Pattern: macro invocations (`model! { name: QueryEngine, root: engine::Engine, entities: {...} }`)
- Purpose: pluggable transport + reingest
- Examples: `crates/exporter/types/`, umbrella enum in `crates/exporter/src/lib.rs`
- Pattern: trait objects created from `ResolvedExporterOptions`, feature-gated variants
- Purpose: zero/low-overhead event pipeline; noop sender drops events entirely (`EventSender::noop()` in `crates/instrumentation/src/observer.rs`)
- Pattern: sync producer, async forwarder per entity type
- Purpose: uniform querying of reconstructed models (resource trees, spans, entity refs)
- Examples: `crates/analyzer/src/lib.rs`; domain impl in `domains/query_engine/analyzer/src/model.rs`
- Purpose: contract between a domain analyzer and the generic HTTP server layer
- Examples: `domains/query_engine/analyzer/src/ui.rs`, implemented by `examples/simulator/analyzer/src/lib.rs` (`SimulatorUiAnalyzer`)
## Entry Points
- Location: `examples/simulator/server/src/main.rs`
- Triggers: `cargo run -p quent-simulator-server` or `docker compose up`
- Responsibilities: run collector gRPC (`:7836`, env `QUENT_COLLECTOR_ADDRESS`) and analyzer HTTP (`:8080`, env `QUENT_ANALYZER_ADDRESS`) via `tokio::try_join!`
- Location: `examples/simulator/application/src/main.rs`
- Responsibilities: emit simulated query engine telemetry to the collector
- Location: `crates/open/src/main.rs`
- Responsibilities: open local artifact directories in a generated, commit-pinned viewer (reads `model.qmi`, builds and serves a viewer crate)
- Location: `ui/src/main.tsx` (Vite dev server `pnpm dev`, `:5173`)
- Routing: TanStack Router file-based routes in `ui/src/routes/`
## Architectural Constraints
- **Threading:** instrumentation is sync-facing over an async tokio backend. `Context` borrows an ambient runtime or spawns its own; blocking sync/async crossings **panic on a current-thread runtime** (`crates/instrumentation/src/context.rs`).
- **Zero-cost guarantee:** workspace `default-members` (root `Cargo.toml`) exclude any crate activating `quent-time/__test-clock-override` so default builds keep `quent-time` zero-cost; opt in with `cargo build -p <crate>`.
- **Feature-gated compilation:** `crates/exporter` requires at least one exporter feature (`compile_error!` in `crates/exporter/src/lib.rs`); server `ui`/`swagger` features change what is built and served.
- **Preliminary crates:** `crates/{constraints,schema,ref-target,ref-tree,fsm,resource,instrumentation-build}` are unused preliminary work for issue rapidsai/quent#191 — do not build new code on them without checking the issue.
- **Pre-alpha, breaking changes expected:** no releases; APIs change freely (root `README.md`).
## Anti-Patterns
### Importing `quent-instrumentation` directly from application code
### Depending on `quent-collector-client` for the `CollectorSink` bound
## Error Handling
- `AnalyzerError` + `AnalyzerResult<T>` (`crates/analyzer/src/error.rs`, `crates/analyzer/src/lib.rs`)
- `ExporterError`/`ImporterError` + result aliases (`crates/exporter/types/`)
- Server route errors in `domains/query_engine/server/src/error.rs`
- Binaries return `Result<(), Box<dyn std::error::Error>>` (`examples/simulator/server/src/main.rs`)
- Event send failures log once then suppress (`EventSender::send` in `crates/instrumentation/src/observer.rs`)
## Cross-Cutting Concerns
<!-- GSD:architecture-end -->

<!-- GSD:skills-start source:skills/ -->
## Project Skills

No project skills found. Add skills to any of: `.claude/skills/`, `.agents/skills/`, `.cursor/skills/`, `.github/skills/`, or `.codex/skills/` with a `SKILL.md` index file.
<!-- GSD:skills-end -->

<!-- GSD:workflow-start source:GSD defaults -->
## GSD Workflow Enforcement

Before using Edit, Write, or other file-changing tools, start work through a GSD command so planning artifacts and execution context stay in sync.

Use these entry points:
- `/gsd-quick` for small fixes, doc updates, and ad-hoc tasks
- `/gsd-debug` for investigation and bug fixing
- `/gsd-execute-phase` for planned phase work

Do not make direct repo edits outside a GSD workflow unless the user explicitly asks to bypass it.
<!-- GSD:workflow-end -->



<!-- GSD:profile-start -->
## Developer Profile

> Profile not yet configured. Run `/gsd-profile-user` to generate your developer profile.
> This section is managed by `generate-claude-profile` -- do not edit manually.
<!-- GSD:profile-end -->
