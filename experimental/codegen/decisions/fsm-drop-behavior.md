# Decision: Auto-Emit Exit on Drop

## Context

The spec requires every FSM to ultimately reach the exit state. If an FSM
handle is dropped without calling `exit()`, the FSM would violate this
requirement.

## Decision

`Drop` automatically emits the exit event if `exit()` was not called
explicitly. Explicit `exit()` is also available.

## Design

```rust
impl Task {
    pub fn exit(&mut self) {
        if !self.exited {
            self.emit_exit();
            self.exited = true;
        }
    }
}

impl Drop for Task {
    fn drop(&mut self) {
        self.exit();
    }
}
```

- Calling `exit()` explicitly emits the exit event and marks the handle.
- If the handle is dropped without `exit()`, `Drop` emits it automatically.
- Calling `exit()` multiple times is a no-op after the first.

## Rationale

- Every FSM reaches exit, satisfying the spec.
- No silent leaks — the analyzer always sees a complete FSM lifecycle.
- No panics — forgetting `exit()` is handled gracefully, not treated as a
  fatal error.
- Explicit `exit()` is available for when the developer wants to control the
  exact timestamp of the exit event or make the intent clear in code.
