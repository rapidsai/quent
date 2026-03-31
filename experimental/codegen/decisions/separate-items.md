# Decision: Separate Items over Module-Level Macro

## Context

Proc macros can be applied at different granularities:

1. **Module-level macro** (`#[quent::model] mod my_model { ... }`): the proc
   macro sees the entire model at once and can cross-validate all items in a
   single pass.
2. **Per-item macros** (`#[quent::fsm]`, `#[quent::state]`, `#[quent::resource]`
   on individual structs): each macro processes one item independently.

## Decision

Per-item macros on separate structs.

## Rationale

### Cross-crate composition

Domain models (e.g., the query engine model defining Engine, Query, Plan,
Operator) are published as standalone crates. Application models import domain
model crates and add application-specific FSMs and resources.

A module-level macro requires all model items to be in one `mod` block. Items
from dependency crates cannot be placed inside another crate's module. This
makes cross-crate composition impossible.

Per-item macros work naturally across crate boundaries: the application crate
imports domain types with `use` and defines its own types alongside them.

### Cross-model validation still works

Cross-references between model items (e.g., `Usage<WorkerMemory>` in a state
struct referencing a resource) are validated by Rust's trait system, not by the
proc macro. `Usage<T>` requires `T: Resource`. If `WorkerMemory` is not
annotated with a resource macro, it does not implement `Resource`, and the
compiler emits an error. This works across crate boundaries without any macro
needing visibility over both items.

FSM-internal validation (reachability, exit convergence, transition
completeness) only requires visibility over the FSM's own transition table and
the referenced state types. The `#[quent::fsm]` macro has this.

### Standalone states allow sharing

States declared as standalone structs can be referenced by multiple FSMs if the
model requires it. A module-level macro would scope states to their enclosing
module, making sharing awkward.

### Tradeoff

Per-item macros cannot validate relationships that span multiple items within
the same crate without Rust's type system doing the work. In practice this is
not a limitation: the only cross-item relationships are resource usages, and
`T: Resource` covers that. If a future relationship type cannot be expressed
as a trait bound, a `quent::validate!()` macro or build script could be added
as a post-hoc validation step.
