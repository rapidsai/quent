# Architecture

**Analysis Date:** 2026-04-01

## Pattern Overview

**Overall:** Modular multi-domain telemetry platform with client-server separation

**Key Characteristics:**
- Core telemetry modeling framework (Entities, FSMs, Resources, Capacities, Usages)
- Rust backend with domain-specific implementations (Query Engine focused)
- React frontend for interactive visualization of query plans and resource timelines
- Pluggable analyzers and exporters for extensibility
- Typed instrumentation API generated from declarative models

## Layers

**Instrumentation Layer:**
- Purpose: Provides type-safe API for applications to emit telemetry
- Location: `crates/instrumentation`, `crates/attributes`
- Contains: Macro-based API generation, event emission utilities
- Depends on: Core modeling types (`crates/events`, `crates/analyzer`)
- Used by: Domain-specific servers (Query Engine simulator, domain implementations)

**Event & Collection Layer:**
- Purpose: Collects raw events from instrumented applications via gRPC
- Location: `crates/collector/server`, `crates/collector/client`, `crates/collector/proto`
- Contains: gRPC service definitions, event collection logic, protobuf schemas
- Depends on: Tokio, Tonic, Prost for async runtime and serialization
- Used by: Backend servers that expose analysis endpoints

**Analysis Layer:**
- Purpose: Reconstructs application models from raw events and computes derived data
- Location: `crates/analyzer`, `domains/query_engine/analyzer`, `examples/simulator/analyzer`
- Contains: FSM runtime, resource tree construction, timeline computation, entity resolution
- Depends on: Event types, timing utilities (`crates/time`)
- Used by: Server HTTP/UI layer to expose query results

**Export Layer:**
- Purpose: Serializes analyzed data in various formats for transport
- Location: `crates/exporter`, `crates/exporter/msgpack`, `crates/exporter/ndjson`, `crates/exporter/postcard`
- Contains: Format-specific serializers (MessagePack, NDJSON, Postcard)
- Depends on: Core UI types from `crates/ui` for TypeScript bindings
- Used by: Server endpoints returning data to frontend

**UI Integration Layer:**
- Purpose: Bridges Rust backend types to frontend TypeScript types
- Location: `crates/ui`, `domains/query_engine/ui`
- Contains: TypeScript-serializable types (via ts-rs), resource and entity definitions
- Depends on: `crates/analyzer` types, serialization framework
- Used by: Server ts-bindings generation, frontend type definitions

**Frontend Layer:**
- Purpose: Interactive visualization and exploration of query plans and metrics
- Location: `ui/src`
- Contains: React components, routing, state management, visualization logic
- Depends on: Backend API endpoints, React ecosystem libraries
- Used by: End users for performance analysis and debugging

## Data Flow

**Query Plan Visualization Flow:**

1. User navigates to Engine → Query view in UI (`/profile/engine/{engineId}/query/{queryId}`)
2. Route loaders trigger `fetchQueryBundle()` via React Query
3. API request to `/api/engines/{engineId}/query/{queryId}` hits backend server
4. Backend analyzer reconstructs QueryBundle from stored events:
   - Resolves plan tree structure from Plan entities and their FSM states
   - Associates operators, ports, and workers with plans
   - Computes plan metadata
5. Response serialized via `crates/exporter` and returned as JSON
6. Frontend receives QueryBundle and transforms via `getTreeData()` / `getPlanDAG()`
7. TreeView displays plan hierarchy, DAGChart renders operator graph via ELK auto-layout
8. User interactions (node selection, tree navigation) update Jotai atoms
9. Connected components (timeline, resource tree) reactively update based on atom state

**Timeline Data Flow:**

1. Timeline component calls `fetchBulkTimelines()` or `fetchSingleTimeline()` with filters
2. Backend analyzer computes resource utilization timelines from events
3. Data returned as time-series of resource usage snapshots
4. Frontend renders via ECharts (custom echarts.ts build for tree-shaking)
5. TimelineController manages pan/zoom/playback state via hooks

**State Management:**
- Global UI state: Jotai atoms in `atoms/dag.ts` and `atoms/timeline.ts` (selected nodes, plans, hovered workers)
- Server state: React Query cache in `lib/queryClient.ts` with 5-minute stale time
- Component state: Local React hooks for interactive controls (TreeView, Timeline pan/zoom)

## Key Abstractions

**Model Trait:**
- Purpose: Unifies different application models (Query Engine, Simulator) under common interface
- Examples: `crates/analyzer/src/lib.rs`, `domains/query_engine/analyzer/src/lib.rs`
- Pattern: Trait-based polymorphism allowing generic analysis code to work with domain-specific types

**Entity Traits (Entity, Instant, Span):**
- Purpose: Abstract common properties of telemetry entities (Query, Operator, Resource, etc.)
- Pattern: Marker traits providing id, type_name, instance_name, and temporal properties
- Used to: Enable generic FSM runtime and timeline computation

**ResourceCollection & ResourceGroup:**
- Purpose: Hierarchical representation of resources with nested groups and aggregated usage
- Pattern: Tree structure with parent-child relationships
- Used to: Compute utilization at any level of abstraction

**FSM (Finite State Machine) Runtime:**
- Purpose: Validates event sequences against declared state transitions
- Location: `crates/analyzer/src/fsm/runtime.rs`
- Pattern: State machine that processes events and detects violations
- Used to: Ensure data integrity during reconstruction

**QueryBundle:**
- Purpose: Complete query execution snapshot for visualization
- Pattern: Combines plan tree, operators, ports, workers, and associated metadata
- Used to: Single API response containing all data needed for query plan visualization

## Entry Points

**Frontend Entry:**
- Location: `ui/src/main.tsx`
- Triggers: User opens application in browser
- Responsibilities: Initialize React app with Router, QueryClient, theme provider; render root layout

**Backend API Endpoints:**
- Listing: `/engines` - List available engines with metadata
- Listing: `/engines/{engineId}/query-groups` - List coordinators/query groups
- Listing: `/engines/{engineId}/query_group/{coordinatorId}/queries` - List queries in coordinator
- Fetching: `/engines/{engineId}/query/{queryId}` - Fetch complete QueryBundle for visualization
- Streaming: `/engines/{engineId}/timeline/single` - Fetch single timeline with filters (POST)
- Streaming: `/engines/{engineId}/timeline/bulk` - Fetch bulk timelines with filters (POST)

**Routes:**
- Location: `ui/src/routes/` (generated via TanStack Router)
- `/` - Engine selection page
- `/profile/engine/{engineId}` - Engine detail view
- `/profile/engine/{engineId}/query/{queryId}` - Query plan and timeline visualization
- `/profile/engine/{engineId}/query/{queryId}/node/{nodeId}` - Operator node detail view

**Backend Server Initialization:**
- Location: `domains/query_engine/server/src/lib.rs`
- Registers: HTTP router (Axum), gRPC collector, analyzer caches
- Features: CORS support, optional Swagger UI, optional static UI serving

## Error Handling

**Strategy:** Defensive error handling with typed results and user-facing messages

**Patterns:**
- Rust backend uses `AnalyzerResult<T>` wrapper and `AnalyzerError` enum for domain errors
- Frontend API layer catches HTTP errors and throws descriptive messages
- Components display error states with loading skeletons and retry mechanisms
- BigInt JSON parsing handles numeric overflow gracefully
- Timeline controller gracefully degrades when time binning data

## Cross-Cutting Concerns

**Logging:**
- Backend: Structured logging via `tracing` crate with configurable levels
- Frontend: Console logging for development, no structured logs in production

**Validation:**
- Backend: FSM runtime validates event sequences during reconstruction
- Frontend: Type safety via TypeScript strict mode, path aliases prevent circular imports

**Authentication:**
- Not implemented (assumed same-origin or trusted network deployment)
- CORS configured for development (localhost:5173)

**Data Consistency:**
- Timeline computations are deterministic (no caching of derived values across requests)
- Plan DAG layout is computed client-side from QueryBundle for consistency
- Resource tree is reconstructed from events on each analysis request

---

*Architecture analysis: 2026-04-01*
