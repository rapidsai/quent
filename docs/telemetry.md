# Telemetry

## Events

An Event is a single instant in time accompanied by arbitrary information.

An Event consists of at least two [Attribute][attribute].

Must have:

- `timestamp: Timestamp`: the moment in time upon which this
  [Timestamp][timestamp] of this Event.

## Entity Events

Event emitted on behalf of an [Entity][entity] must have:

- `id: uuid`: the ID of the [Entity][entity] producing this Event.

## FSM events

The events of an [FSMs][fsm] represent [Transitions][transition].

Because an FSM is an [Entity][entity], each of its [Transition][transition]
[Events][event] must have the following [Attributes][attribute]:

- `id: uuid`: the ID of the [FSM][fsm]
- `timstamp: Timestamp`: the moment in time upon which the [FSM][fsm]
  transitioned into the next [State]

### Implementation restrictions

Implementations are free to choose the mechanism by which the next
[State][state] is conveyed. It is recommended to provide types for distinct
[Transition][transition] [Events][event] in order to promote type-safety in the
instrumentation API.

> TODO: we're not using this next part yet, so it is irrelevant for now, but
> in the future, it may be useful if this would be e.g. layered onto something
> else, like OTel Logs:
>
> If implementations choose to use an [Attribute][attribute] to convey the next
> [State][state], then that [Attribute][attribute] shall be:
>
> - `state: string`: the name of the [State][state] that the [FSM][fsm]
>   transitioned into.

## Implementation-specific notes on capturing time

Implementations may be pratically limited in their methods to capture
[Timestamps][timestamp]. Due to such limitations, it may be that two events A
and B have the exact same [Timestamp][timestamp],
while in real time B occurs after A. If the implementation can guarantee that,
by construction B, must have occured after A, the implementation must capture
the order of these events in some way.

For example, in C++, one would typically employ `std::chrono::steady_clock` to
capture [Timestamps][timestamp]. However, `steady_clock` only guarantees it
does not decrease as time moves forward, but it does not guarantee that
subsequent calls increase the timestamp by at least one nanosecond. Whatever the
reason, causality must somehow be retained in the emitted telemetry.

Implementations are furthermore encouraged to consider and apply techniques to
mitigate clock skew, either during run-time or in post-processing, in case
[Timestamps][timestamp] are captured from multiple distinct clock sources, e.g.
in the case of distributed applications.

## Rationale for the FSM and Resource Telemetry Concepts

Traditionally, there are three types of telemetry:

- logs: captures single events associated with a timestamp and holds
  (un)structured data
- metrics: captures a sequence of values associated with a timestamp (a
  timeseries)
- traces: captures a tree of spans of time, typically with names associated with
  function calls of a program, in order to trace the call stack

This project defines, and leans heavily on, a fourth opinionated type:

- finite state machines (FSM): captures the state of things, and transitions
  between those states associated with a timestamp

The idea of adding a fourth type is that it makes it easier to track the state
and evolution of things that concurrently come into and go out of existence
during the lifetime of a program, without having to necessarily trace the call
stack.

With respect to traces, modern software systems have an incredibly complicated
call stack, where "work" is not just represented as a tree of nested functions,
but often (temporarily) transformed to data structures (e.g. by pushing work
descriptors into a queue that feeds some asynchronous execution engine, and vice
versa.

Causal relationships between executed pieces of code are therefore, at
_a certain level of abstraction in our human minds_, often not thought to follow
the call stack, and aren't necessarily trees. Those that add telemetry
instrumentation to their applications based on tracing are therefore required to
explicitly propagate contextual information in addition to the implicit context
propagation often baked in by tracing libraries, which may arguably cause some
degree of cognitive dissonance. In other words, the restrictions imposed by the
fact traces are trees and their implicit propagation of context as such
can actually feel like getting in the way of obtaining a clear picture of all of
the many concurrently moving parts in a complex system.

This project furthermore adds an explicit description of the concept of
resources, as things that can get saturated and that can be scarce, because this
is what developers trade off when implementing certain classes of applications
(high-performance, resource-constrained).

Combined with the concept of FSMs, the idea is that clear but fully
developer-driven graphs (application models) of how things relate to eachother
can easily be constructed. From these graphs, visual overviews can be built from
which the developer can quickly observe the state of control structures and the
resources that they utilize. In this way, developers themselves can define what
they find important to see in the user interface of a performance analysis /
profiling tool.

[attribute]: ./modeling/attributes.md
[entity]: ./modeling/entity.md
[event]: #events
[fsm]: ./modeling/fsm.md
[state]: ./modeling/fsm.md#state
[timestamp]: ./modeling/time.md#timestamp
[transition]: ./modeling/fsm.md#transition
