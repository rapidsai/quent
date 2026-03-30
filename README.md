# Quent

Quent is a framework for instrumenting and analyzing applications. It provides
a set of modeling concepts (especially Finite State Machines, Resources, and
their relationships) from which a statically typed instrumentation API is
derived. Applications instrumented with this API emit structured telemetry
that can be stored, analyzed, and visualized.

The current focus is on data processing / query engines, but the concepts are
domain-agnostic and may be applied to other domains in the future.

## How

A developer constructs a **model** of their application: FSMs to track entity
lifecycles, Resources to represent things with limited capacity, and Usages to
tie the two together. The model produces a type-safe instrumentation API.

Analysis tools consume the emitted events and, using the structural information
from the model, automatically derive timelines and utilization graphs. For
query engines, this includes plan DAG visualizations and per-operator
breakdowns. Any application-specific analysis is easier to build on top of the
structured model the framework provides.

## Provided by this repository

- **Specification**: the [docs](docs/) directory, defining the modeling
  concepts and a domain-specific model for query engines.
- Rust implementations of:
  - **Instrumentation**: a domain-agnostic library for emitting
    type-safe telemetry from a model.
  - **Exporters**: pluggable telemetry transports.
  - **Analyzers**: domain-agnostic and domain-specific libraries that
    reconstruct in-memory models from collected events, with traits for
    querying FSM states, resource usage, and entity relationships.
  - **Web UI**: a React-based frontend for interactive visualization of query
    plans (DAGs), resource timelines, and operator statistics.
  - **Simulator**: an example application that emits telemetry for a simulated
    query engine, useful for development and demonstration.

The core of the project is the modeling approach: Entities, FSMs, Resources,
Capacities, and Usages, and the logic that connects them (resource utilization
tracking, hierarchical aggregation, and model reconstruction from events).
Everything else (storage, transport, instrumentation, and visualization) is an
opinionated but replaceable implementation based on the modeling approach.

## Status

This project is experimental and under heavy development. The modeling concepts,
APIs, and tooling are heavily opinionated and subject to change. There are no
official releases yet.

## Development

### Prerequisites

- Rust (stable, >= 1.93)
- Node.js (>= 24.11)
- pnpm (>= 10)
- protoc (protobuf compiler)

Or use [pixi](https://pixi.sh) to manage all dependencies:

```bash
pixi shell
```

This installs the required toolchains and drops you into a shell with
everything on `PATH`.

### UI development

The easiest way to get a working backend for UI development is with Docker
Compose:

```bash
docker compose up --build
```

This spawns the simulator server (collector on `:7836`, analyzer HTTP API on
`:8080`) and runs the simulator application, which generates a test dataset
by sending simulated query engine events to the collector.

Then start the Vite dev server:

```bash
cd ui
pnpm install
pnpm dev
```

The dev server starts on <http://localhost:5173> by default.

#### Running the server without Docker

Without the `ui` feature, the server only exposes the analysis API and does not
build and serve the static webpage, so you can use the Vite dev server as
described previously.

```bash
cargo run -p quent-simulator-server -- --cors-address http://localhost:5173
```

To generate a test dataset, run the simulator:

```bash
cargo run -p quent-simulator
```

### Building with the static webpage

With the `ui` feature flag, the server also serves the static webpage, removing
the need for a separate frontend server. This approach can be useful to do some
stress testing on the UI.

```bash
cargo build -p quent-simulator-server --features ui --release
```

This runs `pnpm install && pnpm build` in `ui/` as part of the Cargo build and
bundles the output into the binary.

### Swagger UI

An interactive API explorer is available behind the `swagger` feature flag:

```bash
cargo build -p quent-simulator-server --features ui,swagger --release
```

Then visit <http://localhost:8080/swagger-ui>.
