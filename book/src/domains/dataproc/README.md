# Domain-specific model for data processing / query engines

> TODO: This is VERY much work-in-progress

This section describes mandatory requirements, mostly in the form of
[Entities][entity] that must exist for valid engine model.

> Rationale: The reason for having a minimal set of required constructs in a
> engine model is that it provides a common basis for analysis tools
> to perform a basic set of useful analyses across different engine
> implementations, which can furthermore be used to compare engines.

## Engine

An Engine is an [FSM][finite-state-machine] that holds
[Query Groups][query-group] that execute [Queries][query].

It represents the top-level entry point for every engine model from
which all constructs of the model can be explored.

FSM:

```text
⊙ -> initializing -> operating -> finalizing -> ⊗
```

Must have:

- `name: string`: a name for this instance of the engine

Notes:

- This is an [FSM][finite-state-machine] because resource allocation and
  deallocation in the initializing and finalizing state, respectively, can take
  significant amounts of time, and are thus potential candidates for performance
  optimizations.

## Query Group

A Query Group is an [FSM][finite-state-machine] encapsulates a set of
[Queries][query].

FSM:

```text
⊙ -> initializing -> operating -> finalizing -> ⊗
```

Must have:

- `engine_id: uuid`: the ID of the [Engine][engine] this
  [Query Group][query-group] belongs to.

Notes:

- A Query Group can be used to e.g. represent sessions of a users running
  concurrent queries on multi-user engine instance, or a set of queries of a
  benchmark.

## Query

A Query is an [FSM][finite-state-machine] representing the top-level unit of
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
[Operators][operator] and edges represent data flowing between [Ports][port]
of [Operators][operator]. [Operators][operator] sink or source data, or
transform it. A Plan may have a child Plans, where the [Operators][operator] of
a child Plan may be logically encapsulated by [Operators][operator] of a parent
Plan, or vice versa. One Plan at the lowest level of a potential lineage of
plans is executed by one [Worker][worker] on behalf of one [Query][query].

FSM:

```text
⊙ -> initializing -> executing -> idle -> finalizing ⊗
```

Must have:

- `name: string`: The name of the Plan
- `query_id: uuid`: the ID of the [Query][query] this is a Plan for
- `edges: list< struct{ source: uuid, target: uuid } >`: a list of edges where
  `source` is the ID of the Port producing data and `target` is the ID of the
  Port consuming data.

To form a valid Plan, at least one edge must exist for every
[Operator][operator] of the Plan. Thus, a Plan always has at least two
[Operators][operator].

May have:

- `worker_id: uuid`: the ID of the [Worker][worker] this Plan has specifically
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

An Operator is an [FSM][finite-state-machine] that sinks, sources, or transforms
data.

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
    - `ports: list<uuid>`: The IDs of the [Port][port]s that this Operator is
      blocked on.
- `blocked`:
  - work on this operator cannot progress because it is blocked internally or at
    the output, e.g. by backpressure from another Operator consuming the output
    of this Operator.

Must have:

- `plan_id: uuid`: The ID of the [Plan][plan] that this Operator belongs to.
- `ports: list<Port>`: A non-empty list of [Ports][port] that this Operator has,
  where every `name` of each [Port][port] is unique.

May have:

- `parent_operator_ids: list<uuid>`: The IDs of parent [Plan][plan] Operators
  from which this Operator was derived.

> TODO(johanpel): The definition of attributes of the FSM transitions are
> likely very sensitive to implementation details of engines. Multiple
> engines should be studied to understand whether a generic set of
> attributes can be specified.
>
> TODO(johanpel): We could simply have one blocking state and have some
> enumeration of reasons, including waiting for inputs with data conveying
> which inputs.

## Port

A Port is an [Entity][entity] that represents either an input or output of an
[Operator][operator].

Must have:

- `operator_id`: The ID of the [Operator][operator] this port belongs to.
- `name: string`: The name of the Port.

## Worker

A Worker is an [FSM][finite-state-machine] responsible for the execution of a
[Plan][plan] at the lowest level.

FSM:

```text
⊙ -> initializing -> operating -> finalizing -> ⊗
```

## Application-specific model construction guidelines

... for data processing engines.

> 🚧 WORK IN PROGRESS 🚧

This section described guidelines and best practises in the construction of
engine models using the meta model and required model constructs from
the previous sections. This section is not strictly part of the model
specification.

## Relations

Engine models must aim to define entities in such a way that they can,
possibly through several layers of indirection, related to an Engine.

[engine]: #engine
[entity]: ../../modeling/entity.md
[finite-state-machine]: ../../modeling/fsm.md
[operator]: #operator
[plan]: #plan
[port]: #port
[query]: #query
[query-group]: #query-group
[worker]: #worker
