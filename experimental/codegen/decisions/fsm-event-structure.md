# Decision: Common FsmEvent Wrapper with Sequence Numbers

## Context

FSM events need per-instance sequence numbers for ordering transitions and
deferred field updates. The question is whether sequence numbers belong on the
core `Event<T>` type or within the FSM-specific event payload.

## Decision

Sequence numbers live in a common `FsmEvent<S, D>` wrapper, not on the core
`Event<T>`. Each generated FSM event type is a type alias over this wrapper.

## Definition

```rust
// In quent-events or quent-model
pub enum FsmEvent<S, D> {
    Transition { seq: u64, state: S },
    Deferred { seq: u64, deferred: D },
}
```

## Generated types per FSM

```rust
// Generated for Task
pub enum TaskTransition {
    Queueing(Queueing),
    Computing(Computing),
    Allocating(Allocating),
    Loading(Loading),
    Spilling(Spilling),
    Sending(Sending),
    Exit,
}

pub enum TaskDeferred {
    ComputingRowsProcessed(u64),
    // one variant per deferred field across all states
}

pub type TaskEvent = FsmEvent<TaskTransition, TaskDeferred>;
```

## Rationale

- **Sequence numbers are FSM-specific.** Entity events (one-shot) and resource
  events do not need sequencing. Adding `seq` to the core `Event<T>` would
  waste space on non-FSM events and conflate concerns.
- **Common wrapper enables generic analysis.** The analyzer can operate on
  `FsmEvent<S, D>` generically — extracting sequence numbers and
  distinguishing transitions from deferred updates without knowing the
  concrete FSM type.
- **Naming consistency.** `#[quent::deferred]` in the model definition,
  `Deferred` in the event variant, `TaskDeferred` in the generated enum.
