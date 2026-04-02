# Decision: Derive-Style Macros for Model Definitions

## Context

The original design used separate proc macro attributes, then evolved to a
unified `#[quent_model(...)]` attribute. Both approaches used opaque attribute
macros that IDEs couldn't introspect. Additionally, `#[quent_model(entity(events(...)))]`
listed events as a parameter, disconnected from the struct body.

## Decision

Derive macros (`#[derive(State)]`, `#[derive(Fsm)]`, `#[derive(Entity)]`) with
helper attributes. Entity events and FSM states are struct fields, making the
definitions self-documenting and IDE-friendly.

## Syntax

### State

```rust
#[derive(State)]
pub struct Running {
    pub thread: Usage<ProcessorResource>,
    #[deferred]
    pub rows: Option<u64>,
    #[capacity]
    pub capacity_bytes: u64,
    #[instance_name]
    pub instance_name: String,
}
```

### FSM — states as fields, transitions as annotations

```rust
#[derive(Fsm)]
pub struct Task {
    #[entry, to(Running)]
    idle: Idle,
    #[to(Idle, exit)]
    running: Running,
}
```

- `#[entry]` marks the initial state
- `#[to(...)]` lists valid next states
- `exit` is the terminal state keyword
- The field name is the state's identifier
- The field type is the state struct

### Entity — events as fields

```rust
#[derive(Entity)]
pub struct Operator {
    #[event]
    declaration: Declaration,
    #[event]
    statistics: Statistics,
}

pub struct Declaration { pub plan_id: Uuid, pub name: String }
pub struct Statistics { pub rows: u64 }
```

- `#[event]` marks a field as an event type
- The field name becomes the observer method name
- The field type becomes the event enum variant

### Resource Group — outer attribute on Entity or FSM

Resource groups are always entities. The `#[resource_group]` (or
`#[resource_group(root)]`) outer attribute is detected by the `Entity` and
`Fsm` derives. There is no standalone `ResourceGroup` derive.

```rust
#[derive(Entity)]
#[resource_group(root)]
pub struct Engine {
    #[event]
    init: Init,
    #[event]
    exit: Exit,
}

#[derive(Fsm)]
#[resource_group]
pub struct Query {
    #[entry, to(Planning)]
    init: Init,
    #[to(Executing)]
    planning: Planning,
    #[to(exit)]
    executing: Executing,
}
```

Eventless resource group entities (no `#[event]` fields) get an implicit
declaration event:

```rust
#[derive(Entity)]
#[resource_group]
pub struct QueryGroup;
```

Non-root resource group FSMs must have `#[parent_group]` on a field in their
entry state to provide the parent resource group UUID. This is enforced at
compile time.

```rust
#[derive(State)]
pub struct Init {
    #[parent_group]
    pub query_group_id: Uuid,
}
```

## Model composition

```rust
quent_model::define_model! {
    pub MyModel(MyEvent) {
        Domain: QueryEngineModelDef,
        Task: Task,
        Job: Job,
    }
}

quent_model::define_context!(pub MyContext(MyEvent));
```

## What is NOT user-definable

Resources (Memory, Processor, Channel) are predefined in the stdlib.
Application code references them in `Usage<T>` fields. A resource is a
leaf in the hierarchy — it cannot be a resource group. Any other entity
(FSM or otherwise) can be a resource group.

## Field-level annotations on State structs

- `Usage<T>` fields are detected automatically by type (no annotation needed)
- `#[deferred]` — marks an `Option<T>` field as settable after transition
- `#[capacity]` — marks a numeric field as a capacity value
- `#[instance_name]` — marks a String field as the entity instance name
- `#[parent_group]` — marks a UUID field as the parent resource group reference

## Rationale

- Derive macros are IDE-friendly (autocomplete, trait documentation)
- Entity events and FSM states are struct fields — self-documenting
- `#[event]` on entity fields mirrors `#[to(...)]` on FSM fields
- `#[resource_group]` as an outer attribute keeps resource group semantics
  separate from the derive list — it modifies behavior rather than defining a
  new trait impl
- `#[parent_group]` on State fields provides compile-time enforcement of
  parent resource group linkage for non-root FSM resource groups
- Eventless resource groups get implicit declaration events, reducing
  boilerplate for pure grouping entities
- Field names drive generated API (observer method names, data struct fields)
- Consistent pattern: both FSMs and entities use fields to declare components
