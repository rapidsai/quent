# Decision: No Standalone DSL

## Context

A custom DSL (e.g., `.quent` files with dedicated syntax) was considered for
model definitions. This would provide a language-agnostic definition format
with potentially more concise syntax than any host language.

## Decision

No standalone DSL. Model definitions use Rust with proc macro annotations.

## Rationale

### Marginal syntax benefit

A DSL for FSM definitions would look like:

```
fsm task {
    state queueing(operator_id: uuid, instance_name: string)
    state computing(use_thread: uuid, use_memory_bytes: u64)
    entry -> queueing
    queueing -> computing
    computing -> exit
}
```

The Rust equivalent is:

```rust
#[quent::fsm]
pub struct Task {
    #[quent::transition(entry -> Queueing)]
    #[quent::transition(Queueing -> Computing)]
    #[quent::transition(Computing -> exit)]
}

#[quent::state]
pub struct Computing {
    pub use_thread: Uuid,
    pub use_memory_bytes: u64,
}
```

The Rust version is slightly more verbose but not meaningfully harder to read
or write.

### Infrastructure cost

A standalone DSL requires:

- A parser with error reporting
- IDE/editor support (syntax highlighting, diagnostics)
- Documentation for the syntax
- A build system integration story for each target language

Rust proc macros provide all of this for free: the Rust compiler handles
parsing and error reporting, IDEs already support Rust, and Cargo handles
the build integration.

### Audience feedback

At least one prospective user of this system has expressed a preference against
DSLs. The Rust annotation approach is familiar to Rust developers and readable
(as annotated structs) to non-Rust developers.

### Not precluded

If a DSL proves valuable later, it can be added as a frontend that parses
`.quent` files and produces the same in-memory model representation. This does
not require changing the codegen architecture. The Rust definition path remains
the primary and always-supported path.
