# External Integrations

**Analysis Date:** 2026-03-25

## APIs & External Services

**Collector Service (gRPC):**
- Service: Quent Collector
  - What it's used for: Event collection and ingestion from instrumented applications
  - SDK/Client: `quent-collector-client` crate (`quent-collector-proto`)
  - Protocol: gRPC with bidirectional streaming (CollectEvents RPC)
  - Authentication: Metadata-based via "engine-id" header (UUID string)
  - Max buffer: 4 MiB per request (3.75 MiB usable, 256 KiB overhead)
  - Batching: Events buffered up to 128ms or buffer full before dispatch

**REST API (HTTP via Axum):**
- Endpoint: `/api/engines/*` - Query engine analysis endpoints
  - What it's used for: Frontend queries for engine list, query execution plans, timelines, resource data
  - Framework: Axum 0.8.7
  - Features: Swagger/OpenAPI documentation (optional)
  - Base URL configurable: `VITE_API_BASE_URL` environment variable

**Frontend API Client:**
- Location: `ui/src/services/api.ts`
- Communication method: Fetch API (native browser HTTP)
- BigInt support: Custom JSON parser for large integers (numbers > Number.MAX_SAFE_INTEGER)
- API stubs for: QueryBundle, QueryGroup, Query, BulkTimelines, SingleTimeline, QueryFilter, TaskFilter

## Data Storage

**Databases:**
- Not applicable - No persistent database configured
- In-memory only (development/testing scenario)

**In-Memory Cache:**
- moka 0.12.13 - Concurrent cache for:
  - Analyzer results (AnalyzerCache in `domains/query_engine/server/src/analyzer_cache.rs`)
  - Timeline data (TimelineCache in `domains/query_engine/server/src/timeline_cache.rs`)
- Stale time: 5 minutes (DEFAULT_STALE_TIME on frontend)

**File Storage:**
- Local filesystem only
- Exporters support:
  - NDJSON format (newline-delimited JSON) - `quent-exporter-ndjson`
  - MessagePack binary format - `quent-exporter-msgpack`
  - Postcard binary format - `quent-exporter-postcard`
  - Collector push (to gRPC collector) - `quent-exporter-collector`
- Default features: All exporters enabled via cargo feature flags

**Caching:**
- Moka cache (async-aware, distributed-friendly)
- No Redis or external cache service

## Authentication & Identity

**Auth Provider:**
- Custom/None - No centralized auth provider configured
- gRPC Metadata: Engine ID passed via metadata header `engine-id` (UUID v7)
- CORS handling: Optional CORS layer configured in Axum router
  - Configurable via environment variable
  - Methods: GET, POST, OPTIONS
  - Headers: Content-Type

**Session Management:**
- Frontend: TanStack Query manages request state
- Backend: Stateless HTTP (per-request context)

## Monitoring & Observability

**Error Tracking:**
- Not configured - No external error tracking service

**Logs:**
- Tracing framework (structured logging)
- Tracing-subscriber with environment filtering
- Output: stderr (default)
- Filtering: RUST_LOG environment variable (format: `warn,quent=debug`)
- Span events: Logged on span close
- Excluded noisy targets: `h2=off`, `tonic=off`
- Frontend: console logging (console.warn/error only via ESLint enforcement)

**Metrics:**
- No external metrics service configured
- Implicit metrics via analysis results (timeline data, operator statistics)

## CI/CD & Deployment

**Hosting:**
- Not externally deployed - Self-hosted
- Deployable as: Standalone Rust binary or Docker container

**CI Pipeline:**
- GitHub Actions (not fully inspected, but git status suggests project management)
- Frontend CI check script: `pnpm ci:check`
  - Routes generation (`tsr generate`)
  - Linting (`eslint`)
  - Format checking (`prettier`)
  - Type checking (`tsc`)
  - Tests (`vitest run`)
  - Audit (`npm audit`)

**Build System:**
- Cargo (Rust workspaces)
- Vite (TypeScript/React)
- Custom build.rs script integrates UI build into binary

## Environment Configuration

**Required env vars:**
- `RUST_LOG` - Controls logging level (optional, defaults to "info")
- `VITE_API_BASE_URL` - Frontend API endpoint (optional, defaults to `/api` for dev proxy)
- Application-specific analyzer configuration (injected at runtime, not environment variables)

**Optional env vars:**
- `CARGO_FEATURE_UI` - Build-time flag to enable UI embedding (automatically set by cargo)
- `VITE_API_TARGET` - Dev server API proxy target (defaults to `http://localhost:8080`)

**Secrets location:**
- Not applicable - No secrets required for core functionality
- Environment variables stored in: `.env` files (local development, not committed)
- No API keys, credentials, or tokens detected in configuration

## Webhooks & Callbacks

**Incoming:**
- Not detected - No webhook endpoints

**Outgoing:**
- Not detected - No external service callbacks

## Frontend Data Fetching

**Query Client Configuration:**
- TanStack Query (React Query) 5.90.21
- Default stale time: 5 minutes (300,000 ms)
- Window focus refetch: Disabled
- Devtools: @tanstack/react-query-devtools for debugging

**Mock Service Worker (MSW):**
- MSW 2.12.10 for HTTP request interception in tests
- Setup file: `ui/src/test/setup.ts`
- Mock handlers: `ui/src/test/mocks/handlers.ts`
- Mock server: `ui/src/test/mocks/server.ts`

## Type Generation & Bindings

**TypeScript Generation from Rust:**
- ts-rs 11.1.0 generates TypeScript types from Rust structs
- Generated bindings located: `examples/simulator/server/ts-bindings/*`
- Build command: `tsr generate` (TanStack Router integration)
- Path alias: `~quent/types/*` points to generated bindings
- Used for type-safe API communication (QueryBundle, Query, QueryGroup, Engine, etc.)

## Development Server Configuration

**Vite Dev Server Proxy:**
- Proxies `/api/*` requests to backend (default: `http://localhost:8080`)
- Configurable via `VITE_API_TARGET` environment variable
- Removes CORS headers from backend responses (proxy handles CORS)
- Follows redirects, insecure SSL allowed for local dev

**Build Configuration:**
- Code splitting for better caching:
  - `react-vendor` chunk: React + React DOM
  - `tanstack` chunk: React Query + Router
  - `xyflow` chunk: XYFlow graph library
  - `echarts` chunk: ECharts visualization
  - Manual chunk configuration prevents large combined bundles
- Rollup visualizer: Generates `stats.html` after build for bundle analysis
- Fetch priority plugin: Prioritizes JS chunk loading over API requests

---

*Integration audit: 2026-03-25*
