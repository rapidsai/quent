# Telemetry

Instrumented applications emit events. This section defines the event model
and explains why Quent uses FSMs and Resources rather than existing telemetry
frameworks.

## Events

An Event is a single instant in time accompanied by arbitrary information.

Must have:

- `timestamp: Timestamp`: the [Timestamp][timestamp] of this Event.
- At least one additional [Attribute][attribute] carrying information about
  what occurred.

## Entity Events

An Event emitted on behalf of an [Entity][entity] must have:

- `id: uuid`: the ID of the [Entity][entity] producing this Event.

## FSM Events

The events of [FSMs][fsm] represent [Transitions][transition].

Because an FSM is an [Entity][entity], each of its [Transition][transition]
[Events][event] must have the following [Attributes][attribute]:

- `id: uuid`: the ID of the [FSM][fsm]
- `timestamp: Timestamp`: the moment in time upon which the [FSM][fsm]
  transitioned into the next [State][state]

### Implementation restrictions

Implementations are free to choose the mechanism by which the next
[State][state] is conveyed. It is recommended to provide types for distinct
[Transition][transition] [Events][event] in order to promote type-safety in the
instrumentation API.

## Implementation-specific notes on capturing time

Implementations may be practically limited in their methods to capture
[Timestamps][timestamp]. Due to such limitations, it may be that two events A
and B have the exact same [Timestamp][timestamp],
while in real time B occurs after A. If the implementation can guarantee that,
by construction, B must have occurred after A, the implementation must capture
the order of these events in some way.

For example, monotonic clocks on many platforms only guarantee non-decreasing
values, not that subsequent calls produce distinct timestamps. Whatever the
reason, causality must somehow be retained in the emitted telemetry.

Implementations are furthermore encouraged to consider and apply techniques to
mitigate clock skew, either during run-time or in post-processing, in case
[Timestamps][timestamp] are captured from multiple distinct clock sources, e.g.
in the case of distributed applications.

## Rationale for the FSM and Resource Concepts

This project introduces FSMs as a modeling primitive on top of timestamped
events. An FSM transition is an event, but the FSM structure adds two things
that plain events (or logs) lack:

1. **Implicit spans of time.** Each FSM state spans from its entry transition
   to the next transition. Developers instrument only transitions; the
   durations of states are derived automatically without requiring explicit
   begin/end pairs.
2. **Structural constraints.** The declared set of states and allowed
   transitions forms a schema. Invalid transitions can be detected, and
   analysis tools can reason about what states an entity can be in without
   application-specific code.

FSMs are well-suited for tracking things that concurrently come into and go
out of existence during the lifetime of a program. Unlike tracing spans, FSM
entities are not organized into trees; they are flat, independent state
machines that relate to each other through explicit attributes (e.g. a Task
FSM references an Operator by ID). This avoids the need for implicit context
propagation and lets developers define the relationships that matter to them
directly.

This project also introduces explicit Resource modeling: named, capacity-bound
things that can be occupied or saturated. Resources with declared capacities
enable analysis tools to automatically compute utilization, detect saturation,
and visualize resource pressure over time, without application-specific logic.

Traditional metrics (e.g. "memory usage at time T") are snapshots: they
capture a value but not the events that produced it. Because resource
utilization in this project is derived from FSM transitions and Usage events,
any aggregate value (e.g. total bytes allocated) can be traced back to the
individual state transitions and entity lifecycles that contributed to it.

Combined, FSMs and Resources form a structured schema (an application model)
from which visualizations of control structure states and resource utilization
can be automatically derived.

### Why not OpenTelemetry?

OpenTelemetry (OTel) is a widely adopted and valuable observability framework.
However, this project has specific requirements that benefit from a different
approach:

- **Partial model recovery.** OTel tracing implementations typically export
  spans only after they are closed. If a program crashes or a query fails
  mid-flight, in-progress spans are lost. FSM transitions are emitted
  individually as they occur. If a failure happens, the model can be partially
  reconstructed from whatever transitions were already emitted.
- **Static typing.** OTel's data model relies on string-keyed attribute bags
  with runtime type information. The modeling concepts in this project use
  statically typed, schema-driven events. This enables compile-time guarantees
  on the instrumentation API and avoids the overhead of runtime type dispatch
  in both the instrumentation and analysis paths.
- **Full control over transport and encoding.** This project's concepts are
  defined independently of any particular telemetry framework. OTel Logs, for
  instance, could serve as an underlying layer to carry FSM transition events,
  but that is an implementation choice, not a requirement. By not coupling to
  a specific data model, the project is free to choose encodings and transports
  (e.g. Protobuf over gRPC, MessagePack, postcard, Arrow IPC) that best fit the
  performance and deployment constraints of the target application.

[attribute]: ./modeling/attributes.md
[entity]: ./modeling/entity.md
[event]: #events
[fsm]: ./modeling/fsm.md
[state]: ./modeling/fsm.md#state
[timestamp]: ./modeling/time.md#timestamp
[transition]: ./modeling/fsm.md#transition
