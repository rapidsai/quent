# Quent

Quent is a framework for building tools that help understand behavior and
resource utilization of abstract data and control flow structures in your
application. It provides a set of modeling concepts (especially Finite State
Machines, Resources, and how they can be related).

From an application model, a statically typed instrumentation API is generated.
Applications instrumented with this API emit structured telemetry that can be
stored, analyzed, and visualized.

Quent provides building blocks for each of these layers, so you (or preferably
your coding agent) can mix and match to build a dedicated, semantically rich
profiling / telemetry tool for your application.

In this experimental stage, the first domain we target is that of query engines,
but the basic concepts are domain-agnostic and may be applied to other domains.

## Status

This project is experimental and under heavy development. The modeling concepts,
generated and non-generated APIs, and implementations are continunously subject
to breaking changes for now. There are no releases. Consider this project
pre-alpha. Expect bugs. At the same time, early experiments are welcome, as well
as thoughts, questions, suggestions, and feature requests.

## Show me the code

An extensive example of using all modeling concepts to define a model and the
resulting instrumentation API is found here:

- [Example](examples/readme/src/lib.rs)

A simulated application (a query engine), analysis back-end and front-end can be
found here:

- [Simulator](examples/simulator/)
- [Analyzer](examples/simulator/analyzer/)
- [Front-end](ui/)

While Quent is a Rust-based project, it can generate a C++ instrumentation API.
This is shown here:

- [C++ Integration Example](examples/cpp-integration/)

## How

A developer constructs a **model** of their application: **FSMs** to track
lifecycles of objects, **Resources** to represent things with limited capacity,
and **Usages** to tie the two together. The model produces a type-safe
instrumentation API through which events are emitted and stored.

Analysis tools consume the emitted events and, using the structural information
from the model, automatically derive timelines and utilization graphs. For
query engines, this includes plan DAG visualizations and per-operator
breakdowns. Application-specific analysis is easy to build on top of the
structured modeling approach that the framework provides.

## Provided by this repository

- **Specification**: the [docs](docs/) directory, defining the modeling
  concepts and a domain-specific model for query engines.
- Rust implementations of:
  - **Instrumentation**: a domain-agnostic library for emitting
    type-safe telemetry from a model.
  - **Exporters**: pluggable telemetry transports.
  - **Analyzers**: components to build application-specific services that
    reconstruct in-memory models from collected events, with traits for querying
    FSM states, resource usage, and entity relationships, besides
    application-custom logic.
- Query engine domain-specific building blocks for the above, and examples:
  - **Web UI**: a React-based frontend for interactive visualization of query
    plans (DAGs), resource timelines, and operator statistics.
  - **Simulator**: an example application that emits telemetry for a simulated
    query engine, useful for development and demonstration.

The core of the project is the modeling approach: Entities, FSMs, Resources,
Capacities, and Usages, and the logic that connects them (resource utilization
tracking, hierarchical aggregation, and model reconstruction from events).
Everything else (storage, transport, instrumentation, and visualization) is an
opinionated but replaceable implementation based on the modeling approach.

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
