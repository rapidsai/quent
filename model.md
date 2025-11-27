# Query Engine Model for Profiling

## Terms
- WCT: Wall-Clock Time, nanoseconds passed since the Unix epoch
- lifetime: the time range a stateful entity exists (e.g. in memory), typically WCT
- FNTM: for now :tm: - clear sources of technical debt introduced by cutting corners

## Entity (meta)

Anything that can be traced, measured, or in some other way produce useful telemetry. 
Every entity has a universally unique identifier such that no coordination is required between any systems to generate the identifiers.

Has:
- [UUID](https://www.rfc-editor.org/rfc/rfc9562) (ideally v7 which includes a Unix timestamp)

## Query

A Query represents the top-level unit of work executed on an Engine.

Has:
- lifetime (WCT)
- planning time  (WCT): the time spent on query planning
- execution time (WCT): the time spent on executing the query

May have:
- statement: a binary blob capturing any arbitrary data representing the query statement. This can be e.g. a UTF-8 SQL statement or some serialized form of a Polars or DataFusion dataframe that is to be lazily evaluated.

Notes:
- FNTM: the statement binary blob should aim to be small (let's say less than a one or two MiB). This is to prevent OTel over gRPC with default configs to not exceed the default max message size of 4 MiB.

### Plan

A Plan is a directed acyclic graph (DAG) where vertices are Operators and Edges represent data flowing between Operators.
A Plan can be the parent of other Plans (FNTM one child per plan), where the Operators of a child Plan may be logically encapsulated by Operators of a parent Plan.

Must have:
- a parent Query
- lifetime (WCT)
- execution time (WCT)

May have:
- a parent Plan

#### Operator

Must have:
- A parent Plan

May have: 
- A parent Plan Operator: in case it is a lowering of such a parent

Metrics:
- Processing time (WCT): the time range in which all rows of any inputs were processed by this operator

Notes: 
- At least one Edge (and thus one port) must be associated with an Operator.

##### Port

Must have:
- A parent Operator

Metrics:
- Input rows
- Input bytes
- Output rows
- Output bytes

#### Edge

Must have:
- Source Operator Port
- Destination Operator Port

## Engine

Has:
- lifetime (WCT)

### Worker

A worker

### Resource (meta)
A Resource is an Entity with at least one bounded quantity.
The quantity and bounds can change over time.

### Use (meta)
A Use of a Resource


### Memory
A spatial resource holding bytes.

Examples:
- Memory Pool

Has:
- lifetime (bounded by engine lifetime)
- capacity (the maximum amount that could be stored).
- utilization (the number of stored bytes)

#### Allocation
A reservation

Has:
- 


#### Interface
Any type of resource transferring bytes over time.

Examples:
- H2D / D2H

Can have a lifetime.

#### Compute Resource
##### Compute task
#### Interface Resource
