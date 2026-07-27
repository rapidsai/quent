# Technology Stack

**Analysis Date:** 2026-07-08

## Languages

**Primary:**
- Rust (edition 2024, toolchain >= 1.93 per `README.md`; pixi pins `rust >= 1.96`, Docker builds with `rust:1.91-trixie`) - All backend crates under `crates/`, `domains/`, `examples/`, `proto/`
- TypeScript ~5.9 - Web UI under `ui/` (React app + workspace packages in `ui/packages/@quent/`)

**Secondary:**
- C++ - Generated instrumentation API consumers: `examples/cpp-integration/`, `domains/query_engine/tests/cpp/` (built via `cxx`/`cxx-build` and CMake)
- Python >= 3.11 - PyO3 extension module consumers: `examples/python-integration/` (maturin, `pyproject.toml`), `domains/query_engine/tests/python/`
- Protocol Buffers (proto3) - `proto/quent/collector/v1/collector.proto`

## Runtime

**Environment:**
- Rust binaries (Tokio multi-threaded async runtime for servers/exporters)
- Node.js >= 24.11.0 (pinned via `ui/package.json` `engines` and `volta`; pixi provides `nodejs >= 24.11.0`)
- Browser (React 19 SPA built with Vite)
- Python >= 3.11, < 3.15 for the PyO3 bridge (`abi3-py311`)

**Package Manager:**
- Cargo (workspace resolver "3", ~60 member crates in root `Cargo.toml`)
  - Lockfile: present (`Cargo.lock`)
- pnpm 11.6.0 (`packageManager` field; `preinstall` enforces pnpm via `only-allow`)
  - Lockfile: present (`ui/pnpm-lock.yaml`); pnpm workspace via `ui/pnpm-workspace.yaml`
- pixi (conda-forge) for toolchain management: `pixi.toml` / `pixi.lock` provision rust, nodejs, pnpm, libprotobuf, cmake, cxx-compiler, python, maturin, pytest, ty, uv

## Frameworks

**Core (Rust):**
- Tokio 1.48 - async runtime for all servers and the collector client
- Tonic 0.14.2 + Prost 0.14.1 - gRPC collector service (`crates/collector/`, `proto/`)
- Axum 0.8.7 - HTTP analysis API servers (`domains/query_engine/server/`, `examples/simulator/server/`)
- tower-http 0.7 (`cors` feature) - CORS layer for the HTTP API
- Clap 4.5 (`derive`, `env`) - CLI argument parsing for binaries
- PyO3 0.29 - Python extension module bridges
- cxx 1 - C++ bridge (`examples/cpp-integration/bridge/`)

**Core (UI):**
- React 19 + react-dom 19 - `ui/src/main.tsx`
- TanStack Router 1.x (file-based routes generated via `tsr generate`; config `ui/tsr.config.json`) - `ui/src/routes/`
- TanStack Query 5 - server state (`ui/src/lib/queryClient.ts`)
- Jotai 2 (+ jotai-family) - client state atoms (`ui/src/atoms/`)
- Tailwind CSS 4 (`@tailwindcss/vite`) + Radix UI primitives + class-variance-authority/clsx/tailwind-merge - styling and components (shadcn-style, `ui/components.json`)
- ECharts 5 (custom tree-shaken build via `@/lib/echarts.ts`) - charts
- @xyflow/react 12 + elkjs 0.11 (bundled `elk.bundled.js` alias) - DAG visualization/layout
- @tanstack/react-table, @tanstack/react-virtual, react-resizable-panels, lucide-react

**Testing:**
- Rust: built-in `cargo test` (workspace-wide, `--all-features --all-targets` in CI)
- UI unit: Vitest 4 (+ @vitest/coverage-v8, jsdom, Testing Library, MSW 2 for API mocking) - `ui/vitest.config.ts`, `ui/vitest.workspace.ts`
- UI e2e: Playwright 1.60 - `ui/playwright.config.ts`, `ui/e2e/`
- Python: pytest >= 8 (`domains/query_engine/tests/python/test_query_engine.py`); `ty` for type checking; `uv` for env management

**Build/Dev:**
- Vite 7 (`ui/vite.config.ts`) with @vitejs/plugin-react, TanStack router plugin, rollup-plugin-visualizer, manual chunk splitting
- tsdown 0.22 - builds for `ui/packages/@quent/*` workspace packages
- ESLint 9 (flat config `ui/eslint.config.js`, typescript-eslint 8) + Prettier 3
- tonic-prost-build 0.14.2 - proto codegen at build time (`crates/collector/proto/`)
- maturin >= 1.14 - Python wheel builds
- cargo-deny (`deny.toml`) - license/dependency auditing
- Docker (`Dockerfile`, multi-stage `rust:1.91-trixie` -> `debian:trixie`) + `docker-compose.yml`

## Key Dependencies

**Critical (Rust):**
- serde 1 / serde_json 1 - event serialization backbone across all crates
- prost + tonic + tonic-prost - collector gRPC transport
- postcard 1, rmp-serde 1 (msgpack), ciborium 0.2 (CBOR framing in `crates/collector/client/`) - binary event encodings for exporters (`crates/exporter/{postcard,msgpack,ndjson}/`)
- uuid 1 (v7) - event/context identifiers
- petgraph 0.8 - graph modeling (analyzer/model)
- ts-rs 12 - generates TypeScript bindings consumed by the UI (aliased as `~quent/types` -> `examples/simulator/server/ts-bindings` in `ui/vite.config.ts`)
- moka 0.12 (`future`) - in-process async caching in `domains/query_engine/server/`
- thiserror 2, tracing 0.1 / tracing-subscriber 0.3, log 0.4 (kv)
- smallvec, indexmap, rustc-hash - performance-oriented data structures

**Critical (quent-open, `crates/open/`):**
- reqwest 0.12 (`rustls-tls-native-roots`, `json`; optional, behind `db` feature) - fetch runs from internal Benchmarking API
- tar/flate2/zstd/zip/tempfile (optional, behind `archive` feature) - open archived telemetry
- dotenvy (optional) - loads `.env` for `QUENT_OPEN_*` vars
- backon 1 - retry; gix-url, cargo-manifest, syn/quote/prettyplease - viewer-crate generation

**Infrastructure:**
- rust-embed 8 + mime_guess 2 (optional, `ui` feature) - embed the built webpage into the server binary
- utoipa 5 + utoipa-swagger-ui 9 (optional, `swagger` feature) - OpenAPI docs at `/swagger-ui`
- pyo3 (workspace, `abi3-py311`) - Python bridges

## Configuration

**Environment:**
- CLI-first via Clap with env fallbacks (see INTEGRATIONS.md for var list); no committed `.env` files (quent-open optionally loads one at runtime via dotenvy, `crates/open/src/main.rs`)
- Vite dev/preview proxy target: `VITE_API_TARGET` (default `http://localhost:8080`), `ui/vite.config.ts`

**Build:**
- Root `Cargo.toml` - workspace members, `default-members` (excludes crates enabling `quent-time/__test-clock-override` to preserve zero-cost default builds), shared `[workspace.dependencies]`
- `pixi.toml` / `pixi.lock` - toolchain provisioning
- `ui/tsconfig.json` + `tsconfig.base.json` + `tsconfig.node.json`; path alias `@` -> `ui/src`
- `ui/vite.config.ts`, `ui/vitest.config.ts`, `ui/playwright.config.ts`, `ui/eslint.config.js`, `ui/tsr.config.json`
- `deny.toml` - cargo-deny policy
- Cargo feature flags on servers: `ui` (embed static webpage, runs `pnpm install && pnpm build`), `swagger`

## Platform Requirements

**Development:**
- Rust stable >= 1.93, Node >= 24.11, pnpm >= 10, protoc (or just `pixi shell` which provides everything)
- pixi platforms: linux-64, linux-aarch64, osx-arm64, osx-64
- CMake >= 3.24 + C++ compiler for the C++ integration examples/tests

**Production:**
- Self-hosted binaries or Docker; `docker-compose.yml` runs `quent-simulator-server` (HTTP `:8080`, gRPC collector `:7836`) and the `quent-simulator` event generator
- No cloud-provider dependency detected

---

*Stack analysis: 2026-07-08*
