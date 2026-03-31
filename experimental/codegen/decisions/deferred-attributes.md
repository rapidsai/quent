# Decision: Deferred Attributes via Amendments with Sequence Numbers

## Context

Some applications need to add attributes to a state after the transition
occurs (e.g., values computed during the state). The question is when events
are emitted, how deferred attributes are expressed in the model, and how the
analyzer associates amendments with their parent transition.

## Decision

Events are emitted immediately on transition-in. Deferred fields are explicitly
marked with `#[quent::deferred]` and must be `Option<T>`. Setting a deferred
field emits an amendment event. Each event from an FSM instance carries a
per-instance sequence number for ordering and association.

## Model definition

```rust
#[quent::state]
pub struct Computing {
    #[quent::usage]
    pub thread: Usage<Thread>,
    #[quent::usage]
    pub memory: Usage<WorkerMemory>,
    #[quent::deferred]
    pub rows_processed: Option<u64>,
}
```

- `#[quent::deferred]` marks a field as settable after the transition.
- Deferred fields must be `Option<T>`. The proc macro enforces this.
- Non-deferred fields are required at transition time.

## Generated API

The transition method accepts only non-deferred fields and returns a state
handle. The handle exposes setters for deferred fields.

```rust
// seq 0: emits transition event immediately (rows_processed: None)
let state = task.transition(ComputingAttrs {
    thread: Usage { resource_id: tid, capacity: () },
    memory: Usage { resource_id: mid, capacity: MemoryCapacity { used_bytes: 4096 } },
});

// seq 1: emits amendment event (rows_processed: 1000)
state.set_rows_processed(1000);

// seq 2: emits next transition event
task.transition(SendingAttrs { /* ... */ });
```

If the value is not set before the next transition, `rows_processed` remains
`None`. The handle is invalidated on the next transition (compile-time
enforcement in Rust via move semantics, runtime check in C++).

## Sequence numbers

Each FSM instance maintains a monotonically increasing sequence counter
(starting at 0). Every event emitted by that instance (transitions and
amendments) gets the next sequence number. Events carry:

- `entity_id: Uuid`
- `sequence: u64`
- `timestamp: Timestamp`
- Event data (transition or amendment)

The analyzer groups events by `entity_id`, orders by `sequence`, and merges
amendments into the preceding transition event.

## Event stream example

```
entity_id: abc-123, seq: 0, ts: 100, Transition("computing", {thread: ..., memory: ..., rows_processed: None})
entity_id: abc-123, seq: 1, ts: 150, Amendment({rows_processed: 1000})
entity_id: abc-123, seq: 2, ts: 200, Transition("sending", {thread: ..., channel: ...})
```

## Rationale

- **Emit on transition-in**: events are available to the analyzer and exporters
  immediately. No buffering, no risk of losing pending state on crash.
- **Explicit `#[quent::deferred]`**: the model documents which fields are
  expected upfront vs. which come later. The proc macro generates different
  APIs for immediate vs. deferred fields.
- **Sequence numbers**: solve both the amendment association problem (the
  analyzer merges amendments into the preceding transition by sequence order)
  and the spec's causal ordering concern (sequence numbers are the tiebreaker
  when timestamps collide). Per-instance, not global, so the counter is just
  a u64 on the FSM handle.
