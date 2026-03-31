# Decision: Auto-Generated Top-Level Event Enum

## Context

The instrumentation context is generic over a single event type:
`Context<T>`. All observers push events into `EventSender<T>`. A top-level
enum must exist that wraps all possible event types from the domain model and
application model. In Sirius, this enum is hand-written.

## Decision

The top-level event enum is auto-generated from the model type alias.

## How it works

The `quent::Model<T>` type alias lists all model components. The proc macro
(or a generated trait impl) produces a top-level event enum with one variant
per component, plus `From` impls so each component's handle can wrap its events.

```rust
pub type SimulatorModel = quent::Model<(
    quent_qe_model::QueryEngineModel,
    Task,
    WorkerMemory,
    Thread,
    FsToMem,
    MemToFs,
)>;

// Generated:
pub enum SimulatorEvent {
    // From QueryEngineModel (recursively flattened)
    Engine(EngineEvent),
    Query(QueryEvent),
    Plan(PlanEvent),
    Operator(OperatorEvent),
    Port(PortEvent),
    Worker(WorkerEvent),
    QueryGroup(QueryGroupEvent),
    // From application model
    Task(TaskEvent),
    // Resource events
    WorkerMemory(WorkerMemoryEvent),
    Thread(ThreadEvent),
    FsToMem(FsToMemEvent),
    MemToFs(MemToFsEvent),
}
```

Each component's proc macro generates its own event type (e.g., `TaskEvent`
with variants per transition and amendments). The model-level generation
composes these into the top-level enum.

`From` impls are generated so handles can push events without knowing the
top-level enum:

```rust
impl From<TaskEvent> for SimulatorEvent {
    fn from(e: TaskEvent) -> Self { SimulatorEvent::Task(e) }
}
```

The `Context` is instantiated as `Context<SimulatorEvent>`.

## Rationale

- The model already enumerates all types. Generating the enum avoids manual
  duplication and guarantees it stays in sync.
- Composed domain models are recursively flattened — the top-level enum has
  one variant per leaf component, not nested enums.
- `From` impls allow each handle to push events without coupling to the
  top-level enum type, keeping component code independent.
