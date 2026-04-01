# Decision: Typed Entity References

## Context

Entity attributes frequently reference other entities by UUID (e.g., a Plan
references a Query via `query_id`, a PlanEdge references Ports). Using raw
`Uuid` fields loses semantic information — the model doesn't express what type
of entity is being referenced, and the instrumentation API can't prevent passing
the wrong entity's ID.

## Decision

Use `Ref<T>` for entity references. `Ref<T>` resolves to `Uuid` on the wire
but provides compile-time type safety in the instrumentation API.

## Example

```rust
#[derive(Entity)]
pub struct Plan {
    pub name: String,
    pub query_id: Ref<Query>,
    pub edges: Vec<PlanEdge>,
}

pub struct PlanEdge {
    pub source: Ref<Port>,
    pub target: Ref<Port>,
}

#[derive(State)]
pub struct Queueing {
    pub operator_id: Ref<Operator>,
    pub instance_name: String,
}
```

## How it works

- `Ref<T>` requires `T` to be an entity, FSM, or resource type known to the
  model. The proc macro validates this.
- Entity and FSM handles return `Ref<Self>` from their `id()` method, not
  raw `Uuid`.
- The instrumentation API accepts `Ref<T>` for reference fields, preventing
  accidental use of the wrong entity type's ID.
- On the wire and in the model representation, `Ref<T>` is a `Uuid`. The type
  parameter is erased at serialization time.
- The codegen backend emits `Uuid` (or the target language equivalent) for
  reference fields.

## Model representation

`Ref<T>` adds a reference target to the `ValueType` enum:

```rust
pub enum ValueType {
    // ...existing types...
    Ref(String),  // name of the referenced entity/FSM/resource type
}
```

This tells the codegen and the analyzer which entity type is being referenced.

## Benefits

- **Compile-time safety**: passing an Operator ID where a Query ID is expected
  is a type error.
- **Model semantics**: the model explicitly documents which entities reference
  which. Tooling (UI, analyzer) can use this to render relationships and
  validate referential integrity.
- **Zero runtime cost**: `Ref<T>` is a newtype over `Uuid`. The type parameter
  is phantom — no additional data stored or transmitted.
