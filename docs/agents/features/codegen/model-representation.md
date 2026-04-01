# In-Memory Model Representation

This is the fully resolved data structure from which codegen backends emit
language-specific code. It is populated by collecting metadata from proc
macro-generated trait impls on the model types.

Every type here maps directly to a concept in the
[modeling specification](../../docs/modeling/).

```rust
/// The full application model.
pub struct Model {
    pub name: String,
    pub fsms: Vec<FsmDef>,
    pub resources: Vec<ResourceDef>,
    pub entities: Vec<EntityDef>,
    pub resource_groups: Vec<ResourceGroupDef>,
}

// ---------------------------------------------------------------------------
// FSMs
// ---------------------------------------------------------------------------

pub struct FsmDef {
    pub name: String,
    pub states: Vec<StateDef>,
    pub transitions: Vec<TransitionDef>,
}

pub struct StateDef {
    pub name: String,
    /// Non-deferred attributes, required at transition time.
    pub attributes: Vec<AttributeDef>,
    /// Deferred attributes, settable after transition via the state handle.
    pub deferred_attributes: Vec<AttributeDef>,
    pub usages: Vec<UsageDef>,
}

pub struct TransitionDef {
    pub from: TransitionEndpoint,
    pub to: TransitionEndpoint,
}

pub enum TransitionEndpoint {
    Entry,
    Exit,
    State(String),
}

pub struct UsageDef {
    pub field_name: String,
    pub resource_name: String,
    pub capacities: Vec<CapacityValueDef>,
}

pub struct CapacityValueDef {
    pub name: String,
    pub value_type: ValueType,
}

// ---------------------------------------------------------------------------
// Resources
// ---------------------------------------------------------------------------

pub struct ResourceDef {
    pub name: String,
    pub kind: ResourceKind,
    pub capacities: Vec<CapacityDef>,
}

pub enum ResourceKind {
    Unit,
    FixedBounds,
    DynamicBounds,
}

pub struct CapacityDef {
    pub name: String,
    pub capacity_type: CapacityType,
    pub value_type: ValueType,
    pub bounded: bool,
}

pub enum CapacityType {
    Occupancy,
    Rate,
}

// ---------------------------------------------------------------------------
// Resource groups
// ---------------------------------------------------------------------------

pub struct ResourceGroupDef {
    pub name: String,
    /// If set, the parent type is fixed (domain model constraint).
    /// If None, the parent is assigned at runtime via Option<Uuid>.
    pub fixed_parent: Option<String>,
}

// ---------------------------------------------------------------------------
// Entities and events
// ---------------------------------------------------------------------------

/// A plain entity (not an FSM, not a resource).
pub struct EntityDef {
    pub name: String,
    /// Attributes emitted in the declaration event.
    pub attributes: Vec<AttributeDef>,
    /// Additional one-shot event types associated with this entity.
    pub events: Vec<EntityEventDef>,
}

/// A one-shot event type associated with an entity.
pub struct EntityEventDef {
    pub name: String,
    pub attributes: Vec<AttributeDef>,
}

// ---------------------------------------------------------------------------
// Shared types
// ---------------------------------------------------------------------------

pub struct AttributeDef {
    pub name: String,
    pub value_type: ValueType,
    pub optional: bool,
}

/// Maps to the spec's attribute value types.
pub enum ValueType {
    Bool,
    Uuid,
    String,
    U8, U16, U32, U64,
    I8, I16, I32, I64,
    F32, F64,
    List(Box<ValueType>),
    Struct(Vec<AttributeDef>),
    /// A typed reference to another entity, FSM, or resource.
    /// Resolves to Uuid on the wire. The string is the referenced type's name.
    Ref(String),
}
```

## Notes

- This representation is fully resolved. Generic types like `Usage<T>` have
  been expanded: `Usage<WorkerMemory>` becomes a `UsageDef` with
  `resource_name: "worker_memory"` and the concrete capacity fields from
  `WorkerMemory`'s `Resource` impl. Similarly, `Ref<Query>` becomes
  `ValueType::Ref("query")`.

- Canonical names are derived from the Rust struct names (converted to
  snake_case). Codegen backends transform these according to their own
  configuration (e.g., PascalCase for C++ classes, snake_case for methods).

- An entity may also be a resource group. If so, it appears in both
  `entities` and `resource_groups`. The relationship is by name.

- `StateDef` separates non-deferred and deferred attributes. Non-deferred
  attributes are required at transition time. Deferred attributes are
  `Option<T>` and can be set after the transition via the state handle,
  emitted as amendment events.

- FSM events carry a per-instance sequence number (`u64`) for ordering.
  The sequence number is not part of the model representation — it is a
  runtime mechanism on the FSM handle.

- This structure is not serialized to disk under normal operation. The codegen
  binary constructs it in-memory by calling trait methods on the imported model
  types. If serialization is ever needed, `Serialize`/`Deserialize` can be
  derived on these types.
