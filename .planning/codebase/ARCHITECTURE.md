# Architecture

**Analysis Date:** 2026-03-25

## Pattern Overview

**Overall:** Layered domain-driven architecture with a separation between instrumentation/collection (data ingestion), analysis (event processing and model construction), and presentation (UI/API serving).

**Key Characteristics:**
- Event-driven architecture: All system behavior modeled through timestamped events
- Multi-domain separation: Generic analysis primitives (`crates/analyzer`) + domain-specific models (`domains/query_engine`)
- Trait-based abstraction: Extensive use of Rust traits to decouple concrete implementations from interfaces
- Server-UI separation: gRPC collector server + HTTP analyzer/timeline API server + React SPA frontend
- Finite State Machine (FSM) modeling: Entities tracked through state transitions with time-bound intervals

## Layers

**Instrumentation Layer:**
- Purpose: Capture telemetry as events from running applications
- Location: `crates/instrumentation/`, `crates/attributes/`, `crates/events/`
- Contains: Event types, trait definitions for tracing (spans) and resources (usage), instrumentation context
- Depends on: `crates/time/`, `crates/exporter/`
- Used by: Application code being profiled; simulator in `examples/simulator/`

**Collection Layer:**
- Purpose: Centralize events from multiple sources, serialize/deserialize, export for analysis
- Location: `crates/collector/server/`, `crates/collector/client/`, `crates/collector/proto/`, `crates/exporter/`
- Contains: gRPC collector service, multiple export formats (msgpack, ndjson, postcard, collector), import/export types
- Depends on: `crates/instrumentation/`, Tokio, Tonic, Prost
- Used by: Instrumentation layer producers; analyzer services for data ingestion

**Analysis Layer (Generic):**
- Purpose: Core event analysis primitives independent of domain
- Location: `crates/analyzer/`
- Contains: Entity trait, FSM trait, Resource/ResourceGroup traits, time utilities, timeline binning
- Key modules:
  - `fsm/`: FSM trait and state transition models (`crates/analyzer/src/fsm/mod.rs`)
  - `resource/`: Resource/ResourceGroup traits, usage tracking, resource trees (`crates/analyzer/src/resource/mod.rs`)
  - `timeline/binned/`: Time-binned resource timeline computation (`crates/analyzer/src/timeline/mod.rs`)
  - `trace/`: Distributed trace reconstruction from events (`crates/analyzer/src/trace/mod.rs`)
- Depends on: `crates/events/`, `crates/time/`, `crates/attributes/`
- Used by: Domain-specific analyzers

**Analysis Layer (Domain-Specific - Query Engine):**
- Purpose: Model query execution using FSM and resource abstractions
- Location: `domains/query_engine/analyzer/`, `domains/query_engine/events/`
- Contains: Query engine entities (Engine, Query, QueryGroup, Plan, Operator, Port, Worker), model view, UI analyzer
- Key structures:
  - `QueryEngineEvent`: Domain event type enum
  - `InMemoryQueryEngineModel`: Concrete model implementing `QueryEngineModel` trait (`domains/query_engine/analyzer/src/model.rs`)
  - `UiAnalyzer`: Trait for UI-specific query methods (`domains/query_engine/analyzer/src/ui.rs`)
- Depends on: `crates/analyzer/`, `crates/events/`
- Used by: Analyzer cache in server layer

**Server Layer:**
- Purpose: Expose collected/analyzed data via HTTP and gRPC APIs
- Location: `domains/query_engine/server/src/`
- Contains: Analyzer cache, timeline cache, HTTP routes (Axum), Swagger API docs, CORS configuration
- Key responsibilities:
  - Cache analyzed models in memory keyed by engine/query
  - Compute timelines on-demand via `timeline_cache`
  - Serve `/api/engines/*` HTTP routes for UI consumption
  - Optional UI embedding via feature flag
- Entry points:
  - `initialize_tracing()`: Set up logging with tracing-subscriber
  - `collector_service()`: Create gRPC collector service
  - `analyzer_service_router()`: Create HTTP analyzer router with state
- Depends on: Axum, Tonic, `domains/query_engine/analyzer/`
- Used by: Applications embedding the server (e.g., `examples/simulator/server/src/main.rs`)

**UI Frontend Layer:**
- Purpose: Interactive visualization of query execution and resource usage
- Location: `ui/src/`
- Contains: React SPA with TanStack Router, TanStack Query, Echarts visualization, Jotai atoms
- Architecture within UI:
  - Routes: File-based routing in `ui/src/routes/` (TanStack Router)
  - Pages: Single-page components in `ui/src/pages/`
  - Components: Reusable UI components in `ui/src/components/`
  - Services: API client stubs in `ui/src/services/api.ts`
  - Hooks: Data fetching hooks in `ui/src/hooks/` (e.g., `useQueryBundle`, `useBulkTimelines`)
  - Atoms: Jotai state atoms in `ui/src/atoms/` (timeline bins, DAG state)
  - Contexts: Theme context in `ui/src/contexts/ThemeContext.tsx`
- Entry point: `ui/src/main.tsx` - Creates router, QueryClient, renders React tree
- Depends on: React, TanStack Router/Query, Echarts, Tailwind CSS, MSW for mocking
- Used by: Web browsers via HTTP

## Data Flow

**Instrumentation → Collection → Analysis → Visualization:**

1. **Application Instrumentation Phase:**
   - Instrumented code calls methods on `quent_instrumentation::Context<T>` (e.g., `trace.span()`, `resource.usage()`)
   - Events are emitted to an `EventSender<T>` (unbounded mpsc channel)

2. **Event Export Phase:**
   - Events forwarded to exporter (determined by `ExporterOptions`)
   - Options: `CollectorExporterOptions` (gRPC), `MsgpackExporterOptions`, `NdjsonExporterOptions`, `PostcardExporterOptions`
   - Collector server (`quent_collector::server`) listens on gRPC, receives events, stores in memory

3. **Analysis Phase:**
   - Server receives request to analyze (e.g., `/api/engines/{engineId}`)
   - `AnalyzerCache` imports raw events via `ImporterFn` into `InMemoryQueryEngineModel`
   - Model is constructed by consuming `Event<QueryEngineEvent>` stream:
     - Events parsed into entity types (Query, Plan, Operator, etc.)
     - FSM transitions extracted and aggregated per entity
     - Resource usages tracked
   - Result is cached per engine ID

4. **Timeline Computation Phase:**
   - UI requests timeline for resource via `/api/engines/{engineId}/query/{queryId}/timeline`
   - Timeline parameters: time bins, resource ID, query ID
   - `TimelineCache` calls analyzer's timeline computation (e.g., `UiAnalyzer::resource_timeline()`)
   - Timeline binned data computed on-demand using `binned::ResourceTimeline`

5. **Visualization Phase:**
   - React components fetch model data via `useQueryBundle()`, `useBulkTimelines()` hooks
   - Data cached in TanStack Query
   - Components render DAG (Echarts), resource tree, timelines

**State Management:**
- Analyzer/timeline caches are in-memory, keyed by engine/query IDs
- UI uses TanStack Query for server state caching + Jotai atoms for client state (expanded tree nodes, highlighted items)
- Theme context provides dark/light mode state

## Key Abstractions

**Entity Trait:**
- Purpose: Represent anything with an ID and type name (queries, plans, resources, etc.)
- Pattern: All domain entities implement `Entity` trait with `id()`, `type_name()`, `instance_name()`
- Examples: `domains/query_engine/analyzer/src/query.rs`, `operator.rs`, `port.rs`

**Span Trait:**
- Purpose: Anything associated with a time interval
- Pattern: Implements `fn span(&self) -> AnalyzerResult<SpanUnixNanoSec>`
- Used for FSM states, traces, query execution duration

**FSM Trait:**
- Purpose: Model entity behavior as sequence of named state transitions
- Pattern: `Fsm` trait defines `len()`, `transition()`, `state()` accessors; entities impl via `FsmUsages` trait
- Example: A query has states: Init → Queued → Running → Completed (tracked via transitions)
- Location: `crates/analyzer/src/fsm/mod.rs`

**Resource/ResourceGroup Traits:**
- Purpose: Model hierarchical resource allocation and usage
- Pattern: `Resource` has parent group; `ResourceGroup` has optional parent group (tree structure)
- Usage tracked via `Usage<'a>` trait: capacity type (Occupancy or Rate), values, time span
- Example: Engine → Worker → ThreadPool → Thread (hierarchy); each uses CPU, memory, etc.
- Location: `crates/analyzer/src/resource/mod.rs`

**Model Trait:**
- Purpose: Central type-safe repository for looking up entities by ID
- Pattern: Generic trait; domains implement concrete types (e.g., `InMemoryQueryEngineModel`)
- Provides: `root()` for resource tree, `try_entity_ref()` for type-safe entity lookups
- Location: `crates/analyzer/src/lib.rs`; `domains/query_engine/analyzer/src/lib.rs`

**QueryEngineModel Trait:**
- Purpose: Domain-specific model interface with query engine entity lookups and iterators
- Pattern: Extends `Model` trait with methods like `engine()`, `query()`, `worker()`, `query_plans()`
- Provides: Convenience methods like `query_epoch()`, `query_workers()`
- Location: `domains/query_engine/analyzer/src/lib.rs`

## Entry Points

**Simulator Example:**
- Location: `examples/simulator/application/src/main.rs`
- Triggers: `cargo run --example simulator`
- Responsibilities:
  - Creates `SimulatorContext` with configured exporter
  - Simulates query execution with multiple workers, queries, operators
  - Emits FSM transitions for queries/plans/operators, resource usages for threads/memory
  - Can export to collector (gRPC), files (msgpack/ndjson/postcard)

**Analyzer Server:**
- Location: Embedded via `domains/query_engine/server/src/lib.rs` functions:
  - `collector_service()`: Returns gRPC router for event collection
  - `analyzer_service_router()`: Returns HTTP router for analysis/timeline API
- Usage: Applications call these to embed Quent analysis server
- Example: `examples/simulator/server/src/main.rs` starts both servers on different ports

**UI Frontend:**
- Location: `ui/src/main.tsx`
- Triggers: `npm run dev` or production build
- Responsibilities:
  - Initializes TanStack Router with file-based routes
  - Sets up TanStack Query client with default stale time
  - Renders React tree with theme provider
  - Routes: `/` → engine selection, `/profile/engine/{engineId}` → engine overview, `/profile/engine/{engineId}/query/{queryId}` → query details

## Error Handling

**Strategy:** Result-based error propagation using `AnalyzerResult<T>` = `Result<T, AnalyzerError>`

**Patterns:**
- Import errors bubble up from exporter: `AnalyzerError::Importer()`
- Validation errors (missing events, malformed state machines): `AnalyzerError::Validation()`
- Entity lookup failures: `AnalyzerError::InvalidId()`, `AnalyzerError::InvalidTypeName()`
- Time/span errors: `AnalyzerError::Time()`
- Incomplete entities (missing FSM transitions, resource data): `AnalyzerError::IncompleteEntity()`

**Example:**
```rust
// From crates/analyzer/src/lib.rs
pub type AnalyzerResult<T> = std::result::Result<T, AnalyzerError>;

// Usage in FSM span computation
impl<U: Fsm> Span for U {
    fn span(&self) -> AnalyzerResult<SpanUnixNanoSec> {
        if let Some(start) = self.first().map(|s| s.span().start())
            && let Some(end) = self.last().map(|s| s.span().end())
        {
            Ok(SpanUnixNanoSec::try_new(start, end)?)
        } else {
            Err(AnalyzerError::IncompleteEntity(format!("fsm is incomplete")))
        }
    }
}
```

## Cross-Cutting Concerns

**Logging:** Tracing infrastructure via `tracing` and `tracing-subscriber` crates
- Server initialization: `initialize_tracing()` sets up subscriber with configurable log level, targets, span events
- Field: Uses structured logging with KV pairs
- UI: Console logging in development; no production logging configured

**Validation:** Performed at model construction time
- Events validated when imported (missing transitions, state machine violations)
- Entities validated on lookup (ID must exist, type must match)
- Time spans validated (start < end, valid nanosecond timestamps)

**Authentication:** Not implemented
- Collector server and analyzer API are unauthenticated
- UI has no auth layer
- CORS configuration available but not enforced by default

**Serialization:** Event-centric serialization via serde
- Events serialized in transit (exporter formats)
- Types use `#[serde(Serialize, Deserialize)]` derives
- TypeScript bindings generated via `ts-rs` crate for UI types
- BigInt handling in UI: Custom JSON parser (`parseJsonWithBigInt`) for nanosecond timestamps

---

*Architecture analysis: 2026-03-25*
