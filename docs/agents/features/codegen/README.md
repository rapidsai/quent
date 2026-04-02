# Codegen: Generating Instrumentation APIs from Model Definitions

This document describes the design for generating language-specific
instrumentation APIs from model definitions written in Rust. The goal is to
eliminate the boilerplate required to set up an instrumentation library based
on Quent's modeling primitives.

## Problem

Defining a single FSM previously required ~480 lines of hand-written code
spread across 6 layers: event type enums, observer emission methods, analyzer
builders, FSM trait impls, type declarations, and usage impls. This boilerplate
had to be kept in sync manually across the Rust instrumentation, analyzer, and
C++ bindings (via CXX bridge).

## Approach

Model definitions (FSMs, Entities, Resources) are written in Rust using derive
macros. From these definitions:

1. The derive macros generate the Rust instrumentation API (typed transition
   methods, event types, trait impls, observer types, analyzer data structs) and
   collect structural metadata via trait impls.
2. A Rust codegen binary imports the model crate, reads the metadata from the
   trait impls, and emits language-specific code (CXX bridge modules for C++,
   etc.).

There is no intermediate representation file. The Rust types, enriched by
macro-generated trait impls, are the single source of truth.

## Design decisions

The following decisions were made during the design of this system. Each is
documented in its own file with context and rationale.

**Source of truth and definition approach:**

- [Rust as source of truth](./decisions/rust-source-of-truth.md)
- [No intermediate representation](./decisions/no-ir.md)
- [No standalone DSL](./decisions/no-standalone-dsl.md)
- [Separate items over module-level macro](./decisions/separate-items.md)
- [Cross-model validation via Rust's type system](./decisions/cross-model-validation.md)
- [Derive-style macro syntax](./decisions/unified-attribute-syntax.md)

**Model structure:**

- [Resource group hierarchy expression](./decisions/resource-group-hierarchy.md)
- [Transition attributes are per-state](./decisions/per-state-attributes.md)
- [Typed entity references](./decisions/typed-entity-references.md)
- [Resources are standard library FSM definitions](./decisions/resources-as-fsms.md)
- [Entities emit freestanding one-shot events](./decisions/entity-events.md)

**Event emission and runtime behavior:**

- [Deferred attributes via amendments with sequence numbers](./decisions/deferred-attributes.md)
- [Common FsmEvent wrapper with sequence numbers](./decisions/fsm-event-structure.md)
- [Flat deferred event enum](./decisions/deferred-event-shape.md)
- [One handle per FSM instance](./decisions/fsm-instantiation.md)
- [State handle borrows FSM handle via &mut](./decisions/state-handle-borrowing.md)
- [Auto-emit exit on drop](./decisions/fsm-drop-behavior.md)
- [Auto-generated top-level event enum](./decisions/event-enum-composition.md)

**Code generation:**

- [Proc macro generates all boilerplate layers](./decisions/generate-all-layers.md)
- [Model collection via type alias and tuple composition](./decisions/model-collection.md)
- [CXX bridge for C++ FFI, no separate C++ runtime](./decisions/cxx-bridge-ffi.md)
- [Codegen backend configuration](./decisions/codegen-backend-config.md)
- [C++ CMake integration](./decisions/cpp-cmake-integration.md)

**Implementation:**

- [Implementation order](./decisions/implementation-order.md)
- [Remaining work](./decisions/remaining-work.md)

## Model definition in Rust

### States

FSM states are declared with `#[derive(State)]`. Fields carry annotations for
usages, deferred attributes, capacity values, and instance names.

```rust
#[derive(Debug, Clone, State, serde::Serialize, serde::Deserialize)]
pub struct Computing {
    #[usage]
    pub thread: Usage<ProcessorResource>,
    #[usage]
    pub memory: Usage<MemoryResource>,
    #[deferred]
    pub rows_processed: Option<u64>,
}
```

### FSMs

FSMs use `#[derive(Fsm)]` with states as struct fields. Transitions are
declared with `#[entry]` and `#[to(...)]` annotations on the fields.

```rust
#[derive(Fsm)]
pub struct Task {
    #[entry]
    #[to(Allocating)]
    queueing: Queueing,
    #[to(Computing, Loading)]
    allocating: Allocating,
    #[to(Computing)]
    loading: Loading,
    #[to(Sending, Spilling, exit)]
    computing: Computing,
    #[to(Allocating)]
    spilling: Spilling,
    #[to(Queueing)]
    sending: Sending,
}
```

The `Fsm` derive validates the transition graph: all states reachable from
entry, every state can reach exit, no transitions out of exit.

### Entities

Entities use `#[derive(Entity)]` with events as struct fields annotated with
`#[event]`. Each event type appears at most once per entity instance.

```rust
#[derive(Entity)]
#[resource_group]
pub struct Operator {
    #[event]
    declaration: Declaration,
    #[event]
    statistics: Statistics,
}

pub struct Declaration {
    pub plan_id: Uuid,
    pub instance_name: String,
    pub custom_attributes: Vec<Attribute>,
}

pub struct Statistics {
    pub custom_attributes: Vec<Attribute>,
}
```

The derive generates:
- `OperatorEvent` enum with one variant per event type
- `OperatorObserver<E>` with one method per event (named after the field)
- `OperatorData` struct with `Option<T>` per event for analyzer use
- `HasEventType`, `EntityData`, `ModelComponent` trait impls
- `From` impls for each event type into the event enum

### Resource groups

`#[resource_group]` is an outer attribute detected by the `Entity` and `Fsm`
derives. Use `#[resource_group(root)]` for the root resource group.

```rust
#[derive(Entity)]
#[resource_group(root)]
pub struct Engine {
    #[event]
    init: Init,
    #[event]
    exit: Exit,
}
```

Parent-child relationships are established at runtime from event data, not at
the type level.

### Resources

Resources are predefined FSMs from the standard library (`quent-stdlib`).
Application code references them in `Usage<T>` fields:

```rust
use quent_stdlib::{MemoryResource, ProcessorResource, ChannelResource};

#[derive(Debug, Clone, State, serde::Serialize, serde::Deserialize)]
pub struct Computing {
    #[usage]
    pub thread: Usage<ProcessorResource>,
    #[usage]
    pub memory: Usage<MemoryResource>,
}
```

### Resource usages

State fields annotated with `#[usage]` link an FSM state to a resource.
`Usage<T>` is a generic struct that requires `T: Resource`:

```rust
pub struct Usage<T: Resource> {
    pub resource_id: Ref<T>,
    pub capacity: T::CapacityValue,
}
```

### Model composition

Models are composed using the `define_model!` macro, which generates the model
type alias, top-level event enum, and `From` impls:

```rust
quent_model::define_model! {
    pub QueryEngineModelDef(QueryEngineEvent) {
        Query: query::Query,
        Engine: engine::Engine,
        Worker: worker::Worker,
        QueryGroup: query_group::QueryGroup,
        Plan: plan::Plan,
        Operator: operator::Operator,
        Port: port::Port,
    }
}
```

Domain models can be nested in application models by listing them as a
component.

### Non-model event types

For event types that are not model components (e.g., trace events), use the
`extra` block:

```rust
quent_model::define_model! {
    Simulator {
        task::Task,
        quent_stdlib::Memory,
    }
    extra {
        Trace: quent_events::trace::TraceEvent,
    }
}
```

Extra variants are included in the event enum with `From` impls but do not
contribute to the `Model<T>` type alias.

### Context generation

The `define_context!` macro generates the instrumentation context:

```rust
quent_model::define_context!(pub SimulatorContext(SimulatorEvent));
```

This generates a struct with `try_new()` and `events_sender()`. Application
code extends it with additional observer factories via impl blocks.

## FSM instantiation and event emission

Each FSM instance is created via `{Name}Handle::new()`, returning an owned
handle with auto-generated UUIDv7, sequence counter, and event sender. Events
are emitted immediately on transition.

```rust
let tx = context.events_sender();
let mut task = TaskHandle::new(&tx, Queueing { ... });
task.transition(Computing { ... });
task.exit(); // or auto-exits on Drop
```

Entity events are emitted via generated observers:

```rust
let obs = engine::EngineObserver::new(&tx);
obs.init(id, Init { ... });
obs.exit(id, Exit);
```

## Analysis

### AnalyzedFsm<T>

Generic FSM reconstruction from events. Implements `Entity`, `Fsm`,
`FsmUsages`, `Using`, `FsmTypeDeclaration` for any `T: TransitionInfo`.

```rust
pub type Task = AnalyzedFsm<TaskTransition>;
pub type TaskBuilder = AnalyzedFsmBuilder<TaskTransition, TaskDeferred>;
```

### AnalyzedEntity<M>

Generic entity reconstruction from events. Stores one `Option<T>` per event
type, populated by `push()`.

```rust
pub struct Engine(AnalyzedEntity<engine::Engine>);
```

Access fields via `self.0.data().init`, `self.0.data().exit`, etc.

## Derive macro summary

| Derive | Helper attributes | Purpose |
|--------|-------------------|---------|
| `State` | `#[usage]`, `#[deferred]`, `#[capacity]`, `#[instance_name]` | FSM state struct |
| `Fsm` | `#[entry]`, `#[to(...)]`, `#[resource(...)]`, `#[resource_group]` | FSM with states as fields |
| `Entity` | `#[event]`, `#[resource_group]`, `#[resource_group(root)]` | Entity with events as fields |
| `ResourceGroup` | `#[resource_group]`, `#[resource_group(root)]` | Standalone resource group |

## C++ code generation

### quent-codegen crate

The `quent-codegen` library takes a `ModelBuilder` and emits CXX bridge Rust
source files. It is called from the instrumentation crate's `build.rs`.

```rust
// build.rs
use quent_codegen::CxxOptions;
use quent_model::ModelComponent;

fn main() {
    let mut builder = quent_model::ModelBuilder::new();
    my_model::Job::collect(&mut builder);
    my_model::Task::collect(&mut builder);

    let options = CxxOptions {
        namespace: "telemetry".to_string(),
        crate_name: "my-instrumentation".to_string(),
        bridge_path: "src/bridge".to_string(),
        model_crate: "my_model".to_string(),
        event_type: "my_model::MyEvent".to_string(),
    };
    let files = quent_codegen::emit_cxx(&builder, &options);

    // Write to src/bridge/, run cxx_build
    for file in &files {
        std::fs::write(format!("src/bridge/{}", file.name), &file.content).unwrap();
    }
    cxx_build::bridges(/* bridge files */).compile("my_instrumentation");
}
```

### Generated modules

The codegen produces one Rust file per model component:

- **`uuid.rs`** — shared UUID type (high_bits/low_bits) with bidirectional
  conversion to `uuid::Uuid`
- **`context.rs`** — instrumentation context with global `EventSender` via
  `OnceLock`. `create_context()` initializes the exporter and sender.
- **Entity bridges** (e.g., `job.rs`) — shared event structs + opaque observer
  type with one method per event. Methods convert FFI types to model types and
  push through the `From` chain.
- **FSM bridges** (e.g., `task.rs`) — shared state structs + opaque handle
  type wrapping the model-generated `{Name}Handle<E>`. Factory function creates
  the handle, transition methods convert FFI types and delegate.
- **`lib.rs`** — module declarations

### Global sender pattern

CXX does not allow sharing opaque Rust types across bridge module boundaries.
The context module uses a `OnceLock<EventSender<E>>` static, set during
`create_context()`. All observer and handle factory functions access it via
`global_sender()` without needing a context reference passed from C++.

### CXX header propagation

`cxx_build` generates C++ headers in `OUT_DIR/cxxbridge/include/`. The
`build.rs` copies these to a stable `include/` directory within the
instrumentation crate. CMake references this directory:

```cmake
target_include_directories(example PRIVATE
    ${CMAKE_SOURCE_DIR}/../instrumentation/include
)
```

### C++ usage

```cpp
#include "my-instrumentation/src/bridge/context.rs.h"
#include "my-instrumentation/src/bridge/job.rs.h"
#include "my-instrumentation/src/bridge/task.rs.h"

int main() {
    auto ctx = telemetry::create_context("ndjson", "data");

    // Entity observer
    auto job_obs = telemetry::job::create_observer();
    auto job_id = uuid::now_v7();
    job_obs->submit(job_id, telemetry::job::Submit{.name = "batch", .num_tasks = 4});
    job_obs->complete(job_id);

    // FSM handle
    auto task = telemetry::task::create(telemetry::task::Queued{
        .job_id = job_id, .name = "task-0",
    });
    task->running(telemetry::task::Running{.thread_resource_id = thread_id});
    task->exit();
}
```

### CMake integration

```cmake
corrosion_import_crate(
    MANIFEST_PATH ${CMAKE_SOURCE_DIR}/../instrumentation/Cargo.toml
    CRATES my-instrumentation
)
target_link_libraries(example PRIVATE my_instrumentation)
target_include_directories(example PRIVATE
    ${CMAKE_SOURCE_DIR}/../instrumentation/include
)
```

Corrosion builds the Rust crate (including CXX bridge compilation),
`cxx_build` generates C++ headers, and `build.rs` copies them to `include/`.

## Crate layout

```
quent-model/            derive macros + core types (Ref<T>, Usage<T>, traits)
quent-model-macros/     proc macro implementations
quent-stdlib/           standard library FSMs (Memory, Processor, Channel)
quent-codegen/          CXX bridge code generator

domains/
  query_engine/
    model/              query engine domain model definitions
    events/             re-export facade over model crate

examples/
  cpp-integration/
    model/              example model (Job, ThreadPool, Task)
    instrumentation/    CXX bridges (generated by build.rs)
    cpp/                C++ application + CMakeLists.txt
  simulator/
    model/              simulator application model (Task FSM)
    events/             SimulatorEvent composition
    instrumentation/    generated context + resource observers
```

## Status

Core framework implemented and validated:
- Derive macros for model definition (State, Fsm, Entity, ResourceGroup)
- Rust instrumentation API generation (handles, observers, events)
- Rust analysis infrastructure (AnalyzedFsm, AnalyzedEntity)
- Simulator and query engine domain fully converted
- C++ code generation via CXX bridge (quent-codegen)
- End-to-end C++ integration example (model → codegen → CXX → CMake → C++ app)
