# Technology Stack

**Analysis Date:** 2026-03-25

## Languages

**Primary:**
- Rust 1.93+ - Compiled backend services, core analysis, instrumentation, and event processing
- TypeScript 5.9.3 - React UI frontend and type-safe routing

**Secondary:**
- Protocol Buffers (Proto3) - Service definitions for gRPC communication
- HTML/CSS - UI templates and styling

## Runtime

**Environment:**
- Tokio 1.48.0 - Async runtime for Rust services with multi-threaded executor
- Node.js 24.11.0 - JavaScript/TypeScript runtime for UI development and execution

**Package Manager:**
- Cargo - Rust package manager (workspace-based multi-crate project)
- pnpm 9.15.0+ - JavaScript package manager (lockfile-based dependency management)
- Lockfile: `Cargo.lock` (workspace) and `pnpm-lock.yaml` (UI)

## Frameworks

**Backend/Core:**
- Axum 0.8.7 - HTTP server framework (REST API)
- Tonic 0.14.2 - gRPC framework for service-to-service communication
- Tower HTTP 0.6 - HTTP utilities including CORS middleware

**Frontend:**
- React 19.2.4 - UI library for interactive dashboard
- TanStack Router 1.166.2 - Type-safe, file-based routing
- TanStack Query (React Query) 5.90.21 - Server state management and data fetching
- Vite 7.3.1 - Modern bundler and dev server
- Tailwind CSS 4.2.1 - Utility-first CSS framework

**Visualization:**
- ECharts 5.6.0 - Data visualization library for charting and graphs
- echarts-for-react 3.0.6 - React wrapper for ECharts
- XYFlow (xyflow) 12.10.1 - DAG/graph visualization for query plans
- ELK.js 0.11.1 - Automatic graph layout algorithm (bundled version for web)

**UI Components:**
- Radix UI - Headless component library (accordion, dropdown, select, navigation, popover, etc.)
- shadcn/ui - Pre-styled components built on Radix UI and Tailwind
- Lucide React 0.564.0 - Icon library

**Serialization:**
- Serde 1.0.228 - Serialization/deserialization framework
- serde_json 1.0.145 - JSON support
- Prost 0.14.1 - Protocol Buffer serialization
- rmp-serde 1 - MessagePack serialization for compact binary format
- Postcard 1 - Compact, self-describing binary format
- Ciborium 0.2.2 - CBOR (Concise Binary Object Representation) serialization

**Utilities:**
- Async-trait 0.1.89 - Async trait support
- Tokio-stream 0.1.17 - Stream utilities for async iteration
- Tokio-util 0.7.18 - Additional Tokio utilities
- Uuid 1.18.1 - UUID generation (v7 variant for sortable IDs)
- rustc-hash 2 - Fast hash function for Rust
- SmallVec 1.15.1 - Vector with inline storage for optimization
- DashMap 6.1.0 - Concurrent hash map
- Moka 0.12.13 - High-performance cache library
- Log 0.4.28 - Logging facade with key-value support
- Tracing 0.1.43 - Structured logging and diagnostics
- Tracing-subscriber 0.3 - Log filtering and formatting

**Error Handling:**
- thiserror 2.0.17 - Error type derivation macro

**Type Generation:**
- ts-rs 11.1.0 - Automatic TypeScript type generation from Rust types

## Testing Frameworks

**Frontend:**
- Vitest 4.0.18 - Unit testing framework (Vite-native)
- @testing-library/react 16.3.2 - React component testing utilities
- @testing-library/user-event 14.6.1 - User interaction simulation
- @testing-library/jest-dom 6.9.1 - DOM assertions
- jsdom 27.4.0 - DOM implementation for Node.js
- MSW (Mock Service Worker) 2.12.10 - HTTP request mocking at network layer
- Vitest Coverage v8 - Code coverage reporting

**Build/Linting:**
- ESLint 9.39.3 - JavaScript/TypeScript linting
- Prettier 3.8.1 - Code formatting
- TypeScript compiler (tsc) - Type checking
- TypeScript ESLint - TypeScript support for ESLint

## Key Dependencies

**Critical:**
- `tonic` 0.14.2 - Core infrastructure for gRPC services and client-server communication
- `tokio` 1.48.0 - Async runtime enabling non-blocking I/O across all services
- `axum` 0.8.7 - REST API server framework
- `react` 19.2.4 - UI rendering engine
- `@tanstack/react-router` 1.166.2 - Type-safe routing system for SPA navigation
- `@tanstack/react-query` 5.90.21 - Server state management and automatic caching

**Infrastructure:**
- `moka` 0.12.13 - Distributed cache for analyzer and timeline results (caching layer)
- `@xyflow/react` 12.10.1 - DAG visualization for query execution plans
- `echarts` 5.6.0 - Time series and performance metric visualization
- `tailwindcss` 4.2.1 - CSS generation and styling system

## Configuration

**Environment:**
- Runtime configuration via environment variables:
  - `VITE_API_BASE_URL` - Frontend API endpoint override (defaults to `/api` for proxying)
  - `RUST_LOG` - Logging level control (integrates with tracing-subscriber)
  - Analyzer/importer configuration is application-specific

**Build:**
- `build.rs` (domains/query_engine/server/) - Custom build script that:
  - Integrates pnpm build process for UI when `ui` feature is enabled
  - Triggers React/Vite build during Rust compilation
  - Embeds built UI assets into binary via `rust-embed` feature

- Cargo features:
  - `ui` - Enables UI embedding and serving from Rust binary
  - `swagger` - Enables Swagger/OpenAPI documentation (utoipa + utoipa-swagger-ui)
  - Exporter features: `ndjson`, `msgpack`, `postcard`, `collector` (all enabled by default)

**Workspace Configuration:**
- Rust edition 2024 (latest stable)
- Workspace resolver v3
- 19 workspace members organized by concern (crates/, domains/, examples/)

## Platform Requirements

**Development:**
- Rust 1.93+
- Node.js 24.11.0 (enforced via volta/nvm)
- pnpm 9.0.0+ (enforced via preinstall script)
- libprotobuf 5+ (for proto compilation)
- Platform support: Linux (x86_64, aarch64), macOS (x86_64, arm64)

**Production:**
- No database required (in-memory caching with moka)
- No external service dependencies detected
- Deployable as:
  - Standalone Rust binary (via `cargo build --release`)
  - Docker container (multi-stage Rust + Node.js build)
- Requires gRPC capability for collector endpoints
- HTTP/REST API served by Axum (configurable CORS)

## Documentation & OpenAPI

**Optional Swagger UI:**
- Available via `swagger` feature flag
- Served at `/swagger-ui`
- OpenAPI spec at `/api-docs/openapi.json`
- Generated with utoipa and utoipa-swagger-ui

---

*Stack analysis: 2026-03-25*
