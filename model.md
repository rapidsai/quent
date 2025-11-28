# Query Engine Model for Profiling

## Terms
- WCT: Wall-Clock Time, nanoseconds passed since the Unix epoch
- *For Now*: clear sources of technical debt, mostly implementation-driven limitations, but decided as such due to implementation workload constraints

## Entity (meta)

Anything that can be traced, measured, or in some other way produce useful telemetry.

Must have:
- [UUID](https://www.rfc-editor.org/rfc/rfc9562)

Notes:
- Examples of implementation artifacts that can be modeled as Entity include an object, function, or event.
- Using UUIDs (ideally v7 which includes a Unix timestamp) practically ensures no coordination is required between any systems to generate the identifiers.

## Finite State Machine (meta)

A Finite State Machine (FSM) is an Entity modeled by a set of states and transitions.
Each transition has a timestamp (WCT).
Each transition may be accompanied by data.

## Engine

An Engine is an Entity that holds Coordinators that execute Queries.

FSM:
```
⊙ → initializing → operating → finalizing → ⊗
```

Must have:
- Name

## Coordinator

A Coordinator is an Entity responsible for the high-level orchestration of a Query on an Engine.

FSM:
```
⊙ → initializing → operating → finalizing → ⊗
```

Must have:
- Engine ID

## Query (FSM)

A Query represents the top-level unit of work executed on a Coordinator.

FSM:
```
⊙ → initializing → planning → executing → idle → finalizing → ⊗
```

Must have:
- Coordinator ID

May have:
- statement: a binary blob capturing any arbitrary data representing the query statement. This can be e.g. a UTF-8 SQL statement, a Substrait serialized binary Protobuf message, or some serialized form of a Polars or DataFusion dataframe that is to be lazily evaluated.

For now: 
- The statement binary blob should aim to be small. This is to prevent OTel over gRPC with default configs to not exceed the default max message size of 4 MiB. Note that multiple telemetry events will be batched into a single message, hence we want to keep this under say 1 MiB.

## Plan

A Plan is a directed acyclic graph (DAG) where vertices are Operators and Edges represent data flowing between Operators.
Operators sink or source data, or transform it.
A Plan may have a child Plans, where the Operators of a child Plan may be logically encapsulated by Operators of a parent Plan.
One Plan at the lowest level of a potential lineage of plans is executed by one Worker on behalf of one Query.

FSM:
```
⊙ → initializing → executing → idle → finalizing ⊗
```

Must have:
- name
- a query ID
- a worker ID, if it is a lowest-level Plan

May have:
- a worker ID
- a parent plan

Notes:
- The model does not explicitly make a distinction between logical plan, physical plan, etc. because definitions can differ between engine implementations. There can be an arbitrary number of plan transformations before arriving to the lowest-level plan.
- There is no rule to restrict that multiple Plans with differing topologies may relate to a Query and may be executed by different Workers. This is done in order to allow different types of workers to execute different types of plans.
- The main purpose of the Plan in the Model is to capture metrics. Thus, even if at some level or instrumentation of an engine implementation there is no explicit DAG-like datastructure, the instrumentation must capture metrics adhering to this model by e.g. piggybacking the necessary information on top of the existing control flow information. Yet, it does intend to force any type of implementation of a DAG-like datastructure if this doesn't make sense for the control mechanism of a specific engine. To further clarify: one can imagine Workers being so simple that they just load data, perform a *single* Operator's work, and store their output, without being aware of the entire Plan. The Orchestrator must pass the necessary identifiers relating back to the Plan and Operators in the plan down to a Worker in order for it to capture its context when emitting telemetry.

## Operator

An Operator is an Entity that sinks, sources, or transforms data.

FSM:
```
⊙ → initializing → waiting for inputs

waiting for inputs → waiting for inputs
waiting for inputs → executing
executing → waiting for inputs

executing → blocked
blocked → executing

executing → idle → finalizing → ⊗
```

States:
- waiting for inputs: 
  - work on this operator cannot progress because it is waiting for input data.
  - has:
    - Port ID
- blocked: 
  - work on this operator cannot progress because it is blocked internally or at the output, e.g. by backpressure in a push-based execution mechanism
  - todo: define data for this state, we could capture various reasons

Must have:
- A parent Plan

May have: 
- A parent Plan Operator: in case it is e.g. a hierarchical lowering or expansion of such a parent Operator.

Notes: 
- At least one Edge (and thus one port) must be associated with an Operator.
- The definition of the FSM of this Entity is likely very sensitive to implementation details of engines. Multiple engines should be studied to understand whether it can generally match. There are various alternatives such as deferring the detection of waiting for inputs and blocked states to post-processing / analysis.

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
- Port telemetry can piggyback onto Operator state transitions telemetry. E.g. the metrics would typically be propagated with the `executing --> idle` transition.

## Worker

A worker is an Entity that executes lowest-level Plans.

FSM:
```
⊙ → initializing → operating → finalizing → ⊗
```

## Resource (meta)
A Resource is an Entity with at least one bounded quantity.
The quantity and bounds can change over time.

## Use (meta)
A Use of a Resource

## Memory
A spatial resource holding bytes.

Examples:
- Memory Pool

Has:
- lifetime (bounded by engine lifetime)
- capacity (the maximum amount that could be stored).
- utilization (the number of stored bytes)

## Allocation
A reservation

## Interface
Any type of resource transferring bytes over time.

Examples:
- H2D / D2H

Can have a lifetime.

## Compute Resource
## Compute Span
## Interface Resource

## General TODOs
- A lot, but just to have a place to quickly park thoughts
- Wherever there are byte counts, we may want to have the option to have both compressed and uncompressed counts.
