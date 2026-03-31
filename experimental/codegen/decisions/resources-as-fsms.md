# Decision: Resources Are Standard Library FSM Definitions

## Context

Resources have spec-defined lifecycles (initializing → operating → finalizing →
exit). Previously, dedicated annotations (`#[quent::memory]`,
`#[quent::processor]`, `#[quent::channel]`) were proposed to generate these
FSMs. The question is whether resources need special code generation or can
use the same FSM mechanism as everything else.

## Decision

Memory, Processor, Channel, and other common resource types are ordinary FSM
definitions shipped in a standard library crate. No special resource-specific
annotations or code generation. A single `#[quent::resource]` marker on an FSM
adds the `Resource` trait so `Usage<T>` works.

## Standard library definitions

```rust
// In quent-stdlib

#[quent::fsm]
#[quent::resource]
pub struct Memory {
    #[quent::transition(entry -> Initializing)]
    #[quent::transition(Initializing -> Operating)]
    #[quent::transition(Operating -> Finalizing)]
    #[quent::transition(Finalizing -> exit)]
}

#[quent::state]
pub struct Initializing;

#[quent::state]
pub struct Operating {
    pub capacity_bytes: u64,
}

#[quent::state]
pub struct Finalizing;
```

Dynamic-bounds resources add the resizing cycle:

```rust
#[quent::fsm]
#[quent::resource]
pub struct DynamicMemory {
    #[quent::transition(entry -> Initializing)]
    #[quent::transition(Initializing -> Operating)]
    #[quent::transition(Operating -> Resizing)]
    #[quent::transition(Resizing -> Operating)]
    #[quent::transition(Operating -> Finalizing)]
    #[quent::transition(Finalizing -> exit)]
}
```

Processor (unit resource):

```rust
#[quent::fsm]
#[quent::resource]
pub struct Processor {
    #[quent::transition(entry -> Initializing)]
    #[quent::transition(Initializing -> Operating)]
    #[quent::transition(Operating -> Finalizing)]
    #[quent::transition(Finalizing -> exit)]
}

#[quent::state]
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

`#[quent::resource]` on an FSM generates a `Resource` trait impl. The capacity
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

- `#[quent::memory]` — replaced by `quent_stdlib::Memory` FSM
- `#[quent::processor]` — replaced by `quent_stdlib::Processor` FSM
- `#[quent::channel]` — replaced by `quent_stdlib::Channel` FSM

The only resource-related annotation is `#[quent::resource]`, which can be
applied to any FSM to mark it as usable with `Usage<T>`.

## Rationale

- One mechanism for everything. Resources and application FSMs use the same
  `#[quent::fsm]` and `#[quent::state]` annotations. No special cases in the
  proc macro.
- Standard library FSMs are reusable definitions, not framework magic. An
  application can define its own resource FSM with custom states if the
  standard ones don't fit.
- The proc macro surface is smaller: `#[quent::fsm]`, `#[quent::state]`,
  `#[quent::resource]`, `#[quent::entity]`, `#[quent::event]`,
  `#[quent::deferred]`, `#[quent::usage]`, `#[quent::resource_group]`.
