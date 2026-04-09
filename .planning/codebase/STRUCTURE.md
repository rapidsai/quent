# Codebase Structure

**Analysis Date:** 2026-04-01

## Directory Layout

```
quent/
├── crates/                    # Core Rust workspace members
│   ├── analyzer/              # Domain-agnostic telemetry analysis engine
│   ├── attributes/            # Macro utilities for model declaration
│   ├── collector/             # Event collection service
│   │   ├── server/            # gRPC collector server
│   │   ├── client/            # Client for sending events
│   │   └── proto/             # Protobuf definitions
│   ├── events/                # Core event type definitions
│   ├── exporter/              # Data serialization formats
│   │   ├── src/               # Base exporter traits
│   │   ├── msgpack/           # MessagePack encoder
│   │   ├── ndjson/            # NDJSON encoder
│   │   ├── postcard/          # Postcard encoder
│   │   ├── collector/         # Collector-specific exporter
│   │   └── types/             # Shared type definitions
│   ├── instrumentation/       # Type-safe instrumentation API generator
│   ├── server/                # Server utilities and bindings
│   │   └── ts-bindings/       # TypeScript type bindings
│   ├── time/                  # Time utilities and span calculations
│   └── ui/                    # UI bridge types and timeline request/response
├── domains/                   # Domain-specific implementations
│   └── query_engine/          # Query Engine specialization
│       ├── analyzer/          # Query Engine model reconstruction
│       ├── events/            # Query Engine specific events
│       ├── server/            # Query Engine server implementation
│       └── ui/                # Query Engine UI types
├── examples/                  # Example implementations
│   └── simulator/             # Simulated query engine for testing
│       ├── application/       # Simulation logic
│       ├── events/            # Simulator event types
│       ├── instrumentation/   # Simulator instrumentation
│       ├── analyzer/          # Simulator model
│       ├── server/            # Simulator server with HTTP/gRPC
│       │   └── ts-bindings/   # Generated TypeScript from Simulator model
│       └── ui/                # Simulator UI types
├── ui/                        # Frontend React application
│   ├── src/
│   │   ├── main.tsx           # App entry point (React, Router, QueryClient)
│   │   ├── atoms/             # Jotai state atoms (DAG, timeline selection)
│   │   ├── components/        # Reusable React components
│   │   │   ├── dag/           # DAG visualization (ReactFlow, ELK)
│   │   │   ├── query-plan/    # Query plan tree and node rendering
│   │   │   ├── resource-tree/ # Resource hierarchy display
│   │   │   ├── timeline/      # Timeline visualization (ECharts)
│   │   │   └── ui/            # Radix UI wrapper components
│   │   ├── contexts/          # React contexts (Theme)
│   │   ├── hooks/             # Custom hooks (data fetching, visualization)
│   │   ├── lib/               # Utilities (query client, formatters, transformers)
│   │   ├── pages/             # Legacy page components (migrating to routes/)
│   │   ├── routes/            # TanStack Router file-based routes (auto-generated)
│   │   ├── services/          # API communication layer
│   │   │   ├── api.ts         # Generic fetch + endpoint definitions
│   │   │   ├── colors.ts      # Operation type color mappings
│   │   │   ├── formatters.ts  # Data formatting utilities
│   │   │   └── query-plan/    # Query plan transformation logic
│   │   └── test/              # Test utilities and mocks
│   ├── package.json           # Dependencies, build scripts
│   ├── vite.config.ts         # Build config with proxy, chunk splitting
│   ├── tsconfig.json          # TypeScript strict mode, path aliases
│   └── public/                # Static assets
├── docs/                      # Documentation
│   ├── domains/               # Domain modeling guides
│   └── modeling/              # General modeling concepts
├── proto/                     # Protocol buffer definitions (shared)
├── data/                      # Test data and examples
├── Cargo.toml                 # Rust workspace config
├── Cargo.lock                 # Dependency lock
├── pixi.toml                  # Environment management
├── pixi.lock                  # Environment lock
└── README.md                  # Project overview
```

## Directory Purposes

**`crates/analyzer`:**
- Purpose: Domain-agnostic telemetry reconstruction and analysis engine
- Contains: FSM runtime, resource tree building, timeline computation, entity traits
- Key files: `src/lib.rs` (trait definitions), `src/fsm/runtime.rs` (FSM engine), `src/resource/` (hierarchical resources)

**`crates/instrumentation`:**
- Purpose: Generates type-safe instrumentation APIs from models
- Contains: Macro definitions for declaring models, traits, and events
- Key files: `src/lib.rs` (public API macros)

**`crates/collector`:**
- Purpose: gRPC service for event ingestion
- Contains: Protobuf service definition, server implementation, client library
- Key files: `server/src/server.rs` (service logic), `proto/src/lib.rs` (generated code)

**`crates/events`:**
- Purpose: Core event type definitions
- Contains: Serializable event structures for Queries, Operators, Resources, Workers
- Key files: `src/lib.rs` (event enum), `src/resource.rs`, `src/trace.rs`

**`crates/exporter`:**
- Purpose: Pluggable data serialization for transport
- Contains: Format-specific encoders (MessagePack, NDJSON, Postcard)
- Key files: Individual encoder crates define encoding logic

**`crates/time`:**
- Purpose: Time utilities for nanosecond-precision timestamps and spans
- Contains: TimeUnixNanoSec type, span calculations, binning utilities
- Key files: `src/lib.rs` (core types), `src/span.rs` (span logic)

**`crates/ui`:**
- Purpose: Types bridge from Rust backend to TypeScript frontend
- Contains: Serializable versions of core types using ts-rs
- Key files: `src/lib.rs` (Resource, ResourceGroup, Plan types), `src/timeline/` (timeline request/response)

**`domains/query_engine/analyzer`:**
- Purpose: Query Engine specific model reconstruction
- Contains: Query, Plan, Operator, Port, Worker entities and their FSM implementations
- Key files: `src/lib.rs` (Model implementation), `src/operator.rs`, `src/plan/tree.rs`

**`domains/query_engine/server`:**
- Purpose: HTTP API server for Query Engine telemetry
- Contains: Axum routes, analyzer caching, timeline computation
- Key files: `src/lib.rs` (server initialization), `src/ui.rs` (API endpoints)

**`examples/simulator/server`:**
- Purpose: Example server for development and testing
- Contains: HTTP API with generated TypeScript bindings
- Key files: `src/main.rs`, `ts-bindings/` (auto-generated from types)

**`ui/src/main.tsx`:**
- Purpose: Application entry point
- Initializes: React app, TanStack Router, React Query client, theme provider

**`ui/src/routes/`:**
- Purpose: File-based routing via TanStack Router
- Structure: `__root.tsx` (layout), `index.tsx` (home), `profile.*.tsx` (nested routes)
- Generated: `routeTree.gen.ts` (auto-generated route tree)

**`ui/src/components/dag/`:**
- Purpose: DAG visualization for query plans
- Contains: ReactFlow setup, ELK auto-layout, node types, minimap
- Key files: `DAGChart.tsx` (main component), `DAGControls.tsx`

**`ui/src/components/query-plan/`:**
- Purpose: Query plan tree and node rendering
- Contains: Custom QueryPlanNode component (colored by operation type)
- Pattern: Map operation types to colors via colors.ts service

**`ui/src/components/timeline/`:**
- Purpose: Resource timeline visualization
- Contains: ECharts integration, timeline controls, filtering
- Key files: `Timeline.tsx` (main), `TimelineController.tsx` (state)

**`ui/src/services/api.ts`:**
- Purpose: API communication layer
- Contains: Generic `apiFetch()` helper, BigInt JSON parsing, endpoint definitions
- Pattern: Each fetch function calls `apiFetch()` with typed generics

**`ui/src/services/query-plan/`:**
- Purpose: Query plan transformation logic
- Contains: Conversion from QueryBundle to TreeView and DAG formats
- Key files: `query-bundle-transformer.ts` (getTreeData, getPlanDAG)

**`ui/src/atoms/`:**
- Purpose: Global Jotai state atoms
- Contains: Selected nodes, selected plan, hovered workers
- Key files: `dag.ts` (DAG state), `timeline.ts` (timeline state)

**`ui/src/hooks/`:**
- Purpose: Custom React hooks for data fetching and visualization
- Contains: useQueryBundle, useQueryPlanVisualization, useBulkTimelines, etc.
- Pattern: Hooks wrapping React Query and transformation logic

**`ui/src/lib/`:**
- Purpose: Utility functions and configuration
- Contains: queryClient setup, formatters, color utilities, echarts custom build
- Key files: `queryClient.ts` (React Query config), `queryBundle.utils.ts`, `timeline.utils.ts`

**`docs/`:**
- Purpose: Documentation and guides
- Contains: Modeling concepts, domain-specific model specs
- Key files: `modeling/` (FSM, Resource, Capacity, Usage docs), `domains/` (Query Engine spec)

## Key File Locations

**Entry Points:**
- Frontend: `ui/src/main.tsx` - Initializes React app and providers
- Frontend routes: `ui/src/routes/__root.tsx` - Root layout and nav
- Backend server: `examples/simulator/server/src/main.rs` - HTTP and gRPC setup
- Backend analyzer: `domains/query_engine/analyzer/src/lib.rs` - Model trait impl

**Configuration:**
- Build: `ui/vite.config.ts` - Vite setup with proxy, chunk splitting, plugins
- Types: `ui/tsconfig.json` - TypeScript strict mode, path aliases
- Workspace: `Cargo.toml` - Rust crate organization, shared dependencies
- Environment: `pixi.toml` - Development environment (Rust, Node, protoc versions)

**Core Logic:**
- Analyzer: `crates/analyzer/src/lib.rs` - Entity and Model traits, FSM runtime
- UI types: `crates/ui/src/lib.rs` - Serializable types for frontend
- Query Engine model: `domains/query_engine/analyzer/src/lib.rs` - Query, Plan, Operator impl
- API service: `ui/src/services/api.ts` - Fetch helpers and endpoints
- DAG visualization: `ui/src/components/dag/DAGChart.tsx` - ReactFlow integration

**Testing:**
- Mocks: `ui/src/test/mocks/` - MSW handlers for API mocking
- Fixtures: Test data in `data/` directory
- Test files: Named `*.test.ts` or `*.test.tsx` in source directories

## Naming Conventions

**Files:**
- React components: PascalCase (`QueryPlan.tsx`, `DAGChart.tsx`)
- Services/utilities: camelCase (`api.ts`, `formatters.ts`)
- Test files: `*.test.tsx`, `*.spec.ts`
- Route files: kebab-case for nested routes, indexed params (`profile.engine.$engineId.tsx`)

**Directories:**
- Feature directories: kebab-case (`query-plan`, `resource-tree`)
- Grouped by function: `components/`, `services/`, `hooks/`, `utils/`, `types/`
- Domain directories: snake_case (`query_engine`, `simulator`)

**Variables/Functions:**
- Constants: UPPER_SNAKE_CASE (`DEFAULT_STALE_TIME`, `API_BASE_URL`)
- Functions: camelCase (`fetchQueryBundle`, `getTreeData`)
- React hooks: start with `use` (`useQueryBundle`, `useBulkTimelines`)
- Atoms: end with `Atom` (`selectedNodeIdsAtom`, `selectedPlanIdAtom`)
- Types: PascalCase (`QueryBundle`, `DAGData`, `EntityRef`)

**Color/Style Tokens:**
- Operation types: Map in `services/colors.ts` (`scan`, `join`, `aggregate`, etc.)
- Tailwind classes: Use via clsx/cn utilities

## Where to Add New Code

**New Feature (e.g., add new visualization):**
- Primary code: `ui/src/components/{feature-name}/`
- Data transformation: `ui/src/services/{feature-name}/` (if complex)
- State: `ui/src/atoms/{feature-name}.ts` (if needed)
- Tests: `ui/src/components/{feature-name}/*.test.tsx`

**New Component/Module:**
- Reusable UI components: `ui/src/components/ui/` or `ui/src/components/{feature}/`
- Custom hooks: `ui/src/hooks/use{FeatureName}.ts`
- Utilities: `ui/src/lib/{utility-name}.ts`

**Backend feature (e.g., new query endpoint):**
- Query implementation: `domains/query_engine/analyzer/src/{entity}.rs`
- HTTP endpoint: `domains/query_engine/server/src/ui.rs`
- Types: Update `domains/query_engine/ui/src/lib.rs`
- Tests: Co-located with implementation as `#[cfg(test)]` modules

**Utilities:**
- Shared formatting: `ui/src/lib/` or `ui/src/services/`
- Data transformation: `ui/src/services/{domain}/`
- Color/style mappings: `ui/src/services/colors.ts`
- Math/algorithms: `ui/src/lib/{utility}.ts`

## Special Directories

**`ui/src/routes/`:**
- Generated automatically by TanStack Router from file names
- Do NOT edit `routeTree.gen.ts` manually (auto-generated)
- Create new routes by adding `*.tsx` files matching pattern: `path.segment.$param.tsx`
- Example: `profile.engine.$engineId.query.$queryId.tsx` → `/profile/engine/:engineId/query/:queryId`

**`ui/.tanstack/router.config.json`:**
- Generated: Yes
- Committed: Yes (tracks router generation config)
- Purpose: TanStack Router configuration

**`ui/src/test/mocks/`:**
- Purpose: MSW (Mock Service Worker) handlers for testing
- Pattern: Define handlers for API endpoints being tested
- Generated: No

**`examples/simulator/server/ts-bindings/`:**
- Generated: Yes
- Committed: Yes
- Purpose: TypeScript bindings auto-generated from Rust types via ts-rs
- Pattern: Types are generated during Rust build, not manually edited

**`crates/server/ts-bindings/`:**
- Generated: Yes
- Purpose: Generic server bindings
- Pattern: Similar to simulator bindings

**`target/`:**
- Generated: Yes (Rust build artifacts)
- Committed: No

**`dist/`, `ui/dist/`:**
- Generated: Yes (Vite build output)
- Committed: No

**`.planning/codebase/`:**
- Purpose: GSD codebase analysis documents
- Files: ARCHITECTURE.md, STRUCTURE.md, CONVENTIONS.md, TESTING.md, CONCERNS.md

---

*Structure analysis: 2026-04-01*
