# Query Engine Model for Profiling

## Terms
- WCT: Wall-Clock Time, nanoseconds passed since the Unix epoch
- lifetime: the time a stateful entity exists in memory
- FNTM: for now :tm: - clear sources of technical debt introduced by cutting corners

## Entity

Anything that can be traced, measured, or in some other way produce useful telemetry. 
Every entity has a universally unique identifier such that no coordination is required between any systems to generate the identifiers.

Has:
- [UUID](https://www.rfc-editor.org/rfc/rfc9562) (ideally v7 which includes a Unix timestamp)

## Query

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
- lifetime (WCT)
- execution time (WCT)

May have:
- A parent plan

#### Operator

Must have:
- At least one input or one output port.

May have: 
- A source operator. Mandatory in case a parent plan exists.

##### Port


#### Edge

Must have:
- Source operator port
- Destination operator

## Engine

Has:
- lifetime (WCT)


### Resource
A Resource is an Entity with at least one bounded quantity.
The quantity and bounds can change at any WCT within the lifetime of an engine.

#### Memory Resource
A spatial resource holding bytes.

Examples:
- Memory Pool

Has:
- lifetime
- capacity (the maximum amount that could be stored).
- utilization (the number of stored bytes)

##### Allocation


#### Interface Resource
Any type of resource transferring bytes over time.

Examples:
- H2D / D2H

Can have a lifetime.

#### Compute Resource
##### Compute task
#### Interface Resource
