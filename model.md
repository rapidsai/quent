# Query Engine Model for Profiling

> ## 🚧 WORK IN PROGRESS 🚧
>
> This specification is work in progress, incomplete, and probably contains
> various inconsistencies. If things that are already laid out imply
> consistency but are not consistent, please create an issue or reach out
> otherwise.

## Introduction

The goal of this document is to specify a generic (meta) model for query
engines. In turn, an engine model helps define semantic conventions for
telemetry emitted from the engine during operation. A more extensive rationale
behind this idea is described in the [README](README.md).

This document consits of three main sections:

1. A definition of constructs necessary to construct a model of a specific
   engine. This is called the meta model (because it is a model of a model).
2. A definition of model constructs that are required to be included by any
   specific engine model.
3. Guidelines, best practises and examples of how engine architects can
   construct their own engine model.

### Document conventions

Names of constructs that are defined by this specification are intentionally
capitalized, e.g. [Entity](#entity), [Timestamp](#timestamp),
[Resource](#resource), etc.

## General definitions

### Attributes

An Attribute is a pair consisting of a key and a value.

In the remainder of this specification. Required Attributes are listed under a
"must have" section under a section describing the construct they apply to.

#### Value types

Attribute values are of the following types.

##### Non-numeric primitive types

- Boolean (`bool`)

##### Numeric primitive types

- Unsigned integers of size 8, 16, 32, or 64 bits (`u[8,16,32,64]`)
- Signed integers of size 8, 16, 32, or 64 bits (`i[8,16,32,64`])
- IEEE 754 floating point values of types binary32 and binary64 (`f32`, `f64`)

##### Other types

- [UUID](https://www.rfc-editor.org/rfc/rfc9562) (`uuid`)
- UTF-8 strings (`string`)
- Lists of variable lengths between $`[0, 2^64-1]`$ of exactly one of the above
  types, that may be empty (`list<T>` where `T` is one of the above)
- A set of attributes (`struct { ... }`)

Implementations may choose to explicitly provide an alias for variable-length
list of 8-bit unsigned integers (`list<u8>`) to capture binary data.

This specification explicitly forbids the use of architecture-specific
pointer-sized integers (such as `usize` in Rust, or (`s`) `size_t` in C++).

#### Keys

If constructs allow capturing arbitrary Attributes, the names of arbitrary keys
(of engine-specific key-value pairs not defined by this speciciation) must be
UTF-8 strings.

Names of predefined keys shall use alphanumeric characters (A..Z, a..z, 0..9)
and underscores (`_`) only, starting with a non-digit.

#### Nullability

Atributes may be nullable, i.e. their value may not exist. To denote
nullability, this specification will denote such atrributes as `option<T>` where
`T` is the value type, or may list them under a "may have" section.

### Time

#### Timestamp

Timestamps are 64-bit unsigned integers (`u64`) representing the amount of
nanoseconds passed since the Unix Epoch as defined in the
[POSIX](https://posix.opengroup.org/) standard (IEEE Std 1003.1-2024).

Rationale:

- The choice of nanoseconds represented as `u64` bits allows timestamps to
  extend approximately 584.6 average Gregorian years past the Unix Epoch.

#### Span

A Span is a `struct` with two [Timestamps](#timestamp):

- `start: u64`: the beginning of some span of time
- `end: u64`: the end of some span of time

The `end` [Timestamp](#timestamp) must be equal to or greater than the
`start` [Timestamp](#timestamp).

> TODO(johanpel): this may be a bit redundant because a Span can also be
> defined as an FSM with a single state.

#### Duration

A Duration is the difference between two [Timestamp](#timestamp) (`u64`).
Thus, a Duration always represents how much time has elapsed on a wall-clock.

#### Implementation notes

Implementations may be pratically limited in their methods to capture
Timestamps. Due to such limitations, it may be that two events A and B (where B
is caused by A) have the exact same Timestamp, while in real time B occurs after
A. If the implementation can guarantee that, by construction B, must have
occured after A, the implementation must capture the order of these events in
some way.

For example, in C++, one would typically employ `std::chrono::steady_clock` to
capture timestamps. However, it only guarantees it does not decrease as time
moves forward, but it does not guarantee that subsequent calls increase the
timestamp.

> TODO(johanpel): we may want to spec this more instead of leaving it up to
> implementation, but since the problem is also implementation-specific,
> I am in doubt. Although most languages I know will have this problem as
> they're ultimately all grabbing timestamps through the OS pretty much the
> same way under water.
>
> TODO(johanpel): talk more about accuracy, precision, and clock skew, somewhere

## Meta Model

The meta model describes concepts that can be used to form a concrete model
of a specific engine implementation. These concepts may be combined to form
more elaborate concepts in the model.

### Entity

An Entity is anything that can be traced, measured, or in some other way produce
telemetry that is potentially useful to understand the peformance
characteristics of an engine.

Must have:

- `id: uuid`

Notes:

- Examples of things that can be modeled as [Entity](#entity) include objects,
  functions, threads, events, a PCIe-based host-to-device/device-to-host
  interface of a GPU, or logical CPU cores.
- Implementations are recommended to use UUIDv7 as `id`, which includes a Unix
  timestamp, which is useful to build indexes for fast analysis and search in
  time ranges.

Rationale:

- Using UUIDs pratically prevents the need to synchronize between various
  producers of telemetry to produce unique identifiers.

### Event

An Event is a single instance in time related to an [Entity](#entity).

Must have:

- `id`: the ID of the [Entity](#entity) producing this Event.
- `timstamp`: the [Timestamp](#timestamp) of this Event.

### Finite State Machine

A Finite State Machine (FSM) is an [Entity](#entity) modeled by a set of
[States](#state) and [Transitions](#transition).

#### State

A State must have a name representable as a `string`.

An FSM must not have more than one State with the same name.

The Exit State is a special State into which a transition means the
[Entity](#entity) no longer exists.

#### Transition

A Transition is an [Event](#event) conveying the new [State](#state) of the
[Entity](#entity).

#### Requirements

An FSM must have an Exit transition.
Implementations must name the Exit transition `exit`.

An FSM must have at least one [State](#state), excluding the Exit
[State](#state).

#### Notation

In the remainder of this document, specifying [States](#state) and their
[Transitions](#transition) is done as follows:

- `⊙ -> a`: transition into existence, with a initial state named `a`.
- `a -> b`: transition from state `a` to state `b`
- `b -> ⊗`: transition out of existence, with the final meaningful state named
  `b` and `⊗` denoting the special Exit state.

For example, an FSM can be described as follows, where each line denotes a
possible transition:

```text
⊙             -> initializing
initializing  -> operating
operating     -> finalizing
finalizing    -> ⊗
```

Note that in this example, while multiple [Transitions](#transition) mention the
same [State](#state), [States](#state) have unique names. Therefore, these
[Transitions](#transition) refer to the same [State](#state).

For brevity, when state transitions must follow a fixed sequence, this is
simplified as:

```text
⊙ -> initializing -> operating -> finalizing -> ⊗
```

### Resource

A Resource is an FSM with at least one or more associated [Capacity](#capacity).

A Resource must have:

- `name: string`: the name of the Resource

#### Capacity

A Capacity of a Resource is a named quantity that can be exclusively claimed
during some [Span](#span) via a [Use](#use).

A Capacity may have bounds (minimum and/or maximum). Bounds can be fixed for
the lifetime of the Resource or change over time.

There are four types of Resources:

- Unit
- Fixed-Bounds
- Dynamic-Bounds
- Unbounded

#### Unit Resource

A Unit Resource has one unnamed dimensionless Capacity with bounds $`[0, 1]`$.
In other words, there can only be one Use of the entire Resource during some
Span.

#### Fixed-Bounds Resource

If a Resource provides at least one bounded Capacity whose bounds may not change
during its lifetime, it is a Fixed-Bounds Resource (unless there is a
dynamically bounded Capacity, see
[Dynamic Bounds Resource](#dynamic-bounds-resource)).

#### Dynamic Bounds Resource

If a Resources provides at least one bounded Capacity whose bounds may change
during its lifetime, the FSM is:

```text
⊙            -> initializing
initializing -> operating
operating    -> resizing
resizing     -> operating
operating    -> finalizing
finalizing   -> ⊗
```

#### Unit, Fixed-Bounds, and Unbounded Resource FSM

If all Capacities of a Resource are unbounded, or if no bounds of any Capacity
can change during the Resource's lifetime, the FSM is:

```text
⊙ -> initializing -> operating -> finalizing -> ⊗
```

> TODO(johanpel): describe transition attribute that conveys bounds change

### Use

A Use is an Entity representing an exclusive assignment of capacities of a
[Resource](#resource).

A Use must not outlive the `operating` (and `resizing` if applicable) states of
the associated [Resource](#resource).

Must have:

- `<resource_name>_id: uuid`: the ID of the [Resource](#resource) being used
- `used_<capacity>`: the amount of assigned capacity of the
  [Resource](#resource). Must be of the same numeric primitive type as the
  associated capacity of the [Resource](#resource). `<capacity>` in the field
  name can be expanded to the specific capacity of the [Resource](#resource)
  that is being used.

May have:

- `used_<capacity>_effective`: the amount usage of the [Resource](#resource)'s
  capacity minus any overhead.

Any concrete Use must be combined with [Timestamps](#timestamp) such that
exactly one [Span](#span) may be derived representing the duration of the Use.

In other words, timing information about Uses
can be added by combining the Use with [Span](#span) attributes.

Another way of deriving the [Span](#span) of the Use is by by encapsulating it
in one or multiple [FSM](#finite-state-machine) [States](#state). In this case,
the required [Attributes](#attributes) must be captured by the
[Transition](#transition) into the [State](#state) which represents the active
Use of a [Resource](#resource). This must be done in at least one
[State](#state). This may be done in multiple [States](#state) if the same Use
outlives a single [State](#state). In case the Use spans multiple states, the
sequence of [States](#state) must not be interrupted by [States](#state) in
which the Use's claim of the [Resource](#resource) associated
[Capcity](#capacity) is released.

Notes:

- Examples of `used_<x>_effective` include: sizes of tables in a memory resource
  without padding or goodput bytes of a message over a network interface
  resource.
- Concrete Uses are recommended to also include an attribute that relates to
  entities causing the Use, typically capturing the control flow of an engine.
  For example, the Use of a thread, say in a thread pool [Resource](#resource),
  may be performed on behalf of an asynchronous task entity related to an
  [Operator](#operator) related to a [Plan](#plan) related to a [Query](#query)
  related to a [Coordinator](#coordinator) related to an [Engine](#engine). This
  way, Uses of a [Resource](#resource) can ultimately be related and aggregated
  over certain levels/layers of control flow captured by the model.

### Memory

A (fixed- or dynamically) bounded [Resource](#resource) with a
[Capacity](#capacity) of bytes of type `u64`.

Any [Transition](#transition) into the `operating` state must have:

- `capacity_bytes: u64`: the maximum amount of bytes that can be stored

### Allocation

A [Use](#use) of a [Memory](#memory) [Resource](#resource).

```text
⊙ -> allocating -> idle -> releasing -> ⊗
```

The [Transition](#transition) into the `idle` state must have:

`used_bytes: u64`: the number of bytes used from the [Memory](#memory).

Note:

- Concrete engine models don't necessarily need to tie an Allocation to e.g. a
  single `malloc`. For example, in a columnar query engine working with Arrow,
  each underlying Arrow buffer would be a single `malloc`, yet in the model, an
  Allocation can be tied to an entire worker-local "Table" (Datum), capturing
  the sum of all Arrow data and metadata buffer capacities. Note that here the
  effective part of the Allocation is the bytes of useful information within
  these buffers, but the true use is the capacity of the buffers (which includes
  unused bytes and padding).

### Channel

A Channel is a [Resource](#resource) responsible for transferring data between
two other [Resources](#resource). A Channel is unidirectional.

Must have:

- `source_id: uuid`: the ID of the Resource the Channel receives Datums from.
- `target_id: uuid`: the ID of the Resource the Channel sends Dataums to.

### Transfer

A Transfer is a [Use](#use) of a [Channel](#channel).

### Processor

A Processor is a [Unit Resource](#unit-resource) responsible for computation.

### Computation

A Computation is a [Use](#use) of a [Processor](#processor).

## Concrete Model Requirements

This section described mandatory requirements, mostly in the form of entities
that must exist for any specific engine model.

Rationale:

The reason for having a minimal set of required constructs in a concrete engine
model is that it provides a common basis for analysis tools to perform a basic
set of useful analyses across different engine implementations, which can
furthermore be used to compare engines.

### Engine

An Engine is an FSM that holds Coordinators that execute Queries.

FSM:

```text
⊙ -> initializing -> operating -> finalizing -> ⊗
```

May have:

- `name: string`: a name for this instance of an engine

> TODO(johanpel): many other attributes. As we integrate with different engines, we can back-annotate those that we found useful enough into the specification. Also see the [reference implementation](crates/entities/src/lib.rs).

Notes:

- An Engine is the top-level entry-point of an engine model.
- This is an FSM because resource allocation and deallocation in the
  initializing and finalizing state, respectively, can take significant amounts
  of time, and are thus potential candidates for performance optimizations.

### Coordinator

A Coordinator is an FSM responsible for the high-level orchestration of a set of
Queries on an Engine.

FSM:

```text
⊙ -> initializing -> operating -> finalizing -> ⊗
```

Must have:

- `engine_id: uuid`: the name of the Engine this Coordinator orchestrates
  Queries for.

Notes:

- A Coordinator groups sets of Queries. It can be used to e.g. represent
  sessions of multiple users running concurrent queries on the same engine
  instance.

> TODO(johanpel): this could also be named something like "session" or "query group", or something else that makes sense to group queries under.

## Query

A Query is an FSM representing the top-level unit of work executed by an Engine,
orchestrated through a Coordinator.

FSM:

```text
⊙ -> initializing -> planning -> executing -> idle -> finalizing -> ⊗
```

Must have:

- `coordinator_id: uuid`: the ID of the Coordinator this Query is executed on

Optional attributes:

- statement: a binary blob capturing any arbitrary data representing the query
  statement. This can be e.g. a UTF-8 SQL statement, a Substrait serialized
  binary Protobuf message, or some serialized form of a Polars or DataFusion
  dataframe that is to be lazily evaluated.

## Plan

A Plan is a directed acyclic graph (DAG) where vertices are Operators and Edges
represent data flowing between Operators. Operators sink or source data, or
transform it. A Plan may have a child Plans, where the Operators of a child Plan
may be logically encapsulated by Operators of a parent Plan. One Plan at the
lowest level of a potential lineage of plans is executed by one Worker on behalf
of one Query.

FSM:

```text
⊙ -> initializing -> executing -> idle -> finalizing ⊗
```

Must have:

- name
- a query ID
- a worker ID, if it is a lowest-level Plan

May have:

- a worker ID
- a parent plan

Notes:

- The model does not explicitly make a distinction between logical plan,
  physical plan, etc. because definitions can differ between engine
  implementations. There can be an arbitrary number of plan transformations
  before arriving to the lowest-level plan.
- There is no rule to restrict that multiple Plans with differing topologies may
  relate to a Query and may be executed by different Workers. This is done in
  order to allow different types of workers to execute different types of plans.
- The main purpose of the Plan in the Model is to capture metrics. Thus, even if
  at some level or instrumentation of an engine implementation there is no
  explicit DAG-like datastructure, the instrumentation must capture metrics
  adhering to this model by e.g. piggybacking the necessary information on top
  of the existing control flow information. Yet, it does intend to force any
  type of implementation of a DAG-like datastructure if this doesn't make sense
  for the control mechanism of a specific engine. To further clarify: one can
  imagine Workers being so simple that they just load data, perform a _single_
  Operator's work, and store their output, without being aware of the entire
  Plan. The Orchestrator must pass the necessary identifiers relating back to
  the Plan and Operators in the plan down to a Worker in order for it to capture
  its context when emitting telemetry.

## Operator

An Operator is an Entity that sinks, sources, or transforms data.

FSM:

```text
⊙ -> initializing -> waiting for inputs

waiting for inputs  -> waiting for inputs
waiting for inputs  -> executing
executing           -> waiting for inputs

executing           -> blocked
blocked             -> executing

executing -> idle -> finalizing -> ⊗
```

States:

- waiting for inputs:
  - work on this operator cannot progress because it is waiting for input data.
  - has:
    - Port ID
- blocked:
  - work on this operator cannot progress because it is blocked internally or at
    the output, e.g. by backpressure in a push-based execution mechanism
  - todo: define data for this state, we could capture various reasons

Must have:

- A parent Plan

May have:

- A parent Plan Operator: in case it is e.g. a hierarchical lowering or
  expansion of such a parent Operator.

Notes:

- At least one Edge (and thus one port) must be associated with an Operator.
- The definition of the FSM of this Entity is likely very sensitive to
  implementation details of engines. Multiple engines should be studied to
  understand whether it can generally match. There are various alternatives such
  as deferring the detection of waiting for inputs and blocked states to
  post-processing / analysis.

## Port

A Port is an Entity that represents either an input or output of an Operator.

Must have:

- A parent Operator
- Whether it is a source or sink
- Source or sink port ID

Metrics:

- Input rows
- Input bytes
- Output rows
- Output bytes

Note:

- Port telemetry can piggyback onto Operator state transitions telemetry. E.g.
  the metrics would typically be propagated with the `executing --> idle`
  transition.

## Worker

A worker is an Entity responsible for the execution of a Plan at the lowest
level.

FSM:

```text
⊙ -> initializing -> operating -> finalizing -> ⊗
```

## Engine Model Construction Guidelines

This section described guidelines and best practises in the construction of
engine models using the meta model and required concrete model constructs from
the previous sections. This section is not strictly part of the model
specification.

> TODO

### Relations

Concrete engine models must aim to define entities in such a way that they can,
possibly through several layers of indirection, related to an Engine.

## Concrete Model Example

> TODO(johanpel): provide a minimal example of a concrete model, ideally
> one that is consistent with the simulator

## Telemetry

> TODO(johanpel): find a good place for this section. It's probably more useful
> in a place where the reference implementation is described versus in the model
> spec.

Traditionally, there are three types of telemetry:

- logs: captures single events associated with a timestamp and holds
  (un)structured data
- metrics: captures a sequence of values associated with a timestamp (a
  timeseries)
- traces: captures a tree of spans of time, typically with names associated with
  function calls of a program, in order to trace the call stack

This project defines, and leans heavily on, a fourth type:

- finite state machines (FSM): captures the state of things, and transitions
  between those states associated with a timestamp

The idea of adding a fourth type is that it makes it easier to track the state
and evolution of resources that come into and go out of existence during the
lifetime of a program, without having to necessarily trace the call stack
related to these things.
