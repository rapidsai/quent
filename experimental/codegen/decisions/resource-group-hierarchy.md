# Decision: Resource Group Hierarchy Expression

## Context

The spec requires resources to belong to resource groups, and resource groups to
form a tree via `parent_group_id`. In the query engine domain model, domain
entities (Engine, QueryGroup, Query, Plan, Operator, Port, Worker) themselves
act as resource groups with a fixed hierarchy.

The question is how parent-child relationships are expressed in the Rust model
definition, and whether they are type-level constraints or runtime assignments.

## Decision

Domain model entities may declare a fixed parent type via
`parent = T` on `#[resource_group]`. All other resource groups and resources
assign their parent at runtime via an `Option<Uuid>`.

## Design

### Fixed parent (domain models)

Domain entities that form a known hierarchy declare the parent type:

```rust
#[derive(Entity, ResourceGroup)]
#[resource_group(root)]
pub struct Engine { pub name: String }

#[derive(Entity, ResourceGroup)]
#[resource_group]
pub struct QueryGroup { /* ... */ }

#[derive(Entity, ResourceGroup)]
#[resource_group]
pub struct Query { /* ... */ }

#[derive(Entity, ResourceGroup)]
#[resource_group]
pub struct Plan { /* ... */ }
```

The root resource group (`Engine`) declares no parent.

When `parent = T` is specified:

- The proc macro validates that `T` is a resource group.
- The generated API accepts an instance of `T` (or its ID), not a raw UUID.
  This is a compile-time guarantee that the parent is the correct type.
- The hierarchy is known statically. Codegen and the UI can render the tree
  structure without runtime discovery.

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

#[derive(Entity, ResourceGroup)]
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

- Some domain models (e.g., query engine) define a fixed entity hierarchy that
  tooling (UI, analyzer) depends on. For these domains, the hierarchy should be
  part of the type definition so it can be validated at compile time and emitted
  statically.
- Other domain models may not have a rigid hierarchy. The `parent = T`
  parameter is a tool available to domain model authors, not a framework
  requirement. Whether a domain model constrains its hierarchy is a
  domain-specific decision.
- Application-specific resources and groups should not be restricted to a
  particular parent type. Different applications using the same domain model may
  organize their resources differently.
- Keeping the parent constraint optional avoids forcing artificial hierarchy
  decisions on model authors while still giving domains that need rigidity the
  ability to enforce it.
