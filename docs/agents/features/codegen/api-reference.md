# Quent Modeling API — Complete Reference

## Spec concept → Macro mapping

| Spec concept | Macro | Meaning |
|---|---|---|
| **Entity** | `#[derive(Entity)]` | Any discrete runtime construct that produces telemetry |
| **FSM** | `#[derive(Fsm)]` | Entity with a set of States and allowed Transitions |
| **State** | `#[derive(State)]` | A named state within an FSM, with typed attributes |
| **Transition** | `#[entry]`, `#[to(...)]` | Allowed state changes (declared on FSM fields) |
| **Resource** (fixed bounds) | `#[derive(Resource)]` | Entity+FSM with Capacities (init→operating→finalizing→exit) |
| **Resource** (dynamic bounds) | `#[derive(ResizableResource)]` | Resource with resizing cycle (operating↔resizing) |
| **Resource Group** | `#[resource_group]` on Entity/Fsm | Hierarchical grouping for resource aggregation |
| **Usage** | `Usage<T>` type (auto-detected) | Claim on a Resource's Capacity during a state |
| **Capacity** | `Capacity<V, K>` type (auto-detected) | Named quantity on a Resource that can be claimed |
| **Attribute** | struct fields | Typed data accompanying a Transition or Event |

---

## Derive macros

### `#[derive(State)]`

An FSM state. Fields are the transition attributes — the data emitted when
entering this state.

```rust
#[derive(Debug, Clone, State, serde::Serialize, serde::Deserialize)]
pub struct Computing {
    pub thread: Usage<ProcessorResource>,              // resource claim (auto-detected)
    pub memory: Usage<MemoryResource>,                 // resource claim (auto-detected)
    #[deferred] pub rows_processed: Option<u64>,       // set after transition
}
```

**Field annotations on State:**

| Annotation | Meaning | Spec concept |
|---|---|---|
| _(none — auto-detected)_ | `Usage<T>` fields are detected by type — claims capacity from a Resource | Usage |
| _(none — auto-detected)_ | `Capacity<V, K>` fields are detected by type — capacity values on a Resource | Capacity |
| `#[deferred]` | Field is `Option<T>` — settable after transition via amendment event | Deferred attribute |
| `#[instance_name]` | Field provides the entity's instance name for the analyzer | Entity.instance_name |
| `#[parent_group]` | Field provides the parent resource group UUID | ResourceGroup.parent_group_id |

---

### `#[derive(Fsm)]`

An entity with a lifecycle defined by state transitions. Fields are the
states. Annotations declare the transition graph.

```rust
#[derive(Fsm)]
pub struct Task {
    #[entry] #[to(Allocating)]
    pub queueing: Queueing,
    #[to(Computing, Loading)]
    pub allocating: Allocating,
    #[to(Sending, Spilling, exit)]
    pub computing: Computing,
    #[to(Allocating)]
    pub spilling: Spilling,
    #[to(Queueing)]
    pub sending: Sending,
    #[to(Computing)]
    pub loading: Loading,
}
```

**Field annotations on Fsm:**

| Annotation | Meaning |
|---|---|
| `#[entry]` | This is the initial state (exactly one required) |
| `#[to(A, B, exit)]` | Allowed transitions from this state |
| `exit` keyword | The terminal state (spec: every FSM must reach exit) |

**Validates at compile time:** reachability from entry, every state can reach
exit, no transitions out of exit, all fields must be `pub`.

**Generates:** `TaskTransition` enum, `TaskDeferred` enum, `TaskEvent` type
alias (`FsmEvent<S, D>`), `TaskHandle<E>` (instrumentation handle with
`{entry_field_name}()` named constructor, `transition()`, `exit()`, auto-exit
on Drop), `ModelComponent` impl, `TransitionInfo` impl, `HasEventType` impl.

**Struct-level annotations on Fsm:**

| Annotation | Meaning |
|---|---|
| `#[resource(capacity = T)]` | This FSM is a Resource; `T` is the Operating state providing capacity |
| `#[resource_group]` | This FSM is also a Resource Group |
| `#[resource_group(root)]` | This FSM is the root Resource Group |

---

### `#[derive(Entity)]`

A non-FSM entity that emits one-shot events. Fields marked `#[event]`
declare the event types.

```rust
#[derive(Entity)]
#[resource_group(root)]
pub struct Engine {
    #[event] pub init: Init,
    #[event] pub exit: Exit,
}

pub struct Init {
    pub instance_name: Option<String>,
    pub implementation: Option<EngineImplementationAttributes>,
}

pub struct Exit;
```

**Field annotations on Entity:**

| Annotation | Meaning |
|---|---|
| `#[event]` | This field's type is an event the entity can emit |

**Struct-level annotations on Entity:**

| Annotation | Meaning |
|---|---|
| `#[resource_group]` | This entity is also a Resource Group |
| `#[resource_group(root)]` | This entity is the root Resource Group |

**Generates:** `EngineEvent` enum (one variant per `#[event]` field),
`EngineObserver<E>` (one method per event, named after the field),
`EngineData` struct (`Option<T>` per event for analyzer), `From` impls,
`HasEventType` impl, `EntityData` impl, `ModelComponent` impl. All fields
must be `pub`.

---

### `#[derive(Resource)]`

A fixed-bounds resource. The spec's `init → operating → finalizing → exit`
lifecycle is generated automatically.

```rust
#[derive(Resource)]
pub struct Memory {
    pub capacity_bytes: Capacity<u64, Occupancy>,
}

#[derive(Resource)]
pub struct Processor;  // unit resource — no capacity

#[derive(Resource)]
pub struct Channel {
    pub source_id: Uuid,                          // goes on Initializing state
    pub target_id: Uuid,                          // goes on Initializing state
    pub capacity_bytes: Capacity<Option<u64>, Rate>,  // goes on Operating state
}
```

**Field detection on Resource:**

| Field type | Meaning |
|---|---|
| `Capacity<V, K>` | Field goes on the generated Operating state (the capacity being offered) |
| _(other)_ | Field goes on the generated Initializing state (metadata set at creation) |

`Capacity<V, K>` wraps a value `V` with a kind marker `K` (`Occupancy` or
`Rate`). `V` is restricted to `u64` (bounded) or `Option<u64>` (unbounded,
`None` = no maximum), matching the spec's non-negative integer requirement.
The kind defaults to `Occupancy` if omitted: `Capacity<u64>` is equivalent
to `Capacity<u64, Occupancy>`.

**Auto-generated Initializing state fields:** `instance_name: String`,
`parent_group_id: Uuid`, `resource_type_name: String` — present on every
resource.

**Generates:** `{Name}Initializing`, `{Name}Operating`, `{Name}Finalizing`
states, full FSM, `{Name}Handle<E>` with `operating()`, `finalizing()`,
`exit()` methods, `{Name}Resource` marker for `Usage<T>`, all trait impls.

---

### `#[derive(ResizableResource)]`

Same as `Resource` but adds the operating ↔ resizing cycle from the spec's
Dynamic-Bounds Resource.

```rust
#[derive(ResizableResource)]
pub struct ResizableMemory {
    pub capacity_bytes: Capacity<u64, Occupancy>,
}
```

Additional generated: `{Name}Resizing` state, `resizing()` method on handle.

---

### Resource groups

Resource groups are always entities. Use `#[resource_group]` on an Entity or
Fsm struct — there is no standalone `#[derive(ResourceGroup)]`.

**Eventless resource group** — the derive generates an implicit declaration
event with `instance_name` (and `parent_group_id` for non-root):

```rust
#[derive(Entity)]
#[resource_group]
pub struct ThreadPool;
// Generates: ThreadPoolDeclaration { instance_name, parent_group_id }

#[derive(Entity)]
#[resource_group(root)]
pub struct Engine;
// Generates: EngineDeclaration { instance_name }
```

**Entity with events + resource group** — user defines events, should include
a parent reference field:

```rust
#[derive(Entity)]
#[resource_group]
pub struct Operator {
    #[event] pub declaration: Declaration,
    #[event] pub statistics: Statistics,
}
```

**FSM + resource group** — the entry state MUST have `#[parent_group]` on
a field (enforced at compile time for non-root):

```rust
#[derive(State)]
pub struct Init {
    #[parent_group]
    pub query_group_id: Uuid,
    #[instance_name]
    pub instance_name: String,
}

#[derive(Fsm)]
#[resource_group]
pub struct Query {
    #[entry] #[to(Planning)]
    pub init: Init,
    ...
}
```

---

## Composition macros

### `define_model!`

Composes model components into a single model type and event enum.

```rust
quent_model::define_model! {
    Simulator {
        quent_query_engine_model::engine::Engine,
        quent_query_engine_model::query::Query,
        quent_simulator_model::task::Task,
        quent_stdlib::Memory,
        quent_stdlib::Processor,
        quent_stdlib::Channel,
    }
    extra {
        Trace: quent_events::trace::TraceEvent,
    }
}
```

**Generates:** `SimulatorModel` (type alias for `Model<(...)>`),
`SimulatorEvent` (event enum with one variant per component + extras),
`From` impls for each component's event type.

Variant names derived from last path segment: `quent_stdlib::Memory` →
`Memory(MemoryEvent)`.

The `extra {}` section includes non-model event types in the enum without
adding them to the Model type.

---

### `define_context!`

Generates the instrumentation context wrapping `Context<E>`.

```rust
quent_model::define_context!(pub SimulatorContext(SimulatorEvent));
```

**Generates:** struct with `try_new(exporter, id)` and `events_sender()`.

---

## Core types

| Type | Purpose |
|---|---|
| `Usage<T: Resource>` | Resource usage — `{ resource_id: Ref<T>, capacity: T::CapacityValue }` |
| `Capacity<V, K>` | Resource capacity value — `V` is `u64` or `Option<u64>`, `K` is `Occupancy` or `Rate` |
| `Occupancy` | Capacity kind: usage value = amount held during a Span |
| `Rate` | Capacity kind: usage value = total quantity processed over a Span |
| `Ref<T>` | Typed entity reference — `Uuid` on the wire, type-safe at compile time |
| `FsmEvent<S, D>` | Common FSM event wrapper — `Transition { seq, state }` or `Deferred { seq, deferred }` |
| `{Name}Handle<E>` | FSM instrumentation handle — `new()`, `transition()`, `exit()`, auto-exit on Drop |
| `{Name}Observer<E>` | Entity observer — one method per event type |
| `AnalyzedFsm<T>` | Generic FSM reconstruction in the analyzer |
| `AnalyzedEntity<M>` | Generic entity reconstruction in the analyzer |
| `AnalyzedResource<T>` | Generic resource reconstruction in the analyzer |

---

## Standard library (`quent-stdlib`)

Predefined resources matching the spec's common entity types:

| Type | Spec concept | Capacity |
|---|---|---|
| `Memory` | Fixed-bounds memory | `capacity_bytes: Capacity<u64, Occupancy>` |
| `ResizableMemory` | Dynamic-bounds memory | `capacity_bytes: Capacity<u64, Occupancy>` |
| `Processor` | Unit resource (computation) | None (unit) |
| `Channel` | Data transfer | `capacity_bytes: Capacity<Option<u64>, Rate>` |

Usage in state definitions:

```rust
#[derive(State)]
pub struct Computing {
    pub thread: Usage<ProcessorResource>,
    pub memory: Usage<MemoryResource>,
}
```
