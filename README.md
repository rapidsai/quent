# Quent

Quent is a framework for building tools that help understand dynamic behavior
and resource utilization of data and control flow structures in your
application. It provides a set of modeling concepts (especially Finite State
Machines, Resources, and their relationships) from which a statically typed
instrumentation API is generated. Applications instrumented with this API emit
structured telemetry that can be stored, analyzed, and visualized. Quent
provides building blocks for each of these layers, so you can mix and match
things to build a dedicated, semantically rich profiling / telemetry tool for
your application.

In this experimental stage, the first domain we target is that of query engines,
but the basic concepts are domain-agnostic and may be applied to other domains.

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

## Example

### Model

The model describes the structure of things you want to track in your
application:

```rust
use quent_model::prelude::*;

// A "unit resource", only one entity can use this at a time.
#[derive(Resource)]
pub struct Thread;

// A resource with a capacity, multiple entities can use this
// at a time, claiming some of its capacity.
#[derive(Resource)]
pub struct Queue {
    pub depth: Capacity<u64>,
}

// A state of the "Task" FSM
#[derive(State)]
pub struct Queued {
    // At least one state must name the Task, for which
    // the field is annotated by this attribute
    #[instance_name]
    pub name: String,
    // This state uses a queue resource:
    pub queue: Usage<QueueResource>,
}

// Another state of the "Task" FSM
#[derive(State)]
pub struct Running {
    // This state uses a thread resource:
    pub thread: Usage<ThreadResource>,
}

// The "Task" FSM
#[derive(Fsm)]
pub struct Task {
    #[entry] #[to(Running)]
    pub queued: Queued,
    #[to(exit)]
    pub running: Running,
}

// Defines an application model, generates all event types
// for the components of the model defined above.
quent_model::define_model! {
    App { Task, Thread, Queue }
}

// Generates the instrumentation context from which event
// emitting APIs are called.
quent_model::define_instrumentation!(App);
```

### Rust instrumentation

The derive macros generate a type-safe instrumentation API from the model.
See [examples/simulator](examples/simulator/) for a complete example.

```rust
let ctx = AppContext::try_new(exporter, uuid::Uuid::now_v7())?;

// Create a task — enters Queued, occupying a queue slot
let mut task = ctx.task_observer().queued(Queued {
    name: "query-42".into(),
    queue: Usage {
        resource_id: Ref::new(queue_id),
        capacity: QueueOperating { depth: Capacity::new(1) },
    },
});

// Task gets scheduled — releases queue slot, acquires thread
task.running(Running {
    thread: Usage {
        resource_id: Ref::new(thread_id),
        capacity: ThreadOperating {},
    },
});

// Task completes
task.exit();
```

### C++ instrumentation

The same model can target C++ via CXX bridge code generation. Capacity wrappers
and typed references are flattened into plain fields.
See [examples/cpp-integration](examples/cpp-integration/) for a complete example.

```cpp
auto ctx = quent::create_context("ndjson", "data");

auto task = quent::task::create(quent::task::Queued {
    .name = "query-42",
    .queue_resource_id = queue_id,
    .queue_depth = 1,
});

task->running(quent::task::Running {
    .thread_resource_id = thread_id,
});

task->exit();
```

### Event output

The `ndjson` exporter in the above example writes one JSON object per line,
which is typically only useful for debugging and manual inspection. Production
deployments can use the MessagePack or Postcard exporters for lower overhead, or
stream to a centralized collector for distributed deployments, but to illustrate
the events stored, an example of the output is shown below:

```json
{"id":"019d...","timestamp":1712345678000000000,"data":{"Task":{"Transition":{"seq":0,"state":{"Queued":{"name":"query-42","queue":{"resource_id":"01a2...","capacity":{"depth":1}}}}}}}}
{"id":"019d...","timestamp":1712345678000100000,"data":{"Task":{"Transition":{"seq":1,"state":{"Running":{"thread":{"resource_id":"01b3...","capacity":{}}}}}}}}
{"id":"019d...","timestamp":1712345678000200000,"data":{"Task":{"Transition":{"seq":2,"state":"Exit"}}}}
```

### Analysis and visualization

The analyzer reconstructs entity relationships from the exported events in order
to produce timelines of resource utilization, FSM activity, and more. UI
building blocks render these as e.g. interactive timeline views, DAG
visualizations, resource heatmaps, etc.

> TODO: simple examples on how to combine analyzer and UI components

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
