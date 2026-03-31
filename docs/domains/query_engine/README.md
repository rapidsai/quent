# Domain-specific model for query engines

This section describes requirements, mostly in the form of [Entities][entity]
that must exist for a valid engine model.

> Rationale: The reason for having a minimal set of required constructs in an
> engine model is that it provides a common basis for analysis tools
> to perform a basic set of useful analyses across different engine
> implementations, which can furthermore be used to compare engines.

## Engine

An Engine is an [Entity][entity] that holds [Query Groups][query-group] that
execute [Queries][query].

It represents the top-level entry point for every engine model from
which all constructs of the model can be explored. An Engine also acts as the
root [Resource Group][resource-group] (with no parent).

An Engine tracks a start and end time (a [Span][span]).

Must have:

- `instance_name: string`: a name for this instance of the engine

May have:

- Implementation attributes (name, version, custom attributes)

## Query Group

A Query Group is an [Entity][entity] that encapsulates a set of
[Queries][query].

Must have:

- `engine_id: uuid`: the ID of the [Engine][engine] this
  [Query Group][query-group] belongs to.

Notes:

- A Query Group can be used to e.g. represent sessions of users running
  concurrent queries on a multi-user engine instance, or a set of queries of a
  benchmark.

## Query

A Query is an [FSM][finite-state-machine] representing the top-level unit of
work executed by an Engine. A Query belongs to a [Query Group][query-group].

FSM:

```text
⊙ -> init -> planning -> executing -> ⊗
```

Must have:

- `query_group_id: uuid`: the ID of the Query Group this Query belongs to

## Plan

A Plan is a directed acyclic graph (DAG) where vertices are
[Operators][operator] and edges represent data flowing between [Ports][port]
of [Operators][operator]. [Operators][operator] sink or source data, or
transform it. A Plan may have child Plans, where the [Operators][operator] of
a child Plan may be logically encapsulated by [Operators][operator] of a parent
Plan, or vice versa. One Plan at the lowest level of a potential lineage of
plans is executed by one [Worker][worker] on behalf of one [Query][query].

A Plan is not an FSM; it is declared once and does not have lifecycle states.
Its topology is fixed at declaration time. Timing information for a Plan is
typically derived from FSMs that reference work performed on behalf of the Plan
(e.g. task FSMs that carry the Plan's ID).

Must have:

- `instance_name: string`: The name of the Plan
- `edges: list< struct{ source: uuid, target: uuid } >`: a list of edges where
  `source` is the ID of the Port producing data and `target` is the ID of the
  Port consuming data.

Edges connect [Ports][port] of different [Operators][operator]. An edge from a
source Port of Operator A to a target Port of Operator B represents data
flowing from A to B. A Plan may have zero edges (e.g. a single-operator plan).

[Mutually exclusive][mutual-exclusion]:

- `query_id: uuid`: the ID of the [Query][query], if this is a root Plan
- `parent_plan_id: uuid`: the ID of the parent Plan, if this is a derived
  (or "lowered") Plan. The Query is reachable by traversing up the parent
  chain.

May have:

- `worker_id: uuid`: the ID of the [Worker][worker] this Plan has specifically
  executed on

Notes:

- The model does not explicitly make a distinction between a "logical" Plan, a
  physical "Plan", etc. because definitions and stages of lowering can differ
  wildly between engine implementations. Thus, there can be an arbitrary number
  of Plan transformations before arriving at the lowest-level executable Plan.
- Engines that, at the same level of how they express and/or implement the
  plan, mix regular Operators with sequences of Plan Operators, e.g. to form
  "pipelines" or "stages", can potentially introduce a virtual Plan level to
  encapsulate such groupings in their model.
- There is no rule to restrict that multiple Plans with differing topologies may
  ultimately relate to a Query and may be executed by e.g. different Workers.
  This allows different types of Workers to execute different types of Plans.
- If at some level of data/control flow of an engine there is no explicit
  presence of a Plan, the instrumentation must still capture metrics that can be
  related back to the Plan. It is up to the implementation to ensure the proper
  contextual information is propagated (also known as context propagation).

## Operator

An Operator is an [Entity][entity] that sinks, sources, or transforms data
within a [Plan][plan].

Operators emit two types of events:

- **Declaration**: emitted when the operator is created, establishing its
  identity and membership in a plan. May include initial attributes and
  statistics.
- **Statistics**: emitted after execution, carrying post-execution metrics
  (e.g. rows processed, bytes read/written).

Must have:

- `plan_id: uuid`: The ID of the [Plan][plan] that this Operator belongs to.
- `type_name: string`: The type name of the Operator.

May have:

- `parent_operator_ids: list<uuid>`: The IDs of parent [Plan][plan] Operators
  from which this Operator was derived.
- `custom_attributes`: Arbitrary attributes.
- `statistics`: Custom attributes emitted post-execution containing operator
  metrics.

Notes:

- Operators intentionally do not have an FSM. Execution states (e.g. waiting,
  executing, blocked) vary too much between engine implementations to
  standardize at this level. Application-specific models introduce FSMs for
  lower-level constructs (e.g. tasks) that perform work on behalf of an
  Operator and reference it by ID.

## Port

A Port is an [Entity][entity] that represents either an input or output of an
[Operator][operator].

Must have:

- `operator_id: uuid`: The ID of the [Operator][operator] this port belongs to.
- `instance_name: string`: The name of the Port.

May have:

- `statistics`: Custom attributes emitted post-execution containing port
  metrics (e.g. total rows, total bytes).

Notes:

- A Port does not declare its direction. Whether a Port is an input or output
  is inferred from the [Plan][plan]'s edges, from the perspective of the
  operator: a Port that appears as a `source` in an edge is an output; a Port
  that appears as a `target` is an input.

## Worker

A Worker is an [Entity][entity] responsible for the execution of a
[Plan][plan] at the lowest level. A Worker tracks a start and end time.

Must have:

- `engine_id: uuid`: the ID of the [Engine][engine] this Worker belongs to.

## Relations

Engine models must aim to define entities in such a way that they can,
possibly through several layers of indirection, be related to an Engine.

All domain entities (Engine, QueryGroup, Query, Plan, Operator, Port, Worker)
act as [Resource Groups][resource-group], forming a hierarchy through which
resource usages can be aggregated. See [Resource Group][resource-group] for
details.

[mutual-exclusion]: ../../modeling/README.md#mutual-exclusion
[engine]: #engine
[entity]: ../../modeling/entity.md
[finite-state-machine]: ../../modeling/fsm.md
[operator]: #operator
[plan]: #plan
[port]: #port
[query]: #query
[query-group]: #query-group
[resource-group]: ../../modeling/resource_group.md
[span]: ../../modeling/time.md#span
[worker]: #worker
