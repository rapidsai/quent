# Codegen: Generating Instrumentation APIs from Model Definitions

This document describes a design for generating language-specific instrumentation
APIs from model definitions written in Rust. The goal is to eliminate the
boilerplate currently required to set up an instrumentation library based on
Quent's modeling primitives.

## Problem

Defining a single FSM today requires ~480 lines of hand-written code spread
across 6 layers: event type enums, observer emission methods, analyzer builders,
FSM trait impls, type declarations, and usage impls. This boilerplate must be
kept in sync manually across the Rust instrumentation, analyzer, and C++
bindings (via CXX bridge).

## Approach

Model definitions (FSMs, Resources, Entities, Events) are written in Rust using
proc macro annotations. From these definitions:

1. The proc macros generate the Rust instrumentation API (typed transition
   methods, event types, trait impls, analyzer builders) and collect structural
   metadata via trait impls.
2. A Rust codegen binary imports the model crate, reads the metadata from the
   trait impls, and emits language-specific code (CXX bridge modules for C++,
   etc.).

There is no intermediate representation file. The Rust types, enriched by proc
macro-generated trait impls, are the single source of truth. The codegen binary
reads them in-process.

## Design decisions

The following decisions were made during the design of this system. Each is
documented in its own file with context and rationale.

**Source of truth and definition approach:**

- [Rust as source of truth](./decisions/rust-source-of-truth.md)
- [No intermediate representation](./decisions/no-ir.md)
- [No standalone DSL](./decisions/no-standalone-dsl.md)
- [Separate items over module-level macro](./decisions/separate-items.md)
- [Cross-model validation via Rust's type system](./decisions/cross-model-validation.md)

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

**Implementation:**

- [Implementation order](./decisions/implementation-order.md)

## Model definition in Rust

### Resources

Resources are ordinary FSMs shipped in a standard library crate
(`quent-stdlib`). Common resource types (Memory, Processor, Channel) are
predefined FSM definitions with the spec-defined lifecycles. Applications use
them directly or alias them. Custom resources are FSMs marked with
`#[quent::resource]`.

```rust
use quent_model::prelude::*;
use quent_stdlib as stdlib;

// Use standard library resource FSMs
pub type WorkerMemory = stdlib::Memory;
pub type Thread = stdlib::Processor;
pub type FsToMem = stdlib::Channel;
pub type MemToFs = stdlib::Channel;

// Custom resource: an FSM marked as a resource
#[quent::fsm]
#[quent::resource]
pub struct GpuSlots {
    #[quent::transition(entry -> Initializing)]
    #[quent::transition(Initializing -> Operating)]
    #[quent::transition(Operating -> Finalizing)]
    #[quent::transition(Finalizing -> exit)]
}

#[quent::state]
pub struct GpuSlotsOperating {
    pub slots: u32,
}
```

`#[quent::resource]` generates a `Resource` trait impl. The capacity type is
derived from the FSM's operating state. `Usage<T>` resolves the capacity fields
at compile time via `T::CapacityValue`.

### Resource groups

Resource groups enable hierarchical aggregation of resource usages. Any entity
can opt in to being a resource group via `#[quent::resource_group]`. Domain
models may constrain the parent type; application-specific groups leave the
parent flexible.

```rust
// Domain model: fixed hierarchy for query engines
#[quent::entity]
#[quent::resource_group]
pub struct Engine { pub name: String }

#[quent::entity]
#[quent::resource_group(parent = Engine)]
pub struct Query { pub query_group_id: Ref<QueryGroup> }

// Application model: flexible parent
#[quent::resource_group]
pub struct MyCustomGroup;
```

Resources assign their parent group at the instance level, not the type level.
Whether a domain model constrains its hierarchy is a domain-specific decision,
not a framework requirement.

### FSMs

FSMs are declared as structs annotated with `#[quent::fsm]`. Transitions are
listed as attributes on the struct. States are separate structs annotated with
`#[quent::state]`.

```rust
#[quent::fsm]
pub struct Task {
    #[quent::transition(entry -> Queueing)]
    #[quent::transition(Queueing -> Computing)]
    #[quent::transition(Queueing -> Allocating)]
    #[quent::transition(Allocating -> Computing)]
    #[quent::transition(Computing -> Sending)]
    #[quent::transition(Computing -> Spilling)]
    #[quent::transition(Spilling -> Computing)]
    #[quent::transition(Loading -> Computing)]
    #[quent::transition(Sending -> Queueing)]
    #[quent::transition(Computing -> exit)]
}

#[quent::state]
pub struct Queueing {
    pub operator_id: Ref<Operator>,
    pub instance_name: String,
}

#[quent::state]
pub struct Computing {
    #[quent::usage]
    pub thread: Usage<Thread>,
    #[quent::usage]
    pub memory: Usage<WorkerMemory>,
    #[quent::deferred]
    pub rows_processed: Option<u64>,
}

#[quent::state]
pub struct Allocating {
    #[quent::usage]
    pub memory: Usage<WorkerMemory>,
}
```

The `#[quent::fsm]` proc macro validates the FSM in isolation: all referenced
states exist, every state is reachable from entry, every state can reach exit,
and no transitions leave exit.

Transition attributes are per-state: each state struct defines the attributes
for all transitions into that state. When a field is only relevant for
transitions from certain source states, it is declared as `Option<T>`.

Fields marked `#[quent::deferred]` must be `Option<T>` and can be set after the
transition via the state handle. They are emitted as deferred events.

Entity references use `Ref<T>` instead of raw `Uuid`, providing compile-time
type safety. `Ref<T>` resolves to `Uuid` on the wire.

### Resource usages

State fields annotated with `#[quent::usage]` link an FSM state to a resource.
`Usage<T>` is a generic struct that requires `T: Resource` and expands to fields
matching the resource's capacity type:

```rust
pub struct Usage<T: Resource> {
    pub resource_id: Ref<T>,
    pub capacity: T::CapacityValue,
}
```

This is validated by Rust's type system. Referencing a nonexistent or
non-resource type in `Usage<T>` is a compile error. The capacity value type is
derived from the resource FSM's operating state.

### FSM instantiation and event emission

Each FSM instance is created via `new()`, which returns an owned handle. The
handle carries the entity ID (auto-generated UUIDv7), a sequence counter, and
a context reference. Events are emitted immediately on transition. Each event
carries a per-instance sequence number for ordering.

All FSM events use a common wrapper type:

```rust
pub enum FsmEvent<S, D> {
    Transition { seq: u64, state: S },
    Deferred { seq: u64, deferred: D },
}
```

Usage:

```rust
// seq 0: emits entry transition event
let mut task = Task::new(&ctx, QueueingAttrs {
    operator_id: operator.id(),
    instance_name: "scan_0".into(),
});

// seq 1: emits transition event (rows_processed: None)
{
    let state = task.transition(ComputingAttrs {
        thread: Usage { resource_id: thread.id(), capacity: () },
        memory: Usage { resource_id: mem.id(), capacity: MemoryCapacity { used_bytes: 4096 } },
    });

    // seq 2: emits deferred event
    state.set_rows_processed(1000);
} // state handle dropped, borrow on task released

// seq 3: emits transition event
task.transition(SendingAttrs { /* ... */ });

// seq 4: emits exit event
task.exit();
// If exit() is not called, Drop emits it automatically.
```

The state handle borrows `&mut` from the FSM handle — calling `transition()`
while a state handle is alive is a compile error in Rust. In C++ via CXX
bridge, the handle is a `rust::Box<Task>` and the constraint is enforced at
runtime.

### Entities and events

Plain entities (not FSMs, not resources) are declared with `#[quent::entity]`.
An entity's struct fields define its declaration event. Additional event types
are separate structs linked via `#[quent::event(entity = T)]`. All entity events
are one-shot emissions at a point in time.

```rust
#[quent::entity]
pub struct Operator {
    pub plan_id: Ref<Plan>,
    pub type_name: String,
}

#[quent::event(entity = Operator)]
pub struct OperatorStatistics {
    pub rows_processed: u64,
    pub bytes_read: u64,
}
```

Generated API:

```rust
let op = Operator::declare(&ctx, OperatorAttrs {
    plan_id: plan.id(),
    type_name: "scan".into(),
});
op.emit(OperatorStatistics { rows_processed: 1000, bytes_read: 4096 });
```

The handle carries the entity ID and a context reference. The proc macro
validates that only event types declared for that entity can be emitted.

### Composing models across crates

Domain models and application models are regular Rust crates composed via
a type alias over `quent::Model<T>`:

```rust
// Domain model crate
pub type QueryEngineModel = quent::Model<(
    Engine,
    QueryGroup,
    Query,
    Plan,
    Operator,
    Port,
    Worker,
)>;

// Application model crate
pub type SimulatorModel = quent::Model<(
    quent_qe_model::QueryEngineModel,
    Task,
    WorkerMemory,
    Thread,
    FsToMem,
    MemToFs,
)>;
```

Each type implements a `ModelComponent` trait (generated by the proc macros).
`Model<T>` recursively collects metadata from all components via tuple impl.
The codegen binary calls `SimulatorModel::collect(&mut builder)` to get the
fully resolved model.

A top-level event enum is auto-generated from the model, with one variant per
component and `From` impls so each handle can push events without knowing the
top-level type.

## Codegen architecture

### In-memory model representation

The codegen binary operates on a fully resolved in-memory representation of
the model. See [model representation](./model-representation.md) for the full
type definitions.

### C++ target

The C++ backend generates Rust `#[cxx::bridge]` modules that expose FSM and
entity handles and their methods to C++. CXX generates the corresponding C++
headers. The C++ application includes these headers and calls methods directly.
There is no separate C++ runtime library — the Rust instrumentation backend
(Context, event channel, exporters) is the runtime, accessed via CXX.

Event flow:

```
C++ application code
  | calls handle methods (transition, exit, declare, emit)
CXX-generated FFI boundary
  |
Rust handle implementation
  | constructs Event<T>, calls push_event()
Rust EventSender -> event channel
  |
Rust exporters (collector, ndjson, msgpack, postcard, ...)
```

Build integration uses Corrosion (CMake <-> Cargo). This pattern is proven in
the Sirius project.

### Backend configuration

Each codegen backend accepts a configuration struct controlling
language-specific output conventions (naming case styles, namespaces, output
layout). The model definition is unaware of these settings.

```rust
quent_codegen::emit_cpp(
    &model,
    CppOptions {
        method_case: SnakeCase,
        class_case: PascalCase,
        namespace: "myapp::telemetry",
        output_dir: "generated/cpp",
        ..Default::default()
    },
);
```

### Adding target languages

Each target language is a backend in the codegen binary. All backends read from
the same in-memory model. Adding a target means writing a new emitter function.

## Proc macro annotation summary

| Annotation               | Applies to | Purpose                                        |
|--------------------------|------------|-------------------------------------------------|
| `#[quent::fsm]`          | struct     | Declares an FSM with a transition table          |
| `#[quent::state]`        | struct     | Declares a state with transition attributes      |
| `#[quent::resource]`     | FSM struct | Marks an FSM as usable with `Usage<T>`           |
| `#[quent::entity]`       | struct     | Declares a plain entity with a declaration event |
| `#[quent::event]`        | struct     | Declares an additional event for an entity       |
| `#[quent::resource_group]` | struct   | Marks an entity as a resource group              |
| `#[quent::deferred]`     | field      | Marks a state field as settable after transition |
| `#[quent::usage]`        | field      | Links a state field to a resource via `Usage<T>` |
| `#[quent::transition]`   | FSM field  | Declares an allowed transition                   |

## Crate layout

```
quent-model/            proc macro crate + core types (Ref<T>, Usage<T>, traits)
quent-stdlib/           standard library FSM definitions (Memory, Processor, Channel)
quent-codegen/          codegen binary (reads model types, emits target code)

domains/
  query_engine/
    model/              query engine domain model crate

examples/
  simulator/
    model/              simulator application model crate
```

## Status

This design is experimental. Nothing is implemented yet.
