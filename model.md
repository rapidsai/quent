# Query Engine Model for Profiling

> ## 🚧 WORK IN PROGRESS 🚧
>
> This specification is work in progress, incomplete, and probably contains
> various inconsistencies. If things that are already laid out imply
> consistency but are not consistent, please create an issue or reach out
> otherwise.

## Introduction

The goal of this document is to specify a model for telemetry obtained from
query engines for the purpose of profiling and performance analysis. An engine
model helps define semantic conventions for telemetry emitted from the engine
during operation.

This document consits of the following sections:

- [General definitions](#general-definitions)
- [Meta Model](#meta-model)
  - A definition of constructs necessary to construct a model of a specific
    engine. This is called the meta model (because it is a model of a model).
- [Concrete Model Requirements](#concrete-model-requirements)
  - A definition of model constructs that are required to be included by any
    concrete engine model.
- [Engine Model Construction Guidelines](#engine-model-construction-guidelines)
  - Guidelines and best practises of how engine architects can construct their
    own engine model.
- [Concrete Model Example](#concrete-model-example)
  - An example of how to construct a concrete engine model based on a fictional
    engine.
- [Telemetry](#telemetry)
  - A description of how telemetry and instrumentation may be derived from a
    concrete model.

### Document conventions

Names of constructs that are defined by this specification are intentionally
capitalized, e.g. [Entity](#entity), [Timestamp](#timestamp),
[Resource](#resource), etc.

---

## General definitions

### Attributes

An Attribute is a pair consisting of a key and a typed value.

In this specification, Attributes are denoted as follows:

- `<key name>: <value type>`.

This specification aims to describe only an absolute minimal set of Attributes
with a pre-defined meaning necessary to relate model constructs in a meaningful
way. Any concrete engine model and implementation of the instrumentation thereof
may choose to include arbitrary Attributes on any type of event (single events
or [Transitions](#transition) of [FSMs](#finite-state-machine)) as long as they
do not replace Attributes that have been assigned a specific meaning by this
specification.

#### Value types

Attribute values are of the following types.

##### Non-numeric primitive types

- Boolean (`bool`)
- [UUID](https://www.rfc-editor.org/rfc/rfc9562) (`uuid`)
- UTF-8 strings (`string`)

##### Numeric primitive types

- Unsigned integers of size 8, 16, 32, or 64 bits (`u{8,16,32,64}`)
- Signed integers of size 8, 16, 32, or 64 bits (`i{8,16,32,64}`)
- IEEE 754 floating-point values of types _binary32_ and _binary64_ (`f32`,
  `f64`)

##### Compound types

- Lists of variable lengths between $`[0, 2^64-1]`$ of exactly one of the above
  types, that may be empty (`list<T>` where `T` is one of the above)
- A set of attributes (`struct { ... }`)

Implementations may choose to explicitly provide an alias for variable-length
list of 8-bit unsigned integers (`list<u8>`) to capture binary data.

This specification explicitly forbids the use of architecture-specific
pointer-sized integers (such as `usize` in Rust, or (`s`) `size_t` in C++).

#### Keys

If constructs described in this specification allow having arbitrary run-time
defined [Attributes](#attributes), the names of arbitrary keys (of
engine-specific key-value pairs not defined by this speciciation) must be of the
type `string`.

Names of predefined keys shall use alphanumeric characters (A..Z, a..z, 0..9)
and underscores (`_`) only, starting with a non-digit.

#### Nullability

Atributes may be nullable, i.e. their value may not exist. To denote
nullability, this specification will denote such atrributes as `option<T>` where
`T` is the value type, or list them under a "may have" section.

### Time

#### Timestamp

Timestamps are 64-bit unsigned integers (`u64`) representing the amount of
nanoseconds passed since the Unix Epoch as defined in the
[POSIX](https://posix.opengroup.org/) standard (IEEE Std 1003.1-2024).

> Rationale: The choice of nanoseconds represented as `u64` bits allows timestamps
> to extend approximately $`584.6`$ average Gregorian years past the Unix Epoch.

#### Span

A Span (of time) consists of two [Timestamps](#timestamp):

- `start: Timestamp`: the beginning of some span of time
- `end: Timestamp`: the end of some span of time

The `end` [Timestamp](#timestamp) must be equal to or greater than the
`start` [Timestamp](#timestamp).

#### Duration

A Duration is the absolute difference between two [Timestamp](#timestamp) (
`u64`). A Duration always represents how much time has elapsed on a wall-clock.

#### Implementation-specific notes on capturing time

Implementations may be pratically limited in their methods to capture
[Timestamps](#timestamp). Due to such limitations, it may be that two events A
and B (where B is caused by A) have the exact same [Timestamp](#timestamp),
while in real time B occurs after A. If the implementation can guarantee that,
by construction B, must have occured after A, the implementation must capture
the order of these events in some way.

For example, in C++, one would typically employ `std::chrono::steady_clock` to
capture [Timestamps](#timestamp). However, `steady_clock` only guarantees it
does not decrease as time moves forward, but it does not guarantee that
subsequent calls increase the timestamp by at least one nanosecond. Whatever the
reason, causality must somehow be retained in the emitted telemetry.

Implementations are furthermore encouraged to consider and apply techniques to
mitigate clock skew, either during run-time or in post-processing, in case
[Timestamps](#timestamp) are captured from multiple distinct clock sources, e.g.
in the case of distributed engines.

---

## Meta Model

The meta model describes the most basic concepts that can be used to form a
_concrete_ model of an engine. These basic concepts are:

- [Entity](#entity)
- [FSM](#finite-state-machine), [State](#state) and [Transition](#transition)
- [Resource](#resource) and [Use](#use)
- [Resource Group](#resource-group)

These basic concepts are combined to form more elaborate concepts that exist
solely for convenience of building concrete models. Concrete engine models are
not required to use them. These include:

- [Memory](#memory) and [Allocation](#allocation)
- [Channel](#channel) and [Transfer](#transfer)
- [Processor](#processor)) and [Computation](#computation)

### Entity

An Entity is any discrete run-time construct that can be traced, measured, or in
some other way produce telemetry that is potentially useful to understand the
peformance characteristics of an engine.

Must have:

- `id: uuid`

Notes:

- Examples of things that can be modeled as [Entity](#entity) include objects,
  functions, threads, events, a PCIe-based host-to-device/device-to-host
  interface of a GPU, or logical CPU cores.

Rationale:

- Using UUIDs pratically prevents the need to synchronize between various
  producers of telemetry to produce unique identifiers.

#### Implementation-specific notes on entities

Implementations are recommended to use _UUIDv7_ as `id`, which includes a Unix
timestamp, which is useful to build indexes for fast analysis and search in
time ranges.

### Finite State Machine

A Finite State Machine (FSM) is an [Entity](#entity) with a set of
[States](#state) and possible [Transitions](#transition) between its
[States](#state).

#### State

Every State must have a name representable as a `string`.
Every State name is unique within the scope of an FSM.

An FSM must have at least two States, including the [Exit](#exit)
[State](#state).

##### Exit

The Exit State is a special reserved State into which a transition means the
[Entity](#entity) no longer exists. Its name is `exit`.

Every FSM must have the Exit state.

#### Transition

A Transition is a [Timestamp](#timestamp) upon which the FSM entered a new
[State](#state).

A Transition may be accompanied by [Attributes](#attributes).

Every FSM must be able to transition from some initial state into the
[Exit](#exit) state.

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

For brevity, when [State](#state) [Transitions](#transition) must follow a fixed
sequence, this is simplified as:

```text
⊙ -> initializing -> operating -> finalizing -> ⊗
```

### Resource

A Resource is an FSM with at least one or more associated [Capacity](#capacity).

Must have:

- `name: string`: the name of the Resource
- `scope: uuid`: the scope of the Resource, which the ID of the Entity across
  which this Resource is shared.

The scope must be an ID of one of:

- [Engine](#engine)
- [Worker](#worker)
- [Query Group](#query-group)
- [Query](#query)
- [Plan](#plan)
- [Operator](#operator)
- [Port](#port)
- [Resource Group](#resource-group)

There are four types of Resources, depending on their [Capacities](#capacity).

- [Unit](#unit-resource)
- [Fixed-Bounds](#fixed-bounds-resource)
- [Dynamic-Bounds](#dynamic-bounds-resource)
- [Unbounded](#unbounded-resource)

#### Capacity

A Capacity of a [Resource](#resource) is a named quantity that can be
exclusively claimed during some uninterrupted period of time via a [Use](#use).

A Capacity may or may not have bounds (minimum and/or maximum). Bounds can be
fixed for the lifetime of the [Resource](#resource) or change over time.

Must have:

- `name: string`
- a primitive numeric type to represent the amount of Capacity claimed by a
  [Use](#use).

#### Unit Resource

A Unit [Resource](#resource) has one unnamed dimensionless [Capacity](#capacity)
with bounds $`[0, 1]`$.

In other words, there can only be one [Use](#use) of the entire Resource
during some period of time.

#### Fixed-Bounds Resource

If a [Resource](#resource) provides only bounded [Capacities](#capacity) whose
bounds may not change during its lifetime, it is a Fixed-Bounds
[Resource](#resource).

#### Dynamic-Bounds Resource

If a [Resource](#resource) provides at least one bounded [Capacity](#capacity)
whose bounds may change during its lifetime, it is a Dynamic-Bounds
[Resource](#resource), for which the FSM is:

```text
⊙            -> initializing
initializing -> operating
operating    -> resizing
resizing     -> operating
operating    -> finalizing
finalizing   -> ⊗
```

The [Transition](#transition) into the `operating` state must convey the new
bounds for each [Capacity](#capacity).

#### Unbounded Resource

#### FSM of Unit, Fixed-Bounds, and Unbounded Resource

If all [Capacities](#capacity) of a [Resource](#resource) are unbounded, or if
no bounds of any [Capacity](#capacity) can change during the lifetime of the
[Resource](#resource), the FSM is:

```text
⊙ -> initializing -> operating -> finalizing -> ⊗
```

For the [Fixed-Bounds Resource](#fixed-bounds-resource), the
[Transition](#transition) into the `operating` state must convey the bounds for
each [Capacity](#capacity).

#### Notes on Resources and Capacities during concrete model construction

When constructing the model of an engine, it may be that there is a desire to
obtain telemetry from some resource which isn't represented in the engine
implementation by some abstraction with an explicit capacity.

For example, when using libraries that perform computations on e.g. a CPU or a
GPU across many threads, it may not be clear or trivial to obtain knowledge on
how many threads are actively being used during the computation. If this
information is unavailable, but there is a desire to still capture that this
computation happened related to some resource, then it is recommended to
introduce an unbounded resource in the model for which the computation takes up
some capacity of one. This way, spans or FSMs that perform this type of
computation can be grouped under this "resource" in post-processing and
visualization. Colloquially speaking, a concrete model of an engine can only
include things the engine implementation already knows - if it doesn't have
numbers on capacities, neither can its telemetry produce them. Traditional
profiling may need to be applied to uncover the used capacity. Future work (as
shown in the overview figure of [README.md](README.md)) aims to provide the
means to correlate outcomes of traditional profiling tools to the telemetry of
concrete engine models.

### Use

A Use represents an exclusive assignment of [Capacities](#capacity) of a
[Resource](#resource).

A Use must not outlive the `operating` (and `resizing`, if applicable) states of
the associated [Resource](#resource).

Must have:

- `resource_id: uuid`: the ID of the [Resource](#resource) being used

For each of the capacities of the resource, it must have:

- `used_<capacity name>: <capacity type>`: the amount of assigned capacity of
  the [Resource](#resource). Must be of the same numeric primitive type as the
  associated capacity of the [Resource](#resource).
- `<capacity name>` in the field name must be expanded to the specific capacity
  of the [Resource](#resource) that is being used
- `<capacity type>` must be of the same primitive numeric typ as the
  [Capacity](#capacity).

May have:

- `used_<capacity name>_effective`: the amount usage of the
  [Resource](#resource)'s capacity minus any overhead.

Any concrete Use must be combined with [Timestamps](#timestamp) such that
exactly one [Span](#span) of time may be derived representing the duration of
the Use.

#### Note on obtaining the Span of time of a Use from an FSM

One way of deriving the [Span](#span) of time of the Use is by by encapsulating
it in one or multiple [FSM](#finite-state-machine) [States](#state). In this
case, the required [Attributes](#attributes) of the Use must be captured by the
[Transition](#transition) into the [State](#state) spanning the Use of a
[Resource](#resource). This must be done in at least one [State](#state). This
may be done in multiple [States](#state) if the same Use outlives a single
[State](#state). In case the Use spans multiple states, the sequence of
[States](#state) must not be interrupted by [States](#state) in which the Use's
claim of the [Resource](#resource) associated [Capcity](#capacity) is released.

Additional notes:

- Examples of `used_<x>_effective` include: sizes of tables in a memory resource
  without padding or goodput bytes of a message over a network interface
  resource.
- Concrete models of Uses are recommended to include an [Attribute](#attributes)
  that relates to entities causing the Use, typically capturing the control flow
  of an engine. For example, the Use of a thread, say in a thread pool
  [Resource](#resource), may be performed on behalf of an asynchronous task
  entity related to an [Operator](#operator) related to a [Plan](#plan) related
  to a [Query](#query) related to a [Query Group](#query-group) related to an
  [Engine](#engine). This way, Uses of a [Resource](#resource) can ultimately be
  related and aggregated over certain levels/layers of control flow captured by
  the concrete model.

### Resource Group

A Resource Group is a [Resource](#resource) that represents a hierarchical
grouping over a set of other Resources.

FSM:

```text
⊙ -> initializing -> operating -> finalizing -> ⊗
```

Notes:

- A Resource Group of a finite set of [Unit Resources](#unit-resource) is a
  [Fixed-Bounds Resource](#fixed-bounds-resource).

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

#### Notes on concrete models using Allocation

Concrete engine models don't necessarily need to tie an Allocation to e.g. a
single `malloc`. For example, in a columnar query engine working with Arrow,
each underlying Arrow buffer would be a single `malloc`, yet in the model, an
Allocation can be tied to an entire worker-local "Table", capturing the sum of
all Arrow data and metadata buffer capacities.. Note that here the effective
part of the Allocation is the bytes of useful information within these buffers,
but the true use is the capacity of the buffers (which includes unused bytes and
padding). Choosing what an Allocation represents, as with all other concrete
model constructs, will be a trade-off between telemetry detail, run-time
overhead and storage.

### Channel

A Channel is a [Resource](#resource) responsible for transferring data between
two [Entities](#entity), typically but not necessarily a [Memory](#memory).

Must have:

- `source_id: uuid`: the ID of the Entities the Channel receives Datums from.
- `target_id: uuid`: the ID of the Entities the Channel sends Dataums to.

Notes:

- A Channel is unidirectional. Concrete models for engines with bidirectional
  channels need to be model them as two Channels.

### Transfer

A Transfer is a [Use](#use) of a [Channel](#channel).

### Processor

A Processor is a [Unit Resource](#unit-resource) responsible for computation.

### Computation

A Computation is a [Use](#use) of a [Processor](#processor).

---

## Concrete Model Requirements

This section describes mandatory requirements, mostly in the form of
[Entities](#entity) that must exist for valid concrete engine model.

> Rationale: The reason for having a minimal set of required constructs in a
> concrete engine model is that it provides a common basis for analysis tools
> to perform a basic set of useful analyses across different engine
> implementations, which can furthermore be used to compare engines.

### Engine

An Engine is an [FSM](#finite-state-machine) that holds
[Query Groups](#query-group) that execute [Queries](#query).

It represents the top-level entry point for every concrete engine model from
which all constructs of the model can be explored.

FSM:

```text
⊙ -> initializing -> operating -> finalizing -> ⊗
```

Must have:

- `name: string`: a name for this instance of the engine

Notes:

- This is an [FSM](#finite-state-machine) because resource allocation and
  deallocation in the initializing and finalizing state, respectively, can take
  significant amounts of time, and are thus potential candidates for performance
  optimizations.

### Query Group

A Query Group is an [FSM](#finite-state-machine) encapsulates a set of
[Queries](#query).

FSM:

```text
⊙ -> initializing -> operating -> finalizing -> ⊗
```

Must have:

- `engine_id: uuid`: the name of the [Engine](#engine) this
  [Query Group](#query-group) belongs to.

Notes:

- A Query Group can be used to e.g. represent sessions of a users running
  concurrent queries on multi-user engine instance, or a set of queries of a
  benchmark.

## Query

A Query is an [FSM](#finite-state-machine) representing the top-level unit of
work executed by an Engine, orchestrated through a Query Group.

FSM:

```text
⊙ -> initializing -> planning -> executing -> idle -> finalizing -> ⊗
```

Must have:

- `query_group_id: uuid`: the ID of the Query Group this Query belongs to

May have

- `statement: string`: a human-readable string representing the query statement.
  This can be e.g. the original SQL statement, the output of an `EXPLAIN`, etc.

## Plan

A Plan is a directed acyclic graph (DAG) where vertices are
[Operators](#operator) and edges represent data flowing between [Ports](#port)
of [Operators](#operator). [Operators](#operator) sink or source data, or
transform it. A Plan may have a child Plans, where the [Operators](#operator) of
a child Plan may be logically encapsulated by [Operators](#operator) of a parent
Plan, or vice versa. One Plan at the lowest level of a potential lineage of
plans is executed by one [Worker](#worker) on behalf of one [Query](#query).

FSM:

```text
⊙ -> initializing -> executing -> idle -> finalizing ⊗
```

Must have:

- `name: string`: The name of the Plan
- `query_id: uuid`: the ID of the [Query](#query) this is a Plan for
- `edges: list< struct{ source: uuid, target: uuid } >`: a list of edges where
  `source` is the ID of the Port producing data and `target` is the ID of the
  Port consuming data.

To form a valid Plan, at least one edge must exist for every
[Operator](#operator) of the Plan. Thus, a Plan always has at least two
[Operators](#operator).

May have:

- `worker_id: uuid`: the ID of the [Worker](#worker) this Plan has specifically
  executed on
- `parent_plan_id: uuid`: the ID of the parent Plan, if this Plan is a
  derivation or "lowering" of another Plan

Notes:

- The model does not explicitly make a distinction between a "logical" Plan, a
  physical "Plan", etc. because definitions and stages of lowering can differ
  wildly between engine implementations. Thus, there can be an arbitrary number
  of Plan transformations before arriving to an executable the lowest-level
  Plan.
- Engines that, at the same level of how it expresses and/or implements the
  plan, mix regular Operators with sequences of Plan Operators, e.g. to form
  "pipelines" or "stages", can potentially introduce a virtual Plan level to
  encapsulate such groupings in their model.
- There is no rule to restrict that multiple Plans with differing topologies may
  ultimately relate to a Query and may be executed by e.g. different Workers.
  This allows different types of Workers to execute different types of Plans.
- If at some level of data/control flow of an engine there is no explicit
  precense of a Plan, the instrumentation must still capture metrics that can be
  related back to the Plan. It is up to the implementation to ensure the proper
  contextual information is propagated (also known as context propagation).

## Operator

An Operator is an [FSM](#finite-state-machine) that sinks, sources, or transforms data.

FSM:

```text
⊙ -> initializing   -> waiting for inputs

waiting for inputs  -> waiting for inputs
waiting for inputs  -> executing
executing           -> waiting for inputs

executing           -> blocked
blocked             -> executing

executing -> idle -> finalizing -> ⊗
```

State definitions:

- `waiting for inputs`:
  - work on this operator cannot progress because it is waiting for input data.
  - has:
    - `ports: list<uuid>`: The IDs of the [Port](#port)s that this Operator is
      blocked on.
- `blocked`:
  - work on this operator cannot progress because it is blocked internally or at
    the output, e.g. by backpressure from another Operator consuming the output
    of this Operator.

Must have:

- `plan_id: uuid`: The ID of the [Plan](#plan) that this Operator belongs to.
- `ports: list<Port>`: A non-empty list of [Ports](#port) that this Operator
  has, where every `name` of each [Port](#port) is unique.

May have:

- `parent_operator_ids: list<uuid>`: The IDs of parent [Plan](#plan) Operators
  from which this Operator was derived.

> TODO(johanpel): The definition of attributes of the FSM transitions are
> likely very sensitive to implementation details of engines. Multiple
> engines should be studied to understand whether a generic set of
> attributes can be specified.
>
> TODO(johanpel): We could simply have one blocking state and have some
> enumeration of reasons, including waiting for inputs with data conveying
> which inputs.

### Port

A Port is an [Entity](#entity) that represents either an input or output of an
[Operator](#operator).

Must have:

- `name: string`: The name of the Port.

## Worker

A Worker is an [FSM](#finite-state-machine) responsible for the execution of a
[Plan](#plan) at the lowest level.

FSM:

```text
⊙ -> initializing -> operating -> finalizing -> ⊗
```

## Engine Model Construction Guidelines

This section described guidelines and best practises in the construction of
engine models using the meta model and required concrete model constructs from
the previous sections. This section is not strictly part of the model
specification.

### Relations

Concrete engine models must aim to define entities in such a way that they can,
possibly through several layers of indirection, related to an Engine.

## Concrete Model Example

> ## 🚧 WORK IN PROGRESS 🚧

This section will describe an example of a model of a contrived distributed
query engine called "Q".

### High-level description

Q can have multiple [Workers](#worker). Q defines two
[Plan](#plan) levels: a "logical" [Plan](#plan) and a "physical" [Plan](#plan).
Each [Worker](#worker) has an instance of a "physical" [Plan](#plan) with the
exact same topology.

Q is very simple. After performing a topological sort of the physical
[Plan](#plan), its scheduling thread visits every physical [Operator
[(#operator)] of the [Plan](#plan) and enqueues a single Task to a Thread Pool
that runs on a Thread until all work of that single Operator is completed.

While the Task is running on the Thread, it can load a RecordBatch from the
Filesystem, which represents a [Worker](#worker)-local partition of a table, and
spill any of its input to the Filesystem if it cannot get an Allocation for both
its inputs and worst-case sized outputs, while it keeps trying to
[Allocate](#allocation) [Memory](#memory) to write the output of some
[Computation](#computation). As such, Tasks running in Q can make room for other
concurrent Tasks, but if the sizes of their input and output RecordBatches
together would exceed total memory capacity, it will simply fail the query. It
may be best to not perform full outer joins on Q.

While the Task is running on the Thread, it can also split up a RecordBatch and
send it to [Memory](#memory) of another [Worker](#worker).

### Entities

#### Resources

##### Worker-scoped

- Filesystem: [Memory](#memory)
- MainMemory: [Memory](#memory)
- FsToMem: [Channel](#channel) between Filesystem and MainMemory
- MemToFs: [Channel](#channel) between MainMemory and Filesystem
- Task Thread: [Processor](#processor)
- Thread Pool: [Resource Group](#resource-group) of Task Threads

##### Engine-scoped

- Link: [Channel](#channel) between the MainMemory of two different Workers
- Network: [Resource Group](#resource-group) of Links

#### Control flow

##### Required by the model

- [Engine](#engine)
- [Query Group](#query-group)
- [Worker](#worker)
- [Query](#query)
- [Plan](#plan)
- [Operator](#operator)

#### Engine-specific

- RecordBatch (FSM)
  - Relates to:
    - Operator
  - The `idle` state claims an [Allocation](#allocation) in either Filesystem or
    MainMemory.
  - State transitions:
    ```text
    ⊙             -> initializing
    initializing  -> idle
    idle          -> moving
    moving        -> idle
    idle          -> finalizing
    finalizing    -> ⊗
    ```

- Task (FSM)
  - Relates to:
    - Operator
  - All states except `initializing`, `queueing`, and `finalizing` claim a
    [Computation](#computation) of one and the same Task Thread.
  - The `sending` state claims a [Transfer](#transfer) of a Link
  - The `loading` state claims a [Transfer](#transfer) of a FilesystemIO
  - State transitions:
    ```text
    ⊙             -> initializing
    initializing  -> queueing
    queueing      -> computing
    computing     -> allocating memory  -> computing
    computing     -> loading            -> computing
    computing     -> allocating storage -> spilling   -> computing
    computing     -> sending            -> computing
    computing     -> finalizing
    finalizing    -> ⊗
    ```

### Model relations

The lowest-level Entities of the model of Q are the Task and the RecordBatch.
A consistent model is able to relate any Entity all the way back to an Engine.

- For Task and RecordBatch, this can be done as follows:

```text
Task/RecordBatch -> Operator -> Plan (physical) -> Plan (logical) -> Query -> Query Group -> Engine
```

Note the above is not some FSM definition, but merely describes how construct
are related through their [Attributes](#attributes).

A consistent model also ensures all defined [Resources](#resource) have a
[Use](#use) somewhere, which in the case of the concrete model of Q:

```text
Task (computing, allocating memory/storage, loading, sending) -> Computation -> Task Thread -> Thread Pool -> Worker -> Engine
Task (loading) -> Transfer -> FilesystemIO -> Filesystem -> Worker -> Engine
Task (sending) -> Transfer -> Link -> Network -> Engine
RecordBatch (idle, moving) -> Allocation -> Memory / Storage -> Worker -> Engine
```

Because all Entities in the concrete model of Q can be related back to the
Engine, a relation graph virtually exists that connects all Engine concepts.

### Notes on Analysis

The concrete model of Q, when combined with telemetry capturing events that
provide data according to the model of Q, will allow answering many questions or
provide the means to visualize performance. Here are some examples provided in
the order in which an analyst may traverse through an interactive performance
analysis tool.

- Given an engine id, list all query groups named "tpc-h benchmark"
- Given the query grouyp id, list all queries named "21"

- Given a query id, show a DAG of the logical and physical [Plan](#plan)
- In the DAG of the logical [Plan](#plan), show the number of input and output
  rows for each [Port](#port) of an [Operator](#operator).
- In the DAG of the logical [Plan](#plan), show the average throughput of a Task
  sending data through the Network.
- In the DAG of the logical [Plan](#plan), color the [Operators](#operator) with
  colors from a colorblindness-friendly heatmap that corresponds to the number
  of bytes transfered trough the Network.
- In the DAG of the physical [Plan](#plan), color the [Operators](#operator)
  with colors from a colorblindness-friendly heatmap that corresponds to total
  amount of time spent in a Task Thread.
- In the DAG of the physical [Plan](#plan), show the maximum number of bytes
  claimed Memory Allocations.

- Given a query id, show a timeline of Tasks running on Thread Pool Threads,
  giving each Task state a unique colorblindness-friendly color.
- Given a query id, show a timeline with a Memory usage graph based on
  Allocations.
- etc.

Herein lies the power of a generic model for query engines - rather than N
engines implementing N performance analysis tools that roughly do the same
thing, there can be a much smaller set of performance analysis tools.

## Non-goals for this document

While the questions below are relevant for the project, they are not relevant
for this document because its sole purpose is to define how to construct a
performance model of query engines.

- How can lower-level profiling tools deliver low-level value under the semantic
  layer that a concrete model (and its telemetry) provides? Examples include
  CUPTI, AON, perf, etc.

## Telemetry

> ## 🚧 WORK IN PROGRESS 🚧

### Event

An Event is a single instance in time related to an [Entity](#entity).

Must have:

- `id`: the ID of the [Entity](#entity) producing this Event.
- `timstamp`: the [Timestamp](#timestamp) of this Event.

> TODO(johanpel): this is telemetry related, we can move this to a chapter 2.
>
> TODO(johanpel): find a good place for this section too. It's probably more useful
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

```text

```
