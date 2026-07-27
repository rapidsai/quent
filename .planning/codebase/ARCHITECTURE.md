<!-- refreshed: 2026-07-08 -->
# Architecture

**Analysis Date:** 2026-07-08

## System Overview

```text
┌─────────────────────────────────────────────────────────────────────┐
│                     Instrumented Application                         │
│  `examples/simulator/application`, `examples/readme`,               │
│  C++/Python apps via generated bridges (`crates/codegen`)            │
└────────────────────────────┬────────────────────────────────────────┘
                             │ typed events (generated API)
                             ▼
┌─────────────────────────────────────────────────────────────────────┐
│              Model Definition + Generated Instrumentation            │
│  `crates/model`, `crates/model-macros`, `crates/stdlib`,             │
│  `crates/instrumentation` (Context/Observer/EventSender)             │
└────────────────────────────┬────────────────────────────────────────┘
                             │ Event<T> over tokio mpsc
                             ▼
┌──────────────────┬──────────────────────┬───────────────────────────┐
│ Filesystem       │ Collector (gRPC)     │ Callback                  │
│ exporters        │ `crates/exporter/    │ `crates/exporter/         │
│ ndjson/msgpack/  │  collector`,         │  callback`                │
│ postcard         │ `crates/collector/*` │                           │
│ `crates/exporter`│  (server on :7836)   │                           │
└────────┬─────────┴──────────┬───────────┴───────────────────────────┘
         │  per-entity event files + `model.qmi` sidecar
         ▼
┌─────────────────────────────────────────────────────────────────────┐
│                     Analysis / Reconstruction                        │
│  `crates/analyzer` (domain-agnostic), `domains/query_engine/analyzer`│
│  (Engine/Query/Operator model rebuild), caches in                    │
│  `domains/query_engine/server`                                       │
└────────────────────────────┬────────────────────────────────────────┘
                             │ HTTP JSON `/api/engines/...` (:8080)
                             ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    Web UI (React 19, Vite)                           │
│  `ui/src` + `ui/packages/@quent/{client,components,hooks,utils}`     │
│  TS types generated from Rust via ts-rs (`crates/ui`,                │
│  `domains/query_engine/ui`)                                          │
└─────────────────────────────────────────────────────────────────────┘
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

**Overall:** Model-driven telemetry pipeline — a layered Cargo workspace of
composable library crates ("building blocks") plus a domain layer and example
applications that wire them together.

**Key Characteristics:**
- Application-agnostic crates live in `crates/`; domain-specific crates in `domains/query_engine/`; runnable wiring in `examples/`.
- Proc macros generate a statically typed instrumentation API and model metadata from declarative definitions (`crates/model-macros`).
- Instrumented code interacts only with the *generated* API; `quent-instrumentation` is an internal backing crate (its docs say so explicitly).
- Transport is pluggable via feature flags on the `quent-exporter` umbrella crate (ndjson, msgpack, postcard, collector, callback).
- Rust↔TypeScript type sharing via `ts-rs` (`crates/ui`, `domains/query_engine/ui` export TS bindings consumed by `ui/`).

## Layers

**Model definition (`crates/model`, `crates/model-macros`, `crates/attributes`, `crates/stdlib`, `crates/time`):**
- Purpose: declare FSMs, Resources, Entities, Usages; generate instrumentation APIs and metadata
- Contains: core types + proc macros; `crates/stdlib` provides reusable resources (`channel`, `memory`, `processor`)
- Depends on: nothing above it
- Used by: instrumentation, analyzer, domain models

**Instrumentation runtime (`crates/instrumentation`, `crates/events`, `crates/build-info`):**
- Purpose: host observers on a tokio runtime, forward events to exporters, write the `model.qmi` provenance sidecar
- Key types: `Context` (`crates/instrumentation/src/context.rs`), `Observer`/`EventSender` (`crates/instrumentation/src/observer.rs`), `Event<T>` (`crates/events/src/lib.rs`)
- Depends on: exporter traits
- Used by: generated instrumentation code only

**Export/transport (`crates/exporter/*`, `crates/collector/*`):**
- Purpose: move serialized events to files, a gRPC collector, or a callback
- Proto contract: `proto/quent/collector/v1/collector.proto`, compiled in `crates/collector/proto/build.rs`
- Filesystem layout: `<root>/<context-uuid>/<EntityName>/<uuid>.<ext>` batch files plus `<context-uuid>/model.qmi`

**Analysis (`crates/analyzer`, `domains/query_engine/analyzer`):**
- Purpose: reconstruct in-memory models from event streams; derive FSM state durations, resource utilization, binned timelines
- Contains: `entity/`, `fsm/`, `resource/`, `timeline/binned/`, `trace/` modules in `crates/analyzer/src/`
- Used by: servers and the `quent-ui` view-type layer

**Serving (`domains/query_engine/server`, `examples/simulator/server`):**
- Purpose: expose collector (tonic gRPC) and analyzer (axum HTTP) services
- `analyzer_service_router()` / `collector_service()` in `domains/query_engine/server/src/lib.rs` are the composition helpers; caching in `analyzer_cache.rs` and `timeline_cache.rs`
- Optional features: `ui` (embed built frontend via `build.rs` + `pnpm build`), `swagger` (utoipa Swagger UI)

**Frontend (`ui/`):**
- Purpose: interactive visualization (plan DAGs via `@xyflow/react`+`elkjs`, timelines via echarts, tables via TanStack Table)
- State: jotai atoms (`ui/src/atoms/`), server state via TanStack Query (`ui/src/lib/queryClient.ts`)
- API access: `ui/packages/@quent/client/src/` (typed fetchers per endpoint)

## Data Flow

### Telemetry Emission Path

1. Application calls generated instrumentation API (e.g. `examples/simulator/application/src/main.rs` uses `quent-simulator-instrumentation`)
2. Generated context wraps `quent_instrumentation::Context` — resolves/spawns a tokio runtime (`crates/instrumentation/src/context.rs`)
3. `EventSender::emit()` stamps `Event::new_now(id, data)` and pushes onto an unbounded mpsc channel (`crates/instrumentation/src/observer.rs`)
4. Per-entity forwarder task drains the channel into an `Exporter` (`crates/exporter/types/`); drop of the observer drains and flushes
5. Exporter writes per-entity batch files (`crates/exporter/{ndjson,msgpack,postcard}`) or streams to the collector (`crates/exporter/collector` → `crates/collector/client`)
6. `write_sidecar()` records model provenance as `model.qmi` (`crates/instrumentation/src/sidecar.rs`)

### Collection Path (remote apps)

1. gRPC `CollectorService` receives event streams on `:7836` (`crates/collector/server/src/server.rs`)
2. A per-source context is created via the `make` factory (`collector_service()` in `domains/query_engine/server/src/lib.rs`)
3. Events are re-exported server-side through a `CollectorSink` (re-exported from `crates/collector/server/src/lib.rs`) into filesystem storage

### Analysis/Query Path

1. `axum::serve` hosts the analyzer HTTP API on `:8080` (`examples/simulator/server/src/main.rs`)
2. Routes under `/api/engines` (`domains/query_engine/server/src/ui.rs`): list engines, `{engine_id}`, `query-groups`, `queries`, `query/{query_id}`, `timeline/single`, `timeline/bulk`
3. `AnalyzerCache` lazily imports a context's event streams (`Simulator::import_events`) and rebuilds the domain model (`domains/query_engine/server/src/analyzer_cache.rs`, `domains/query_engine/analyzer/src/lib.rs`); `TimelineCache` caches binned timelines
4. Responses use ts-rs-typed view structs (`crates/ui/src/lib.rs`, `domains/query_engine/ui/src/lib.rs`)
5. UI fetches via `ui/packages/@quent/client/src/*.ts` with TanStack Query

**State Management:**
- Backend: per-process in-memory caches (`ServiceState` in `domains/query_engine/server/src/state.rs`); event data on the filesystem
- Frontend: jotai for client state, TanStack Query for server state

## Key Abstractions

**Model / FSM / Resource / Usage:**
- Purpose: declarative application performance model
- Examples: `domains/query_engine/model/src/{engine,query,operator,plan,port,worker,query_group}.rs`, extensive walkthrough in `examples/readme/src/lib.rs`
- Pattern: macro invocations (`model! { name: QueryEngine, root: engine::Engine, entities: {...} }`)

**Exporter / ExporterProvider / Importer:**
- Purpose: pluggable transport + reingest
- Examples: `crates/exporter/types/`, umbrella enum in `crates/exporter/src/lib.rs`
- Pattern: trait objects created from `ResolvedExporterOptions`, feature-gated variants

**Observer / EventSender / Context:**
- Purpose: zero/low-overhead event pipeline; noop sender drops events entirely (`EventSender::noop()` in `crates/instrumentation/src/observer.rs`)
- Pattern: sync producer, async forwarder per entity type

**Analyzer `Model` / `Entity` / `Span` traits:**
- Purpose: uniform querying of reconstructed models (resource trees, spans, entity refs)
- Examples: `crates/analyzer/src/lib.rs`; domain impl in `domains/query_engine/analyzer/src/model.rs`

**UiAnalyzer:**
- Purpose: contract between a domain analyzer and the generic HTTP server layer
- Examples: `domains/query_engine/analyzer/src/ui.rs`, implemented by `examples/simulator/analyzer/src/lib.rs` (`SimulatorUiAnalyzer`)

## Entry Points

**`quent-simulator-server`:**
- Location: `examples/simulator/server/src/main.rs`
- Triggers: `cargo run -p quent-simulator-server` or `docker compose up`
- Responsibilities: run collector gRPC (`:7836`, env `QUENT_COLLECTOR_ADDRESS`) and analyzer HTTP (`:8080`, env `QUENT_ANALYZER_ADDRESS`) via `tokio::try_join!`

**`quent-simulator` (application):**
- Location: `examples/simulator/application/src/main.rs`
- Responsibilities: emit simulated query engine telemetry to the collector

**`quent-open`:**
- Location: `crates/open/src/main.rs`
- Responsibilities: open local artifact directories in a generated, commit-pinned viewer (reads `model.qmi`, builds and serves a viewer crate)

**Frontend:**
- Location: `ui/src/main.tsx` (Vite dev server `pnpm dev`, `:5173`)
- Routing: TanStack Router file-based routes in `ui/src/routes/`

**Other binaries:** `examples/readme/src/main.rs`, `domains/query_engine/tests/fixed/src/main.rs` (deterministic emitter, opt-in), `crates/instrumentation-build/example/src/main.rs`

## Architectural Constraints

- **Threading:** instrumentation is sync-facing over an async tokio backend. `Context` borrows an ambient runtime or spawns its own; blocking sync/async crossings **panic on a current-thread runtime** (`crates/instrumentation/src/context.rs`).
- **Zero-cost guarantee:** workspace `default-members` (root `Cargo.toml`) exclude any crate activating `quent-time/__test-clock-override` so default builds keep `quent-time` zero-cost; opt in with `cargo build -p <crate>`.
- **Feature-gated compilation:** `crates/exporter` requires at least one exporter feature (`compile_error!` in `crates/exporter/src/lib.rs`); server `ui`/`swagger` features change what is built and served.
- **Preliminary crates:** `crates/{constraints,schema,ref-target,ref-tree,fsm,resource,instrumentation-build}` are unused preliminary work for issue rapidsai/quent#191 — do not build new code on them without checking the issue.
- **Pre-alpha, breaking changes expected:** no releases; APIs change freely (root `README.md`).

## Anti-Patterns

### Importing `quent-instrumentation` directly from application code

**What happens:** app code uses `Context`/`Observer` from `crates/instrumentation` directly.
**Why it's wrong:** the crate is an internal backing library; its docs (`crates/instrumentation/src/lib.rs`) state instrumented code should use only the generated instrumentation API.
**Do this instead:** define/extend the model (e.g. `examples/simulator/instrumentation/src/lib.rs`) and call the macro-generated context/observers.

### Depending on `quent-collector-client` for the `CollectorSink` bound

**What happens:** server-side code adds a direct dependency to name `CollectorSink`.
**Why it's wrong:** the bound is deliberately re-exported to avoid the extra dependency edge.
**Do this instead:** use `quent_collector::CollectorSink` re-exported from `crates/collector/server/src/lib.rs`.

## Error Handling

**Strategy:** `thiserror`-based error enums per layer with `Result` aliases.

**Patterns:**
- `AnalyzerError` + `AnalyzerResult<T>` (`crates/analyzer/src/error.rs`, `crates/analyzer/src/lib.rs`)
- `ExporterError`/`ImporterError` + result aliases (`crates/exporter/types/`)
- Server route errors in `domains/query_engine/server/src/error.rs`
- Binaries return `Result<(), Box<dyn std::error::Error>>` (`examples/simulator/server/src/main.rs`)
- Event send failures log once then suppress (`EventSender::send` in `crates/instrumentation/src/observer.rs`)

## Cross-Cutting Concerns

**Logging:** `tracing` + `tracing-subscriber`; `initialize_tracing()` in `domains/query_engine/server/src/lib.rs` (env filter, `h2`/`tonic` muted, stderr writer, span-close events).
**Validation:** model-structure validation happens at macro expansion (compile time) and at analysis time (FSM transition sequence checks surface as `AnalyzerError`).
**Time:** all timestamps flow through `crates/time` (`TimeUnixNanoSec`, `SpanUnixNanoSec`); test clock override is feature-gated.
**Type sharing:** `ts-rs` derives on Rust view types generate TS bindings (e.g. `examples/simulator/server/ts-bindings`, `crates/schema/ts`).
**Authentication:** none — services are unauthenticated (development/experimental stage).

---

*Architecture analysis: 2026-07-08*
