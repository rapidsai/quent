# Decision: Model Collection via Type Alias and Tuple Composition

## Context

The codegen binary needs to collect metadata from all model types (FSMs,
Resources, Entities) across domain model crates and the application model crate.
The mechanism must support composing domain models into application models.

## Options considered

- **Explicit enumeration via named struct fields**: readable but the field names
  are meaningless — the struct is a type listing, not a data structure.
- **Tuple struct**: clearly a type list, but confusing what the struct represents.
- **Associated types on an impl block**: verbose for what it does.
- **Declarative macro invocation**: explicit intent but limited error reporting.
- **Module-level attribute macro**: ties back to the module-level approach
  rejected in a prior decision.
- **Trait impl with associated type**: works but unnecessary indirection.
- **Type alias over a generic `Model<T>`**: reads naturally, no fake struct or
  fields, nesting composes via recursion.
- **Const/static with runtime metadata**: loses type-level composition.

## Decision

Type alias over `quent::Model<T>` where `T` is a tuple of model components.

```rust
quent_model::define_model! {
    pub SimulatorModel(SimulatorEvent) {
        QueryEngine: quent_qe_model::QueryEngineModel,
        Task: Task,
        WorkerMemory: WorkerMemory,
        Thread: Thread,
        FsToMem: FsToMem,
        MemToFs: MemToFs,
    }
}
```

Domain models use the same pattern:

```rust
quent_model::define_model! {
    pub QueryEngineModel(QueryEngineEvent) {
        Engine: Engine,
        QueryGroup: QueryGroup,
        Query: Query,
        Plan: Plan,
        Operator: Operator,
        Port: Port,
        Worker: Worker,
    }
}
```

## How it works

Each type annotated with a quent derive macro (`#[derive(Fsm)]`, `#[derive(Entity)]`,
etc.) gets a generated `ModelComponent` trait impl with a `collect()` method
that contributes its metadata to a `ModelBuilder`.

`define_model!` generates a model struct that implements `ModelComponent` by
delegating to each field's impl. Each field's `collect()` is called in
sequence. This is a standard Rust pattern (used by serde, axum, bevy, etc.).

When a field is itself a model (i.e., a composed domain model), collection
recurses into it. The result is a flat sequence of metadata collection calls:

```
SimulatorModel::collect()
  → QueryEngineModel::collect()
      → Engine::collect()
      → QueryGroup::collect()
      → Query::collect()
      → Plan::collect()
      → Operator::collect()
      → Port::collect()
      → Worker::collect()
  → Task::collect()
  → WorkerMemory::collect()
  → Thread::collect()
  → FsToMem::collect()
  → MemToFs::collect()
```

The codegen binary calls `SimulatorModel::collect(&mut builder)` once and
receives a fully populated `Model` struct with all FSMs, resources, and entities.

## Rationale

- Reads naturally: "SimulatorModel is a Model composed of these types."
- No fake structs, fields, or data — it is purely a type-level declaration.
- Composition via nesting: including a domain model is just another element in
  the tuple.
- Tuple arity limit (16-32 direct elements) is not a practical constraint since
  domain models collapse into a single element.
