# Query Engine Model for Profiling

> ## 🚧 WORK IN PROGRESS 🚧
>
> This specification is work in progress, very incomplete, and may contain various
> inconsistencies.

## Introduction

The goal of this document is to specify a generic (meta) model for query
engines. In turn, the model helps define semantic conventions for telemetry
emitted from query engines. These semantic conventions make it simpler for
performance analysis tools and their human users to evaluate performance of an
engine.

### Note on telemetry

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

### Conventions

Names of constructs that are defined by this specification are intentionally
capitalized, e.g. Entity, Timestamp, Resource, etc.

## Attributes

An Attribute is a key-value pair.

In the remainder of this specification. Required attributes are listed under a
"must have" section under a section describing the construct they apply to.

### Value types

Attribute values are of the following types.

#### Non-numeric primitive types

- Boolean (`bool`)

#### Numeric primitive types

- Unsigned integers of size 8, 16, 32, or 64 bits (`u[8,16,32,64]`)
- Signed integers of size 8, 16, 32, or 64 bits (`i[8,16,32,64`])
- IEEE 754 floating point values of types binary32 and binary64 (`f32`, `f64`)

#### Other types

- [UUID](https://www.rfc-editor.org/rfc/rfc9562) (`uuid`)
- UTF-8 strings (`string`)
- Lists of variable lengths between `$0..2^64-1$` of exactly one of the above
  types, that may be empty (`list<T>` where `T` is one of the above)
- A set of attributes (`struct { ... }`)

Implementations may choose to explicitly provide an alias for variable-length
list of 8-bit unsigned integers (`list<u8>`) to capture binary data.

This specification explicitly forbids the use of architecture-specific
pointer-sized integers (such as `usize` in Rust, or (`s`) `size_t` in C++).

### Keys

If constructs allow capturing arbitrary Attributes, the names of arbitrary keys
(of engine-specific key-value pairs not defined by this speciciation) must be
UTF-8 strings.

Names of predefined keys shall use alphanumeric characters (A..Z, a..z, 0..9)
and underscores (`_`) only, starting with a non-digit.

### Nullability

Atributes may be nullable, i.e. their value may not exist. To denote
nullability, this specification will denote such atrributes as `option<T>` where
`T` is the value type, or may list them under a "may have" section.

## Time

### Timestamp

Timestamps are 64-bit unsigned integers (`u64`) representing the amount of
nanoseconds passed since the Unix Epoch as defined in the
[POSIX](https://posix.opengroup.org/) standard (IEEE Std 1003.1-2024).

Rationale:

- The choice of nanoseconds represented as `u64` bits allows timestamps to
  extend approximately 584.6 average Gregorian years past the Unix Epoch.

### Span

A Span is a `struct` with two Timestamps:

- `start: u64`: the beginning of some span of time
- `end: u64`: the end of some span of time

The `end` Timestamp must be equal to or greater than the `start` Timestamp.

### Duration

A Duration is the difference between two Timestamps (`u64`).
Thus, a Duration always represents wall-clock time.

### Implementation notes

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

> TODO(johanpel): we may want to spec this more instead of leaving it up to implementation, but since the problem is also implementation-specific, I am in doubt. Although most languages I know will have this problem as they're ultimately all grabbing timestamps through the OS pretty much the same way under water.
>
> TODO(johanpel): talk more about accuracy, precision, and clock skew, somewhere

## Meta Model

The meta model describes constructs that can be used to form a concrete model of
a specific engine implementation.

### Entity

An Entity is anything that can be traced, measured, or in some other way produce
telemetry that is potentially useful to understand the peformance
characteristics of an engine.

Must have:

- `id: uuid`

Notes:

- Examples of things that can be modeled as Entity include objects, functions,
  threads, events, a PCIe-based host-to-device/device-to-host interface of a
  GPU, or logical CPU cores.
- Implementations are recommended to use UUIDv7 as `id`, which includes a Unix
  timestamp, which is useful to build indexes for fast analysis and search in
  time ranges.

Rationale:

- Using UUIDs pratically prevents the need to synchronize between various
  producers of telemetry.

### Finite State Machine

A Finite State Machine (FSM) is an Entity modeled by a set of states and
transitions.

In the remainder of this document, specifying states and their transitions is
done as follows:

- `⊙ -> a`: transition into existence, with a initial state named `a`.
- `a -> b`: transition from state `a` to state `b`
- `b -> ⊗`: transition out of existence, with the final state named `b`.

For example, an FSM can be described as follows, where each line denotes a
possible transition.

```text
⊙             -> initializing
initializing  -> operating
operating     -> finalizing
finalizing    -> ⊗
```

For brevity, if the state transitions must follow a fixed sequence, this is
simplified to:

```text
⊙ -> initializing -> operating -> finalizing -> ⊗
```

A Transition must have:

- `timstamp`: the moment in time when the transition occured

### Resource

A Resource is an FSM with at least one quantity expressing the
exclusive utilization of that quantity through a Use.

The quantity may have bounds.

FSM:

```text
⊙ -> initializing -> operating -> finalizing -> ⊗
```

A Use must occur within the `operating` state of the Resource.

### Use

A Use represents an exclusive allocation of specific quantities of a Resource.

Must have:

- `resource_id: uuid`: the ID of the Resource being used
- `used_<x>`: the amount of usage of the Resource's quantity. Can be of any
  numeric primitive type. `<x>` in the field name can be expanded to the
  specific quantity of the Resource that is being used.

May have:

- `used_<x>_effective`: the amount usage of the Resource's quantity minus any
  overhead.

Any concrete type of Use must be combined with timing information from which at
least one Span may be derived. In other words, timing information about Uses can
be added by combining a Use with a Span or FSM. In case it is combined with an
FSM, the required Attributes must be captured by at least one state transition.

Notes:

- Examples of `used_<x>_effective` include: sizes of tables in a memory resource
  without padding or goodput bytes of a message over a network interface
  resource.
- Concrete Uses are recommended to also include an attribute that relates to
  entities owning the use, typically capturing the control flow of an engine.
  For example, the Use of a thread resource, say in a thread pool resource, may
  be performed on behalf of an asynchronous task entity related to an Operator
  related to a Plan related to a Query related to a Coordinator related to an
  Engine. This way, resource utilization can ultimately be related and
  aggregated over certain levels of control flow captured by the model.

### Memory

A spatial Resource of bytes.

FSM:

```text
⊙ -> initializing -> operating -> finalizing -> ⊗
```

The `operating` state must have:

- `capacity` (the maximum amount that could be stored).

> TODO(johanpel): figure out how we're going to allow updating the bounds,
> possibly with a resizing state?

### Allocation

A Use of a Memory Resource.

Must have:

`used_bytes: u64`: the number of bytes used by the resource

```text
⊙ -> allocating -> idle -> releasing -> ⊗
```

Note:

- This isn't necessarily tied to e.g. a single `malloc`. For example, in a
  columnar query engine working with Arrow, each underlying Arrow buffer would
  be a single `malloc`, yet in the model, an Allocation can be tied to an entire
  worker-local "Table" (Datum), capturing the sum of all Arrow data and metadata
  buffer capacities. Note that here the effective part of the Allocation is the
  bytes of useful information within these buffers, but the true use is the
  capacity of the buffers (which includes unused bytes and padding).

### Datum

A Datum is an Allocation representing some (grouping of) data local to a Worker.

Example:

- An Arrow RecordBatch
- An IPC message

### Channel

A Channel is a Resource responsible for transferring a Datum.

### Transfer

A Transfer is a Span and a Use of a Channel.

### ComputeUnit

> TODO(johanpel): this may need a better name

A ComputeUnit is a Resource that has a dimensionless utilizable quantity of zero
or one. Therefore, only one Use of a ComputeUnit can exist at a time.

Notes:

- Examples include a Span+Use combination of a thread running on a thread pool

### Compute

A Compute is a Use of a ComputeUnit.

## Concrete Model Requirements

This section described mandatory requirements, mostly in the form of entities
that must exist for any specific engine model.

### Engine

An Engine is an FSM that holds Coordinators that execute Queries.

FSM:

```text
⊙ -> initializing -> operating -> finalizing -> ⊗
```

May have:

- `name: string`: a name for this instance of an engine

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

## Concrete Model Guidelines

> TODO(johanpel): this section is going to describe how to make analysis tooling
> understand the employment of meta model constructs in concrete engine models.
