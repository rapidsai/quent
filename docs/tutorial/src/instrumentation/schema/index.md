# Schema

An Application Event Schema describes the event data that an application can
emit. Quent uses it to generate an application-specific, typed instrumentation
API.

## Entities

An **entity** is anything you want to emit events about. It typically represents
a control-flow or data-flow object in your code, such as a task, request,
buffer, or worker. It can also represent a function call, a metric source, or
another event-producing concept.

Each entity instance has a universally unique identity represented by a UUID,
which keeps its events separate from events emitted by other instances.
Processes can assign these identities independently without coordinating through
a central allocator, shared counter, or global process state.

Entities exist to group related events around the thing they describe.

## Events

An **event** is something that happens to an entity at a particular point in
time, such as a task starting or ending.

For each event, the generated instrumentation API provides a named call that
application code uses to emit it. Depending on the target language, this call
may be exposed as a method or function.

In statically typed target languages, these generated calls are fully
type-safe. The compiler checks that each event receives the expected number and
types of attributes. This makes it harder for instrumentation changes to
accidentally alter event semantics or break downstream analysis.

In this respect, Quent resembles structured logging: each event has a known
name and typed data. Quent can also export events through an end-to-end
statically typed pipeline. Exporters do not necessarily need to attach runtime
type information to each value, which can reduce runtime work and improve
performance.

Events exist to record how an entity behaves over time.

## Attributes

An **attribute** is a typed value captured when an event occurs, such as a task
name or result code. Each attribute becomes a typed argument to the generated
event call.

Attributes exist to record the details needed to interpret an event.

## Records

A **field** is a named, typed value inside a record. A **record** is a named,
reusable group of fields. The generated API represents a record as a `struct`
in Rust and C++. In Python, records are dictionaries with generated `TypedDict`
type hints.

Records exist to keep related values together and avoid repeating the same
field definitions.

## Event cardinality

**Event cardinality** defines how often an event can occur for one entity
instance. A `once` event occurs at most once. A `multi` event may occur
repeatedly.

Cardinality exists to distinguish unique events from repeatable events.

## Events and performance analysis

While Quent provides generated instrumentation libraries for emitting events,
event collection and event analysis are separate concerns. Quent's analysis
does not depend on how events were collected, as long as they are represented
by the schema and an adapter can load them for analysis. Other sources could
include statistical profilers, instrumentation-based profilers, [NVTX] events,
CUDA API calls captured through [CUPTI], [OpenTelemetry] signals, or [eBPF]
probes.
These are examples of possible sources, not a list of currently available Quent
adapters.

[NVTX]: https://nvidia.github.io/NVTX/
[CUPTI]: https://docs.nvidia.com/cupti/
[OpenTelemetry]: https://opentelemetry.io/docs/
[eBPF]: https://ebpf.io/what-is-ebpf/
