# Decision: Resources Are Standard Library FSM Definitions

## Context

Resources have spec-defined lifecycles (initializing → operating → finalizing →
exit). Previously, dedicated annotations (`#[quent::memory]` (removed),
`#[quent::processor]` (removed), `#[quent::channel]` (removed)) were proposed to generate these
FSMs. The question is whether resources need special code generation or can
use the same FSM mechanism as everything else.

## Decision

Memory, Processor, Channel, and other common resource types are ordinary FSM
definitions shipped in a standard library crate. No special resource-specific
annotations or code generation. A single `#[resource(capacity = T)]` on an FSM
adds the `Resource` trait so `Usage<T>` works.

## Standard library definitions

```rust
// In quent-stdlib

#[derive(Fsm)]
#[resource(capacity = Operating)]
pub struct Memory {
    #[entry, to(Operating)]
    initializing: Initializing,
    #[to(Finalizing)]
    operating: Operating,
    #[to(exit)]
    finalizing: Finalizing,
}

#[derive(State)]
pub struct Initializing;

#[derive(State)]
pub struct Operating {
    pub capacity_bytes: u64,
}

#[derive(State)]
pub struct Finalizing;
```

Dynamic-bounds resources add the resizing cycle:

```rust
#[derive(Fsm)]
#[resource(capacity = Operating)]
pub struct DynamicMemory {
    #[entry, to(Operating)]
    initializing: Initializing,
    #[to(Resizing, Finalizing)]
    operating: Operating,
    #[to(Operating)]
    resizing: Resizing,
    #[to(exit)]
    finalizing: Finalizing,
}
```

Processor (unit resource):

```rust
#[derive(Fsm)]
#[resource(capacity = Operating)]
pub struct Processor {
    #[entry, to(Operating)]
    initializing: Initializing,
    #[to(Finalizing)]
    operating: Operating,
    #[to(exit)]
    finalizing: Finalizing,
}

#[derive(State)]
pub struct Operating;  // no fields = unit resource
```

## Application usage

Applications use these types directly or alias them:

```rust
pub type WorkerMemory = quent_stdlib::Memory;
pub type Thread = quent_stdlib::Processor;
pub type FsToMem = quent_stdlib::Channel;
```

## Resource trait and Usage<T>

`#[resource(capacity = T)]` on an FSM generates a `Resource` trait impl. The capacity
type is derived from the FSM's operating state:

```rust
impl Resource for Memory {
    type CapacityValue = Operating;
}
```

`Usage<Memory>` expands to:

```rust
Usage<Memory> {
    resource_id: Uuid,
    capacity: Operating,  // { capacity_bytes: u64 }
}
```

For unit resources (Processor), the operating state has no fields, so
`Usage<Processor>` is just `{ resource_id: Uuid }`.

## Annotations removed

The following annotations are no longer needed:

- `#[quent::memory]` (removed) — replaced by `quent_stdlib::Memory` FSM
- `#[quent::processor]` (removed) — replaced by `quent_stdlib::Processor` FSM
- `#[quent::channel]` (removed) — replaced by `quent_stdlib::Channel` FSM

The only resource-related annotation is `#[resource(capacity = T)]`, which can be
applied to any FSM to mark it as usable with `Usage<T>`.

## Rationale

- One mechanism for everything. Resources and application FSMs use the same
  `#[derive(Fsm)]` and `#[derive(State)]` annotations. No special cases in the
  proc macro.
- Standard library FSMs are reusable definitions, not framework magic. An
  application can define its own resource FSM with custom states if the
  standard ones don't fit.
- The proc macro surface is smaller: `#[derive(Fsm)]`, `#[derive(State)]`,
  `#[resource(...)]`, `#[derive(Entity)]`, `#[event]`,
  `#[deferred]`, `#[usage]`, `#[derive(ResourceGroup)]`.
