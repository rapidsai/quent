# Event Model

This section defines the event model and the rationale behind FSMs and
Resources as modeling primitives.

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

[attribute]: ./modeling/attributes.md
[entity]: ./modeling/entity.md
[event]: #events
[fsm]: ./modeling/fsm.md
[state]: ./modeling/fsm.md#state
[timestamp]: ./modeling/time.md#timestamp
[transition]: ./modeling/fsm.md#transition
