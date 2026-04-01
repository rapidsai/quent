# External Integrations

**Analysis Date:** 2026-04-01

## APIs & External Services

**None detected** - Quent does not depend on external cloud APIs, SaaS platforms, or third-party web services. The system is self-contained.

## Data Storage

**Databases:**
- Not used - Quent does not use persistent database systems (SQL or NoSQL)
- All data lives in-memory during analysis and collection phases

**File Storage:**
- Local filesystem only - Data directory mounted at `/quent/data` in Docker (see docker-compose.yml)
- File format support: CBOR, MessagePack (MsgPack), Postcard, and NDJSON via exporter plugins

**Caching:**
- **In-Memory Cache**: Moka 0.12.13 (async-aware, with TTL support) in `domains/query_engine/server/src/analyzer_cache.rs` and `timeline_cache.rs`
- No external cache service (Redis, Memcached)

## Authentication & Identity

**Auth Provider:**
- Custom or None - No built-in authentication detected in the codebase
- HTTP API uses CORS for cross-origin requests (configured via `QUENT_ANALYZER_CORS_ADDRESS`)
- No API key, JWT, or OAuth implementation in core services

## Monitoring & Observability

**Error Tracking:**
- None detected - No integration with Sentry, DataDog, or similar services

**Logs:**
- **Structured Logging**: Tracing crate with configurable filters
  - CLI flag: `--log-level` (e.g., `debug`, `info`, `warn`, `error`)
  - Environment: `RUST_LOG` for tracing-subscriber filters
  - Output: Formatted to stderr with timestamps and module names (see simulator server)

**Distributed Tracing:**
- None detected - No OpenTelemetry or distributed tracing integration

## CI/CD & Deployment

**Hosting:**
- Docker-based deployment (see Dockerfile and docker-compose.yml)
- Multi-stage build: Rust 1.91 builder image, minimal Debian trixie runtime
- Compiled binaries: `quent-simulator-server`, `quent-simulator`
- Served via: Axum HTTP server (REST API) and Tonic gRPC server

**CI Pipeline:**
- GitHub Actions (see .github/ directory structure)
- No documented CI configuration in provided files

## Environment Configuration

**Required env vars:**
- `QUENT_ANALYZER_CORS_ADDRESS` - CORS origin for analyzer HTTP API (default: unset, use flag)
- `QUENT_COLLECTOR_ADDRESS` - gRPC collector endpoint for simulators (default: unset, use flag)

**Optional env vars:**
- `RUST_LOG` - Tracing filter level (controls logging verbosity)
- `VITE_API_TARGET` - Frontend dev proxy target (default: `http://localhost:8080`)
- `VITE_API_BASE_URL` - Frontend test API endpoint (default: `/api`)

**Secrets location:**
- Not applicable - No secrets management integration detected
- No .env file pattern used (enforced by .gitignore)

## Webhooks & Callbacks

**Incoming:**
- None detected - Quent does not expose webhook endpoints

**Outgoing:**
- None detected - Quent does not call external webhooks or callbacks

## Communication Protocols

**HTTP/REST:**
- Backend: Axum (port 8080 in Docker)
- Endpoints exposed via REST API (see vite.config.ts proxy configuration)
- Request format: JSON with BigInt support
- Response format: JSON with structured data types

**gRPC:**
- Service: `Collector` (defined in `proto/quent/collector/v1/collector.proto`)
- Method: `CollectEvents(stream CollectEventRequest) returns (CollectEventResponse)`
- Port: 7836 (see docker-compose.yml and Dockerfile)
- Use case: Event streaming from instrumented applications to collector server
- Implementation: Tonic 0.14.2 with Prost code generation
- Protocol: Protocol Buffers v3 with binary serialization

**Serialization Formats:**
- Protocol Buffers - gRPC service communication
- JSON - HTTP REST API and TypeScript bindings generation
- CBOR - Optional event export format (ciborium)
- MessagePack - Optional event export format (rmp-serde)
- Postcard - Optional event export format (compact binary)
- NDJSON - Optional event export format (newline-delimited JSON)

## API Specification

**REST API (Analyzer/Query Engine):**

Frontend accesses via reverse proxy at `/api` (see `ui/src/services/api.ts`):

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/engines` | GET | List available query engines with metadata |
| `/engines/{engineId}/query-groups` | GET | List query groups (coordinators) in an engine |
| `/engines/{engineId}/query_group/{coordinatorId}/queries` | GET | List queries in a query group |
| `/engines/{engineId}/query/{queryId}` | GET | Fetch full query bundle with DAG and metadata |
| `/engines/{engineId}/timeline/single` | POST | Generate single timeline for query/task filters |
| `/engines/{engineId}/timeline/bulk` | POST | Generate multiple timelines in bulk |

**Request Format:**
- Query params: `duration` (seconds), `with_metadata` (boolean)
- Body: JSON with filter objects (QueryFilter, TaskFilter)
- Content-Type: `application/json`

**Response Format:**
- JSON with BigInt support (handled by `parseJsonWithBigInt()` in `ui/src/services/api.ts`)
- Types auto-generated to TypeScript via `ts-rs` (Rust to TS bindings)

## Type Sharing

**Rust to TypeScript Bindings:**
- Tool: `ts-rs` 11.1.0 with workspace features
- Direction: Rust types → TypeScript .d.ts files via `#[derive(Serialize, Deserialize)]` + `#[ts(...)]`
- Location: Types imported from `~quent/types/*` alias (resolves to `examples/simulator/server/ts-bindings`)
- Used for: QueryBundle, QueryGroup, Query, BulkTimelinesResponse, Engine, QueryFilter, TaskFilter, EntityRef
- Build step: `tsr generate` (TypeScript Router CLI, part of Vite build)

## Testing & Mocking

**API Mocking:**
- MSW 2.12.10 (Mock Service Worker) in frontend tests
- Setup: `ui/src/test/setup.ts` with `beforeAll()`, `afterEach()`, `afterAll()` hooks
- Handlers: `ui/src/test/mocks/handlers.ts` (API response stubs)
- Environment: `VITE_API_BASE_URL` set to `http://localhost:8000/api` in test config

---

*Integration audit: 2026-04-01*
