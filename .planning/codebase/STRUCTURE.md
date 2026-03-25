# Codebase Structure

**Analysis Date:** 2026-03-25

## Directory Layout

```
project-root/
├── crates/                     # Shared, domain-agnostic libraries
│   ├── analyzer/              # Core analysis traits and primitives
│   ├── attributes/            # Event attributes (key-value pairs)
│   ├── collector/             # Event collection infrastructure
│   │   ├── server/           # gRPC collector service
│   │   ├── client/           # gRPC collector client
│   │   └── proto/            # Protobuf definitions
│   ├── events/               # Event type definitions
│   ├── exporter/             # Data export formats and exporters
│   │   ├── collector/        # Exporter for gRPC collector
│   │   ├── msgpack/          # msgpack binary format
│   │   ├── ndjson/           # Newline-delimited JSON
│   │   ├── postcard/         # Compact binary format
│   │   ├── src/              # Exporter abstraction
│   │   └── types/            # Exporter trait definitions
│   ├── instrumentation/       # Application instrumentation API
│   ├── time/                 # Time utilities and types
│   └── ui/                   # Axum HTTP server utilities for UI serving
├── domains/                   # Domain-specific analyzers and models
│   └── query_engine/         # Query execution analysis domain
│       ├── analyzer/         # Query engine model and analysis logic
│       ├── events/           # Query engine event types
│       ├── server/           # HTTP API server with caching
│       └── ui/               # UI types and display utilities
├── examples/
│   └── simulator/            # Example query engine simulator
│       ├── analyzer/         # Simulator model
│       ├── application/      # Simulator main (generates fake telemetry)
│       ├── events/           # Simulator event types
│       ├── instrumentation/  # Simulator instrumentation wrapper
│       ├── server/           # Simulator server (embeds Quent analyzer)
│       └── ui/               # UI types for simulator
├── ui/                       # React frontend application
│   ├── src/
│   │   ├── atoms/           # Jotai state atoms
│   │   ├── components/      # React components
│   │   │   ├── dag/        # DAG visualization (Echarts-based)
│   │   │   ├── query-plan/ # Query plan visualization
│   │   │   ├── resource-tree/ # Resource hierarchy tree
│   │   │   ├── timeline/   # Timeline charts
│   │   │   └── ui/         # Shadcn/ui base components
│   │   ├── contexts/       # React contexts (theme)
│   │   ├── hooks/          # Custom React hooks (data fetching)
│   │   ├── lib/            # Utilities and helpers
│   │   ├── pages/          # Page components (fallback routes)
│   │   ├── routes/         # TanStack Router file-based routes
│   │   ├── services/       # API client stubs
│   │   ├── test/           # Testing setup, mocks, utilities
│   │   ├── main.tsx        # React entry point
│   │   ├── index.css       # Global styles
│   │   └── types.ts        # TypeScript types
│   ├── package.json         # Dependencies, scripts
│   ├── vite.config.ts       # Vite build config
│   └── eslint.config.js     # ESLint rules
├── proto/                   # Protocol buffer definitions
│   └── quent/
│       └── collector/
│           └── v1/
│               └── collector.proto
├── data/                    # Data and resources
├── .planning/              # GSD planning documents
│   └── codebase/          # Analysis documents (ARCHITECTURE.md, STRUCTURE.md, etc.)
├── Cargo.toml             # Rust workspace manifest
├── Cargo.lock             # Dependency lock file
├── Dockerfile             # Container image definition
├── docker-compose.yml     # Local development setup
├── pixi.toml              # Pixi environment config
└── README.md              # Project documentation
```

## Directory Purposes

**crates/analyzer:**
- Purpose: Generic event analysis primitives (FSM, resources, timelines)
- Contains: Trait definitions, time-bound state machines, resource usage models
- Key files:
  - `lib.rs`: Core traits (Entity, Instant, Span, Model)
  - `fsm/mod.rs`: FSM trait and state models
  - `resource/mod.rs`: Resource/ResourceGroup traits
  - `timeline/binned/resource.rs`: Time-binned resource timeline computation (865 lines - largest file)
  - `trace/mod.rs`: Distributed trace reconstruction (239 lines)

**crates/events:**
- Purpose: Event type definitions for instrumentation
- Contains: Generic `Event<T>` wrapper, resource event types, trace event types
- Key files:
  - `lib.rs`: Generic Event struct
  - `resource.rs`: Resource event types (channel, memory, etc.)
  - `trace.rs`: Trace span event types (Init, Span, Enter, Exit, Close)

**crates/instrumentation:**
- Purpose: Application instrumentation API for emitting events
- Contains: EventSender, Context, resource/trace wrappers for user code
- Key files:
  - `lib.rs`: Context initialization, exporter setup (220 lines)
  - `resource.rs`: Resource usage instrumentation API (189 lines)
  - `trace.rs`: Distributed tracing span instrumentation API

**crates/exporter:**
- Purpose: Serialize and export events in various formats
- Contains: Exporter trait, concrete implementations (msgpack, ndjson, postcard, collector)
- Key files:
  - `src/lib.rs`: Exporter abstraction, factory function
  - `types/src/lib.rs`: Exporter trait, ImporterError
  - `msgpack/src/lib.rs`: MessagePack binary format
  - `ndjson/src/lib.rs`: Newline-delimited JSON (human-readable)
  - `postcard/src/lib.rs`: Compact binary format
  - `collector/src/lib.rs`: gRPC collector exporter

**crates/collector:**
- Purpose: Centralized event collection service
- Contains: gRPC server for receiving events, client for sending events
- Key files:
  - `server/src/lib.rs`: CollectorService implementation
  - `client/src/lib.rs`: CollectorClient for sending events (236 lines)
  - `proto/src/lib.rs`: Protobuf-generated code

**crates/time:**
- Purpose: Time utilities for nanosecond-precision Unix timestamps and spans
- Contains: TimeUnixNanoSec, SpanUnixNanoSec, binning logic
- Key files:
  - `lib.rs`: Time type wrappers
  - `span.rs`: Span operations and binning (423 lines)
  - `bin.rs`: Time-based binning for aggregation (422 lines)

**crates/ui:**
- Purpose: Axum HTTP utilities for embedding the UI server
- Contains: Router setup with swagger support, embedded UI fallback
- Key files:
  - `lib.rs`: UI routes, swagger generation, embedded file serving (252 lines)
  - `quantity.rs`: Resource quantity specifications for UI

**domains/query_engine/analyzer:**
- Purpose: Domain-specific model for analyzing query execution
- Contains: Query engine entities (Query, Plan, Operator, Worker, Port), model implementation
- Key files:
  - `lib.rs`: QueryEngineModel trait (100+ lines)
  - `model.rs`: InMemoryQueryEngineModel concrete implementation (13,035 lines - substantial logic)
  - `engine.rs`: Engine entity definition
  - `query.rs`: Query entity with state machine (6,698 lines)
  - `operator.rs`: Operator entity (4,592 lines)
  - `worker.rs`: Worker/thread resource entity (2,926 lines)
  - `ui.rs`: UI-specific query methods (exposed to API) (2,503 lines)
  - `view.rs`: In-memory model view with serialization (8,636 lines)
  - `plan/mod.rs` and `plan/tree.rs`: Execution plan modeling
  - `port.rs`: Data port entity (3,054 lines)
  - `query_group.rs`: Query grouping entity (1,831 lines)

**domains/query_engine/events:**
- Purpose: Query engine-specific event types
- Contains: Event variants for queries, plans, operators, workers, ports

**domains/query_engine/server:**
- Purpose: HTTP API server for exposing analysis results
- Contains: Analyzer cache, timeline cache, Axum route handlers, CORS configuration
- Key files:
  - `lib.rs`: Server initialization, service factories
  - `analyzer_cache.rs`: In-memory cache of analyzed models
  - `timeline_cache.rs`: On-demand timeline computation cache
  - `state.rs`: Service state container
  - `ui/`: HTTP routes and handlers (`/api/engines/*`)

**examples/simulator:**
- Purpose: Fully functional example application generating simulated query telemetry
- Contains: Fake query engine with workers, queries, plans, operators; FSM state transitions; resource usage
- Key files:
  - `application/src/main.rs`: Main entry point, argument parsing, simulation loop (400+ lines)
  - `analyzer/src/model.rs`: Simulator-specific model view
  - `instrumentation/src/lib.rs`: SimulatorContext wrapper around instrumentation API
  - `server/src/main.rs`: Starts both collector and analyzer servers, serves UI

**ui/src:**
- Purpose: React single-page application frontend
- Contains: Components, routes, data fetching, state management
- Structure:
  - `routes/`: File-based routing via TanStack Router
    - `__root.tsx`: Root layout with navbar, theme toggle
    - `index.tsx`: Home page (engine selection)
    - `profile.tsx`, `profile.index.tsx`: Profile layout and overview
    - `profile.engine.$engineId.tsx`: Engine detail layout (resizable panels)
    - `profile.engine.$engineId.query.$queryId.index.tsx`: Query overview
    - `profile.engine.$engineId.query.$queryId.node.$nodeId.tsx`: Node/operator detail
  - `components/`: React components
    - `dag/DAGChart.tsx`, `dag/DAGControls.tsx`: Graph visualization
    - `query-plan/QueryPlanNode.tsx`: Plan node rendering
    - `resource-tree/ResourceRow.tsx`, `ResourceGroupRow.tsx`: Tree display
    - `timeline/ResourceTimeline.tsx`: Timeline chart component
    - `ui/`: Shadcn/ui components (button, card, dropdown, etc.)
  - `hooks/`: TanStack Query hooks
    - `useQueryBundle.ts`: Fetch query model data
    - `useBulkTimelines.ts`: Fetch multiple timelines in batch
  - `atoms/`: Jotai atoms for client state
    - `timeline.ts`: Timeline bins state
    - `dag.ts`: DAG node selection state
  - `services/api.ts`: API client stubs with base URL configuration
  - `lib/`: Utilities
    - `queryClient.ts`: TanStack Query client configuration
    - `echarts.ts`: Echarts theme configuration
    - `resource.utils.ts`: Resource name parsing
    - `timeline.utils.ts`: Timeline data transformation
  - `test/`: Testing setup and mocks

**proto/:**
- Purpose: Protocol buffer definitions for gRPC services
- Contains: `collector.proto` defining collector service RPC interface

**.planning/codebase/:**
- Purpose: GSD planning documents
- Contains: ARCHITECTURE.md, STRUCTURE.md, CONVENTIONS.md, TESTING.md, CONCERNS.md

## Key File Locations

**Entry Points:**
- `examples/simulator/application/src/main.rs`: Simulator that generates fake telemetry
- `examples/simulator/server/src/main.rs`: Server that starts collector and analyzer services
- `ui/src/main.tsx`: React frontend entry point
- `domains/query_engine/server/src/lib.rs`: Exported server construction functions

**Configuration:**
- `Cargo.toml`: Rust workspace manifest with all crates and dependencies
- `Cargo.lock`: Locked dependency versions
- `ui/package.json`: NPM dependencies and build scripts
- `ui/vite.config.ts`: Vite bundler configuration
- `ui/eslint.config.js`: JavaScript linting rules
- `Dockerfile`: Container build for simulator server
- `docker-compose.yml`: Local development containers

**Core Logic:**
- `crates/analyzer/src/fsm/mod.rs`: FSM trait and state machine definitions
- `crates/analyzer/src/resource/mod.rs`: Resource and resource group abstractions
- `crates/analyzer/src/timeline/binned/resource.rs`: Timeline binning algorithm (865 lines)
- `domains/query_engine/analyzer/src/model.rs`: Query engine model construction (13,035 lines)
- `ui/src/services/api.ts`: API client (base URL, fetch wrappers)
- `ui/src/routes/__root.tsx`: Root layout and navigation

**Testing:**
- `ui/src/test/setup.ts`: Vitest configuration
- `ui/src/test/mocks/server.ts`: MSW mock server for API
- `ui/src/test/mocks/handlers.ts`: API route handlers
- `ui/src/test/test-utils.tsx`: React testing utilities
- `ui/src/routes/profile.index.test.tsx`: Example test

## Naming Conventions

**Files:**
- Rust: `snake_case.rs` (modules), `lib.rs` (crate root), `mod.rs` (module files)
- TypeScript: `camelCase.ts` (files), `PascalCase.tsx` (components)
- Routes: `[segment].tsx` for static, `$param.tsx` for route parameters

**Directories:**
- Rust: `snake_case/` (modules follow domain structure)
- TypeScript: `camelCase/` for features (`components/`, `hooks/`, `services/`)
- TypeScript: `PascalCase/` for grouped components (e.g., `ui/` for base components)

**Rust Types:**
- Traits: `PascalCase` ending in `Trait` or descriptive (e.g., `Entity`, `Fsm`, `Model`)
- Structs: `PascalCase` (e.g., `InMemoryQueryEngineModel`, `RtSpan`, `AnalyzerCache`)
- Enums: `PascalCase` (e.g., `AnalyzerError`, `QueryEngineEvent`, `CapacityType`)
- Functions: `snake_case` (e.g., `try_new`, `try_build`, `initialize_tracing`)
- Constants: `SCREAMING_SNAKE_CASE` (rare; mostly uses type system)

**TypeScript Types:**
- Interfaces: `PascalCase` (e.g., `QueryBundle`, `EntityRef`, `QueryBundleParams`)
- Types: `PascalCase` (e.g., `Engine`, `Query`, `DAGNode`)
- Functions: `camelCase` (e.g., `fetchQueryBundle`, `useQueryBundle`)
- Constants: `camelCase` (e.g., `DEFAULT_STALE_TIME`, `API_BASE_URL`)

## Where to Add New Code

**New Feature (e.g., new resource type):**
- Generic resource definition: `crates/events/src/resource.rs` (event type)
- Instrumentation API: `crates/instrumentation/src/resource.rs` (user-facing wrapper)
- Domain logic: `domains/query_engine/events/src/lib.rs` (query engine event variant)

**New Entity Type (e.g., Thread):**
- Entity definition: New file in `domains/query_engine/analyzer/src/` (e.g., `thread.rs`)
- Model integration: Add HashMap in `InMemoryQueryEngineModel` (`domains/query_engine/analyzer/src/model.rs`)
- Event handling: Update `QueryEngineModel` trait with lookup method
- Server routes: Add endpoint in `domains/query_engine/server/src/ui/`

**New UI Component:**
- Simple component: `ui/src/components/[feature]/ComponentName.tsx`
- Page/route: `ui/src/routes/[path].tsx` (TanStack Router convention)
- Data fetching hook: `ui/src/hooks/use[Feature].ts` (uses TanStack Query)
- State atom: `ui/src/atoms/[feature].ts` (Jotai)
- Utilities: `ui/src/lib/[feature].utils.ts`

**New Analyzer Domain (e.g., storage system):**
- Create `domains/storage/` parallel to `domains/query_engine/`
- Structure:
  - `analyzer/`: Traits/structs for storage entities
  - `events/`: Storage-specific event types
  - `server/`: HTTP API routes and caching
  - `ui/`: UI types (if needed)
- Implement `QueryEngineModel`-like trait for new domain

**Utilities:**
- Shared helpers: `crates/` (new crate if significant)
- UI utilities: `ui/src/lib/` (e.g., formatting, parsing)
- Rust utilities: `crates/[feature]/src/lib.rs` or subdirectory

## Special Directories

**crates/:**
- Purpose: Shared libraries used by multiple domains
- Generated: No
- Committed: Yes
- Accessed by: `domains/`, `examples/`, UI via generated bindings

**domains/:**
- Purpose: Domain-specific implementations (query engine, future storage, etc.)
- Generated: No
- Committed: Yes
- Accessed by: Applications embedding Quent, simulator example

**examples/:**
- Purpose: Example usage demonstrating full system
- Generated: No (but generates telemetry at runtime)
- Committed: Yes
- Accessed by: Users learning Quent, developers testing

**ui/:**
- Purpose: Frontend SPA
- Generated: `routeTree.gen.ts` (TanStack Router), build outputs in `dist/`
- Committed: No for generated files; yes for sources
- Accessed by: Web browsers

**target/:**
- Purpose: Build artifacts (Rust compiled binaries, dependencies)
- Generated: Yes
- Committed: No
- Gitignored: Yes

**.planning/codebase/:**
- Purpose: GSD mapping documents for orchestrator
- Generated: Yes (by `gsd:map-codebase` command)
- Committed: Yes (for reference)
- Accessed by: Other GSD commands (`gsd:plan-phase`, `gsd:execute-phase`)

---

*Structure analysis: 2026-03-25*
