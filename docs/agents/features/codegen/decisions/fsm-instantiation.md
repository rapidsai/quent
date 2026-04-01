# Decision: One Handle per FSM Instance

## Context

An FSM instance needs an entity ID, a sequence counter, and deferred field
state. The question is whether these are managed by a per-instance handle or
externally (e.g., one shared observer with IDs passed per call).

## Decision

One handle per FSM instance. The handle owns the entity ID, the sequence
counter, and any pending deferred fields.

## API

### Rust

```rust
// Creates a new FSM instance with auto-generated UUIDv7
// Emits the entry transition event (seq 0)
let task = Task::new(&ctx, QueueingAttrs {
    operator_id: op_id,
    instance_name: "scan_0".into(),
});

// task.id() returns the entity ID
let id = task.id();

// Transition (seq 1)
let state = task.transition(ComputingAttrs { thread: ..., memory: ... });
state.set_rows_processed(1000); // seq 2

// Transition (seq 3)
task.transition(SendingAttrs { ... });

// Exit (seq 4), consumes the handle
task.exit();
```

### C++ (via CXX bridge)

```cpp
auto task = Task::create(ctx, Queueing{.operator_id = op_id, .instance_name = "scan_0"});

auto id = task->id();

auto state = task->transition(Computing{.thread = ..., .memory = ...});
state->set_rows_processed(1000);

task->transition(Sending{/* ... */});
task->exit();
```

## Handle ownership

- **Rust**: `Task::new()` returns an owned `Task`. `exit()` takes `self`
  (move), consuming the handle. Using the handle after `exit()` is a
  compile error.
- **C++**: `Task::create()` returns `rust::Box<Task>`. Rust owns the memory.
  `exit()` consumes the box. Using the handle after `exit()` is a runtime
  error (null pointer or assertion).

## Entity ID

Auto-generated UUIDv7 at creation time. Accessible via `id()` so the
application can reference the FSM instance (e.g., to set it as a parent
resource group, or to reference it from another entity's attributes).

## What the handle holds

- `id: Uuid` — entity ID
- `seq: u64` — sequence counter, incremented on every emission
- `ctx: &Context` — reference to the instrumentation context (event sender)
- Pending deferred fields for the current state (if any)

## Rationale

- The handle carries per-instance state (sequence counter, deferred fields).
  Managing this externally (keyed by ID in a map) would be more complex.
- One handle per instance is the natural model for an FSM — it represents a
  single entity moving through states.
- Move semantics in Rust enforce that `exit()` ends the handle's lifetime.
  This prevents use-after-exit at compile time.
