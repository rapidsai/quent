# Decision: State Handle Borrows FSM Handle via &mut

## Context

When an FSM transitions into a state with deferred fields, the API returns a
state handle for setting those fields. The question is how the state handle
relates to the FSM handle in terms of ownership and borrowing.

## Decision

The state handle holds a `&mut` reference to the FSM handle. It is a thin
typed wrapper that delegates emission to the FSM handle.

## Design

```rust
pub struct ComputingHandle<'a> {
    fsm: &'a mut Task,
}

impl ComputingHandle<'_> {
    pub fn set_rows_processed(&self, value: u64) {
        self.fsm.emit_deferred(TaskDeferred::ComputingRowsProcessed(value));
    }
}
```

The FSM handle owns the entity ID, sequence counter, and context reference.
The state handle knows which deferred fields are valid for the current state
and provides typed setters.

## Borrow enforcement

While the state handle exists, the FSM handle is mutably borrowed. Calling
`task.transition()` while a state handle is alive is a compile error in Rust.
This prevents transitioning before deferred fields are finalized.

```rust
let state = task.transition(ComputingAttrs { ... });
// task.transition(SendingAttrs { ... });  // compile error: task is borrowed
state.set_rows_processed(1000);
drop(state);  // borrow ends
task.transition(SendingAttrs { ... });  // ok
```

When deferred fields are not needed, the handle is never bound and the borrow
ends immediately:

```rust
task.transition(ComputingAttrs { ... });  // handle dropped
task.transition(SendingAttrs { ... });    // fine
```

## C++ behavior

The `&mut` borrow does not cross the CXX bridge as a lifetime. In C++, the
state handle is a separate object and a runtime assertion on the FSM handle
checks that no active state handle exists when transitioning.

## Rationale

- The state handle needs access to the FSM handle's entity ID, sequence
  counter, and context reference to emit deferred events. Borrowing is simpler
  and cheaper than cloning or sharing these.
- The `&mut` borrow preventing the next transition while the state handle is
  alive is a free correctness guarantee enforced by the Rust compiler.
- FSM handles are attached to the objects they represent (e.g., a task in a
  query engine), which are typically much larger. The per-instance overhead of
  the handle (UUID, u64, reference) is negligible.
