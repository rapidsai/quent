# Decision: Resource Group Hierarchy Expression

## Context

The spec requires resources to belong to resource groups, and resource groups to
form a tree via `parent_group_id`. In the query engine domain model, domain
entities (Engine, QueryGroup, Query, Plan, Operator, Port, Worker) themselves
act as resource groups with a fixed hierarchy.

The question is how parent-child relationships are expressed in the Rust model
definition, and whether they are type-level constraints or runtime assignments.

## Decision

Resource groups are always entities (Entity or FSM). The `#[resource_group]`
outer attribute marks an entity or FSM as a resource group. There is no
standalone `ResourceGroup` derive.

Parent-child relationships are expressed via:
- `#[parent_group]` field annotation on FSM entry states (compile-time enforced
  for non-root resource group FSMs)
- Runtime assignment via event data for entities

## Design

### Resource group entities

Entity resource groups use `#[resource_group]` as an outer attribute on
`#[derive(Entity)]`. The root uses `#[resource_group(root)]`:

```rust
#[derive(Entity)]
#[resource_group(root)]
pub struct Engine {
    #[event]
    init: Init,
    #[event]
    exit: Exit,
}

#[derive(Entity)]
#[resource_group]
pub struct Plan {
    #[event]
    declaration: Declaration,
}
```

### Eventless resource groups

Entity resource groups with no `#[event]` fields get an implicit declaration
event. This reduces boilerplate for pure grouping entities:

```rust
#[derive(Entity)]
#[resource_group]
pub struct QueryGroup;
```

### FSM resource groups with `#[parent_group]`

FSMs that are resource groups use `#[resource_group]` as an outer attribute on
`#[derive(Fsm)]`. Non-root resource group FSMs must annotate a field on their
entry state with `#[parent_group]` to provide the parent resource group UUID.
This is enforced at compile time — the derive macro emits an error if a
non-root resource group FSM's entry state lacks `#[parent_group]`.

```rust
#[derive(State)]
pub struct Init {
    #[parent_group]
    pub query_group_id: Uuid,
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

### Flexible parent (application-specific)

Resources and application-specific resource groups do not declare a parent type:

```rust
#[derive(Fsm)]
#[resource(capacity = Operating)]
pub struct WorkerMemory {
    #[entry, to(Operating)]
    initializing: Initializing,
    #[to(exit)]
    operating: Operating,
}

#[derive(Entity)]
#[resource_group]
pub struct MyCustomGroup;
```

The generated API accepts an `Option<Uuid>` for the parent group, assigned at
runtime. This allows the same resource type to be placed under different groups
in different applications or configurations.

### No group constraint on resources

Resource type definitions (memory, processor, channel, custom) do not restrict
which resource group they can belong to. A `WorkerMemory` might be under a
`Worker` in one application and under a different group in another. The
placement is an instance-level decision, not a type-level one.

## Rationale

- Resource groups are always entities — they represent identifiable things in
  the system. The `#[resource_group]` attribute modifies behavior rather than
  defining a separate trait, keeping the derive list clean.
- `#[parent_group]` on FSM entry states provides compile-time enforcement that
  non-root FSM resource groups supply a parent UUID on creation. This catches
  missing parent assignments at compile time rather than runtime.
- Eventless resource groups with implicit declaration events reduce boilerplate
  for entities that exist purely for grouping (e.g., `QueryGroup`).
- Application-specific resources and groups should not be restricted to a
  particular parent type. Different applications using the same domain model may
  organize their resources differently.
- Keeping the parent constraint optional avoids forcing artificial hierarchy
  decisions on model authors while still giving domains that need rigidity the
  ability to enforce it.
