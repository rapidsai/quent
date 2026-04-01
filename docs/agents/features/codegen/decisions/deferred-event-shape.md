# Decision: Flat Deferred Event Enum

## Context

The `FsmEvent<S, D>` wrapper needs a deferred type `D` that represents all
possible deferred field updates across all states of an FSM. The question is
whether variants are flat (one per field) or nested (grouped by state).

## Decision

Flat. One variant per deferred field, named `{StateName}{FieldName}`.

## Example

```rust
pub enum TaskDeferred {
    ComputingRowsProcessed(u64),
    LoadingBytesTransferred(u64),
    LoadingChecksum(Option<u32>),
}
```

The payload type is the field's inner type (unwrapped from `Option`), except
when the field itself is `Option<Option<T>>` (rare — the outer Option is the
deferred marker, the inner is the actual optionality).

## FSM with no deferred fields

If an FSM has no deferred fields on any state, the deferred type is an empty
enum (uninhabitable), and the `FsmEvent::Deferred` variant can never be
constructed:

```rust
pub enum TaskDeferred {}
pub type TaskEvent = FsmEvent<TaskTransition, TaskDeferred>;
```

## Rationale

- Simpler than nested grouping — one level of variants.
- The analyzer merges by sequence number, not by variant structure, so
  grouping provides no analytical benefit.
- The naming convention `{StateName}{FieldName}` avoids collisions when
  different states have fields with the same name.
