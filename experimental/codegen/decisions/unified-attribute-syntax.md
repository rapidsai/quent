# Decision: Unified `#[quent_model(...)]` Attribute with Composable Flags

## Context

The original design used separate proc macro attributes (`#[quent_model::fsm]`,
`#[quent_model::state]`, `#[quent_model::entity]`, `#[quent_model::resource_group]`,
`#[quent_model::event]`). This caused stacking conflicts — `#[entity]` and
`#[resource_group]` both generated `ModelComponent` impls and could not be
combined on the same struct.

## Decision

A single `#[quent_model(...)]` attribute with composable flags.

## Entity kinds

### FSM — user-defined state machine

```rust
#[quent_model(state)]
pub struct Init { ... }

#[quent_model(fsm(
    entry -> Init,
    Init -> Planning,
    Planning -> exit,
))]
pub struct Query;
```

### Entity — one or more explicitly listed events

```rust
#[quent_model(entity(
    events(Declaration, Statistics),
))]
pub struct Operator;

pub struct Declaration { ... }
pub struct Statistics { ... }
```

### FSM or Entity + Resource Group

`resource_group` is a top-level modifier, comma-separated after the primary
kind. It never nests inside `fsm(...)` or `entity(...)`:

```rust
#[quent_model(fsm(
    entry -> Init,
    Init -> exit,
), resource_group)]
pub struct Query;

#[quent_model(entity(
    events(EngineInit, EngineExit),
), resource_group(root))]
pub struct Engine;
```

### State — FSM state struct

```rust
#[quent_model(state)]
pub struct Computing {
    #[usage]
    pub thread: Usage<Thread>,
    #[deferred]
    pub rows_processed: Option<u64>,
    #[capacity]
    pub capacity_bytes: u64,
    #[instance_name]
    pub instance_name: String,
}
```

## Composable modifiers

| Modifier | Position | Purpose |
|----------|----------|---------|
| `resource_group` | Top-level, after primary kind | Marks as resource group |
| `resource_group(root)` | Top-level, after primary kind | Marks as root resource group |
| `events(T1, T2, ...)` | Inside `entity(...)` | Lists associated event types |

## What is NOT user-definable

Resources (Memory, Processor, Channel) are predefined in the stdlib. They
have fixed FSMs matching the spec. Application code uses them via type
aliases (`type WorkerMemory = quent_stdlib::MemoryResource`) and
references them in `Usage<T>` fields. The `resource(capacity = T)` syntax
is a stdlib-internal concern, not part of the public model API.

A resource is a leaf in the hierarchy — it cannot be a resource group.
Any other entity (FSM or otherwise) can be a resource group.

## Field-level annotations

These remain as plain attributes on struct fields within `#[quent_model(state)]`:

- `#[usage]` — marks a `Usage<T>` field
- `#[deferred]` — marks an `Option<T>` field as settable after transition
- `#[capacity]` — marks a numeric field as a capacity value
- `#[instance_name]` — marks a String field as the entity instance name

## Rationale

- One proc macro entry point eliminates stacking conflicts
- Flags compose naturally — `resource_group` works with both FSM and entity
- Entity events are explicitly listed, matching how FSM states are referenced
  in the transition table
- Every entity must have events (otherwise it cannot exist in the model)
- Consistent structure: FSMs reference state types, entities reference event types
