# External Integrations

**Analysis Date:** 2026-07-08

## APIs & External Services

**Internal telemetry gRPC (self-hosted):**
- Quent Collector service - streams instrumentation events from applications to an analysis server
  - Contract: `proto/quent/collector/v1/collector.proto` (`Collector.CollectEvents` client-streaming RPC; per-stream metadata `source-context-id`, `entity-type`)
  - Server: `crates/collector/server/` (Tonic), default port `:7836`
  - Client: `crates/collector/client/` (Tonic + CBOR via ciborium), used by `crates/exporter/collector/`
  - Config: `QUENT_COLLECTOR_ADDRESS` (e.g. `http://server:7836` in `docker-compose.yml`)

**NVIDIA/RAPIDS Benchmarking API (internal, optional):**
- Used by `quent-open` (`crates/open/`) in non-local `db` mode to download a run's telemetry assets (presigned URLs)
  - SDK/Client: reqwest 0.12 (`rustls-tls-native-roots` to trust an internal CA); implementation `crates/open/src/db.rs`
  - Auth: bearer token `QUENT_OPEN_TOKEN`; base URL `QUENT_OPEN_API_BASE_URL` (a `.env` file is loaded first via dotenvy, `crates/open/src/main.rs`)
  - Off by default (Cargo feature `db`); "irrelevant to non-NVIDIA users" per `crates/open/Cargo.toml`

**Quent Analyzer HTTP API (self-hosted):**
- Axum REST API consumed by the React UI under `/api`
  - Servers: `domains/query_engine/server/`, `examples/simulator/server/` (default `:8080`)
  - UI access: Vite dev/preview proxy `/api` -> `VITE_API_TARGET` (default `http://localhost:8080`), `ui/vite.config.ts`; client code in `ui/packages/@quent/client/`
  - Optional Swagger UI at `/swagger-ui` behind the `swagger` Cargo feature (utoipa + utoipa-swagger-ui)

## Data Storage

**Databases:**
- None. Analyzers reconstruct in-memory models from collected event streams (`crates/analyzer/`, `domains/query_engine/analyzer/`). No SQL/NoSQL client dependencies detected.

**File Storage:**
- Local filesystem only. File exporters write event streams to `QUENT_OUTPUT_DIR` (default `events`, `crates/exporter/src/clap.rs`) in NDJSON / MessagePack / Postcard formats (`crates/exporter/{ndjson,msgpack,postcard}/`)
- Docker compose mounts `./data` into the server container (`docker-compose.yml`)
- `quent-open` (`archive` feature) reads telemetry from tar/tar.gz/tar.zst/zip archives

**Caching:**
- In-process only: moka 0.12 async cache in `domains/query_engine/server/`. No external cache (Redis etc.).

## Authentication & Identity

**Auth Provider:**
- None for the collector gRPC and analyzer HTTP APIs (open endpoints; CORS-restricted via `--cors-address` / `QUENT_ANALYZER_CORS_ADDRESS`, tower-http CORS layer)
- Bearer token only for the optional internal Benchmarking API in `quent-open` (`QUENT_OPEN_TOKEN`, hidden from help output, scrubbed from child-process env in `crates/open/src/viewer.rs`)

## Monitoring & Observability

**Error Tracking:**
- None (no Sentry or similar)

**Logs:**
- `tracing` + `tracing-subscriber` (`fmt`, `env-filter`) in servers; `--log-level` CLI flag (see `docker-compose.yml` command)
- `log` 0.4 with `kv` feature available in library crates
- `quent-open` strips URLs from reqwest errors so presigned (credential-bearing) URLs never reach logs (`crates/open/src/db.rs`)

## CI/CD & Deployment

**Hosting:**
- Self-hosted / Docker. Multi-stage `Dockerfile` (rust:1.91-trixie builder -> debian:trixie runtime) exposing `:8080` (HTTP) and `:7836` (gRPC); orchestrated by `docker-compose.yml`
- No releases; project is pre-alpha (`README.md`)

**CI Pipeline:**
- GitHub Actions (`.github/workflows/`):
  - `rust.yml` - fmt, clippy (`-D warnings`), test, release build (workspace, all features, via pixi)
  - `ui.yml` - format check, lint, typecheck, tests (per-job pnpm install via pixi)
  - `cpp.yml`, `python.yml` - language-bridge integration tests
  - `docker-build.yml` - Docker image build
  - `license-check.yml` (cargo-deny/`deny.toml`), `markdown.yml`, `pr-title.yml`, `checks.yml`
- Dependabot for dependency bumps (per recent commit history)

## Environment Configuration

**Required env vars:**
- None strictly required (all have CLI defaults). Notable optional vars:
  - `QUENT_EXPORTER` - exporter kind selection (`crates/exporter/src/clap.rs`)
  - `QUENT_COLLECTOR_ADDRESS` - collector gRPC endpoint
  - `QUENT_OUTPUT_DIR` - file-exporter output directory (default `events`)
  - `QUENT_ANALYZER_CORS_ADDRESS` - allowed CORS origins for the HTTP API
  - `QUENT_OPEN_API_BASE_URL`, `QUENT_OPEN_TOKEN` - `quent-open` db mode (internal NVIDIA)
  - `QUENT_OPEN_ROOT`, `QUENT_OPEN_ADDR` - `quent-open` viewer wrapper (`crates/open/src/wrapper.rs`)
  - `VITE_API_TARGET` - UI dev-server API proxy target
  - `QUENT_BENCH_PROFILE_HZ`, `QUENT_BENCH_PROFILE_TIME` - bench profiling knobs (`crates/instrumentation/benches/event_emit.rs`)

**Secrets location:**
- No committed `.env` files. `quent-open` optionally loads a local `.env` at runtime (dotenvy). Only secret in play is `QUENT_OPEN_TOKEN`.

## Webhooks & Callbacks

**Incoming:**
- None (no webhook endpoints; the collector gRPC stream is the only inbound integration surface)

**Outgoing:**
- None. In-process callback exporter exists (`crates/exporter/callback/`) but it is a Rust API, not a network callback.

---

*Integration audit: 2026-07-08*
