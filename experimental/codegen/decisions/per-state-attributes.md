# Decision: Transition Attributes Are Per-State

## Context

The spec says transitions may carry attributes. The question is whether
attributes are defined per state (all transitions into a state carry the same
struct) or per (from, to) pair (different source states can carry different
data).

## Decision

Per-state. Each state struct defines the attributes for all transitions into
that state. When a field is only relevant for transitions from certain source
states, it is declared as `Option<T>`.

## Example

```rust
#[derive(State)]
pub struct Computing {
    #[usage]
    pub thread: Usage<Thread>,
    #[usage]
    pub memory: Usage<WorkerMemory>,
    pub allocation_time_ns: Option<u64>,  // only set when coming from Allocating
}
```

## Rationale

- One struct per state keeps the model simple. No combinatorial explosion of
  attribute types for states reachable from many sources.
- Optional fields are a natural and well-understood mechanism for variance.
- The generated API accepts the same struct regardless of source state. The
  caller sets or omits optional fields as appropriate.
- The analyzer and UI always know the shape of a state's data without needing
  to know which transition produced it.
