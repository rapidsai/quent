# Decision: Proc Macro Generates All Boilerplate Layers

## Context

Defining a single FSM currently requires ~480 lines of hand-written code across
six layers:

1. Event type enum (e.g., `TaskEvent` with variants per state)
2. Observer emission methods (e.g., `TaskObserver::task_computing(...)`)
3. Analyzer builder (collecting transitions, constructing the FSM)
4. `Fsm` trait impl (entity identity, transition indexing)
5. `FsmTypeDeclaration` impl (state/transition metadata for the UI)
6. Usage impl (mapping states to resource capacity claims)

The question is whether the proc macro should generate all six layers or leave
some for manual implementation.

## Decision

Generate all six layers. The `#[quent::fsm]` and `#[quent::state]` annotations
produce the complete instrumentation and analysis implementation.

## Rationale

With the `Usage<T>` design, resource usage mappings are declared directly on
state structs. The mapping from state attributes to resource capacity values,
which was previously custom logic (e.g., `create_usages()` in the simulator
analyzer), is now fully determined by the model definition. There is no
remaining application-specific logic in any of the six layers.

## Escape hatch

If an application encounters an edge case the proc macro cannot express, the
escape hatch is to not use `#[quent::fsm]` on that specific FSM and write the
impls manually using the underlying traits and types, which remain public. The
proc macro is a convenience, not mandatory.

## Open concern: deferred attributes

Some applications need to add attributes to a state after the transition
occurs (e.g., values computed during the state that are logically part of the
transition event). This means the generated API cannot simply emit events
on transition. It must return a handle to the current state that accumulates
attributes, with the event finalized on the next transition or explicit flush.

This affects the shape of the generated instrumentation API and the C++ runtime.
Details to be resolved in the API design for state handles and deferred
attribute emission.
