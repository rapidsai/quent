# Technology Stack

**Analysis Date:** 2026-04-01

## Languages

**Primary:**
- **Rust** 1.93+ - Backend services, instrumentation, analysis engines, and exporters
- **TypeScript** 5.9.3 - React UI application with strict type checking
- **Protocol Buffers** 3 (proto3) - Service communication and data serialization

**Secondary:**
- **JavaScript** - Build tooling and configuration files

## Runtime

**Backend:**
- Rust native binaries targeting Linux and macOS (see pixi.toml)
- No external language runtime required

**Frontend:**
- **Node.js** 24.11.0 - Development and build environment
- **Browser** (ES2020+ support) - React 19.2.4 application runtime

**Development Tools:**
- **Pixi** - Environment and dependency manager (pixi.toml)
- **Cargo** - Rust package manager and build system
- **pnpm** 9.15.0+ - Node.js package manager (enforced via volta and preinstall script)

## Frameworks

**Backend Web:**
- **Axum** 0.8.7+ - HTTP server framework with routing, middleware, and async support
- **Tonic** 0.14.2 - gRPC framework built on Tokio
- **Tower-HTTP** 0.6 - CORS middleware for Axum

**Frontend:**
- **React** 19.2.4 - UI component framework
- **TanStack Router** 1.166.2 - File-based routing with type safety
- **TanStack React Query** 5.90.21 - Server state management and data fetching
- **Tailwind CSS** 4.2.1 - Utility-first CSS framework
- **Radix UI** - Headless UI component library (accordion, dropdown, popover, select, etc.)
- **XYFlow** 12.10.1 - Graph/DAG visualization library

**Build & Dev:**
- **Vite** 7.3.1 - Frontend build tool and dev server
- **Vitest** 4.0.18 - Unit/integration test runner for TypeScript/React
- **TypeScript Router CLI (tsr)** - Route code generation from file structure
- **Rollup Visualizer** 6.0.11 - Bundle analysis

**Testing:**
- **Testing Library** (@testing-library/react, @testing-library/jest-dom) - Component testing utilities
- **MSW** (Mock Service Worker) 2.12.10 - API mocking for tests
- **jsdom** 27.4.0 - DOM implementation for test environment

## Key Dependencies

**Backend Critical:**
- **Tokio** 1.48.0 - Async runtime for concurrent operations
- **Serde** 1.0.228 - Serialization/deserialization with derive macros
- **Tracing** 0.1.43 - Structured logging and diagnostics
- **UUID** 1.18.1 (v7 variant) - Unique identifiers throughout the system
- **Tonic-Prost** 0.14.2 - gRPC code generation from proto files

**Backend Infrastructure:**
- **async-trait** 0.1.89 - Async trait method support
- **Tokio-Stream** 0.1.17 - Async stream utilities
- **Tokio-Util** 0.7.18 - Additional Tokio utilities
- **Moka** 0.12.13 - High-performance async caching with expiration
- **Dashmap** 6.1.0 - Concurrent hashmap (used in collector)
- **Ciborium** 0.2.2 - CBOR serialization
- **Postcard** 1.0 - Compact binary serialization
- **RMP-Serde** 1.0 - MessagePack serialization
- **Thiserror** 2.0.17 - Error type derivation
- **ts-rs** 11.1.0 - Generate TypeScript type bindings from Rust types
- **Smallvec** 1.15.1 - Small vector optimization (serde support)
- **Rustc-Hash** 2.0 - Fast hashing for HashMaps

**Frontend Critical:**
- **Echarts** 5.6.0 - Data visualization (charts/timelines)
- **ECharts-for-React** 3.0.6 - React wrapper for echarts
- **Elkjs** 0.11.1 - Graph layout algorithm (ELK implementation)
- **Jotai** 2.18.0 - Primitive atom-based state management
- **Jotai-Family** 1.0.1 - Family atoms for scoped state
- **React-Resizable-Panels** 4.7.1 - Draggable panel layout
- **Lucide-React** 0.564.0 - Icon library
- **Class-Variance-Authority** 0.7.1 - Type-safe component variant definitions
- **Clsx** 2.1.1 - Conditional className utility
- **Tailwind-Merge** 3.5.0 - Merge Tailwind class conflicts
- **Tailwindcss-Animate** 1.0.7 - Keyframe animations for Tailwind

**Frontend Dev:**
- **@vitejs/plugin-react** 5.1.4 - React Fast Refresh support
- **@tailwindcss/vite** 4.2.1 - Tailwind v4 Vite plugin
- **ESLint** 9.39.3 - Code linting with TypeScript support
- **Prettier** 3.8.1 - Code formatting
- **TypeScript-ESLint** 8.56.1 - TypeScript linting rules

## Configuration

**Environment:**
- Development: `VITE_API_TARGET` - Backend API URL (defaults to `http://localhost:8080`)
- Development: `VITE_API_BASE_URL` - Alternative API endpoint for tests (defaults to `/api`)
- Development: `QUENT_COLLECTOR_ADDRESS` - gRPC collector address for simulator
- Development: `QUENT_ANALYZER_CORS_ADDRESS` - CORS origin for analyzer API
- Development: `QUENT_*` environment variables (see examples/simulator/server)

**Build:**
- Backend: `Cargo.toml` (workspace with 12 crates)
- Frontend: `ui/package.json` (pnpm workspace)
- Frontend: `ui/vite.config.ts` - Build optimization, module chunking, path aliases, proxy config
- Frontend: `ui/vitest.config.ts` - Test environment setup (jsdom), coverage config
- Frontend: `ui/tsconfig.json` - TypeScript strict mode configuration
- Frontend: `.eslintrc` config with React hooks rules
- Proto: `proto/quent/collector/v1/collector.proto` - gRPC service definition

**Lockfiles:**
- `Cargo.lock` - Rust dependency lock
- `ui/pnpm-lock.yaml` - npm lockfile (present but not shown)

## Platform Requirements

**Development:**
- **Rust** >= 1.93
- **Node.js** >= 24.11.0 (enforced via .nvmrc, volta, and package.json engines)
- **pnpm** >= 9.0.0 (enforced via preinstall script and volta)
- **libprotobuf** >= 5 (for protobuf compiler)
- **Protobuf compiler** (apt-get install protobuf-compiler in Docker)
- macOS, Linux/x86-64, or Linux/ARM64 (per pixi.toml)

**Production/Deployment:**
- **Docker**: Multi-stage Dockerfile targets Debian trixie (Rust 1.91 builder, Debian runtime)
- Compiled to native executables: `quent-simulator-server` (API on :8080, collector on :7836) and `quent-simulator`
- Static UI assets bundled into binary when `ui` feature flag is enabled
- Optional Swagger UI available with `swagger` feature flag

---

*Stack analysis: 2026-04-01*
