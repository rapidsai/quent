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

## Crate layout

```
quent-model/            derive macros + core types (Ref<T>, Usage<T>, traits)
quent-model-macros/     proc macro implementations
quent-stdlib/           standard library FSMs (Memory, Processor, Channel)

domains/
  query_engine/
    model/              query engine domain model definitions
    events/             re-export facade over model crate

examples/
  simulator/
    model/              simulator application model (Task FSM)
    events/             SimulatorEvent composition
    instrumentation/    generated context + resource observers
```

## Status

Core framework implemented and validated on the simulator example and query
engine domain. C++ code generation (CXX bridge backend) not yet implemented.
