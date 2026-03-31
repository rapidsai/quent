# Decision: Entities Emit Freestanding One-Shot Events

## Context

Plain entities (not FSMs, not resources) need to emit telemetry. Examples
include Operator (declaration + statistics), Plan (declaration with edges),
Engine (initialization metadata). These are not state machines — they have no
lifecycle transitions.

The question is how entity events are modeled and what API the proc macro
generates.

## Decision

Entities and their events are freestanding items linked by an `entity`
attribute. An entity's own struct fields define its declaration event. Additional
event types are separate structs annotated with `#[event]` fields.

All entity events are one-shot: a single timestamped emission, not a lifecycle.

## Example

```rust
#[derive(Entity)]
pub struct Operator {
    pub plan_id: Uuid,
    pub type_name: String,
    #[event]
    statistics: OperatorStatistics,
}

pub struct OperatorStatistics {
    pub rows_processed: u64,
    pub bytes_read: u64,
}
```

## Generated API

```rust
// Declaration: emits the entity's fields as an event, returns a handle
let op = Operator::declare(&ctx, OperatorAttrs { plan_id, type_name });

// Follow-up event: emits additional data linked to the same entity ID
op.emit(OperatorStatistics { rows_processed: 1000, bytes_read: 4096 });
```

The handle carries the entity ID and a context reference. `emit()` accepts any
event type linked to the entity via `#[event]` fields on the Entity struct. The
proc macro validates that only declared event types can be emitted for a given
entity.

## Rationale

- Entity declarations and follow-up events are structurally independent. They
  share an entity ID but have different schemas. Keeping them as separate types
  reflects this.
- The analyzer relates events by entity ID at runtime, not by structural
  coupling in the model definition. This matches how the analyzer already works.
- Entities that previously had init/exit spans (Engine, Worker) can derive their
  spans from the events of their children instead of needing their own lifecycle.
  If an explicit span is needed, it can be modeled as an FSM instead.
- The `#[event]` field annotation lets the proc macro enforce at
  compile time that events are only emitted for the correct entity type.
