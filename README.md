# Quent: QUery ENgine Telemetry

(working title)

PoC to reduce the time-to-conclusion of performance analysis of data processing
engines.

## TL;DR

- A specification of **modeling concepts** (finite state machines and resources)
  for **resource-constrained applications and domains** helps capture and
  visualize **telemetry** in an intuitive way.
- Models dictate the **instrumentation API** to captures telemetry according to
  the model.
- This PoC focuses on **data processing engines** and provides a
  **domain-specific model** on top of which data processing engines can build
  their own engine models.
- This PoC aims to instrument two engines: Presto/Velox/CUDF and SiriusDB. It
  will contain **application-specific models** for these engines based on the
  specified modeling concepts and the domain-specific model.
- This PoC provides a toolchain for **analysis and exploration** of data
  processing engine telemetry to show this framework will
  **reduce the Time-To-Conclusion (TTC)** of people doing
  **performance analysis**.

## Why

Understanding performance of software running on advanced hardware (e.g.
distributed systems with GPUs) is hard, because the software is complex.

While software complexity has rapidly increased as hardware complexity has
rapidly increased, it can be argued that traditional profiling tools have yet to
catch up to the increase in software complexity by providing better
abstractions.

For example, imagine a software engineer happily using some asynchronous
execution library to overlap I/O and computation, and is mainly focused on
high-level abstractions and orchestration around it. As soon as they open a
profiler, they are met with minute details of thread pools, queues, schedulers,
and whatnot, and they will have to spend a lot of time to understand what
they're seeing. Engineers may also spend countless hours reproducing regressions
seen in obscure Continuous Benchmarking infrastructure running in the cloud. Not
to mention all the time spent writing brittle log analysis scripts that break
the second someone makes a commit to refactor a small portion of the code. In
other words, they will experience a high time-to-conclusion (TTC).

This all is especially true for people working with distributed and GPU
accelerated data processing engines, which is what this PoC will focus on.

> Because this a PoC with a narrower scope than the full potential of this framework, in the rest of the README, we'll assume to be in the domain of data processing engines exclusively.

## Who

This project is useful for:

- A) data processing engine developers - when optimizing performance,
  implementing new features, or investigating regressions
- B) engine users - to quickly understand the bottlenecks in their queries
  running on specific engines and systems, in order to rewrite queries or pick a
  better system configuration
- C) system architects - to compare the integration of generic components across
  different engines and systems

## How

From a distance, the execution of queries in data processing engines goes
through three stages:

1. Planning - a query is transformed into a plan (which can go through multiple
   levels of planning) in the form of a Directed Acyclic Graph (DAG) that
   represents a dataflow graph to be executed. Pretty much every engine does
   this.
2. Execution - the query is executed through a typically very specific
   architecture/stack of control flow abstractions that orchestrate the usage of
   resources such as storage and network I/O and computation from a high-level.
3. Hardware - the abstractions ultimately cause work to be performed on CPUs,
   GPUs, etc. These are things that we already have good profiling tools for.

If common concepts of the data processing domain can be modeled in a generic
way, and if we can let engines produce telemetry according to this model, it
becomes possible to:

1. implement common functionality to capture, store, and analyze data processing
   engine telemetry
2. compare the merit of common functionality across multiple engines in order to
   discover which engines do things well (especially integrate GPUs) and which
   engines don't
3. vizualize this telemetry in an intuitive way - much more intuitive than what
   a traditional profiling tool could effectively do, because it has no
   domain-specific knowledge

For data processing engines, this means stakeholders of type A, B, and C (see
[Who](#who)) can quickly answers questions such as:

- When running the same query after a commit with a regression, what is the
  source of the regression?
- How does new feature X for operator Y affect the performance characteristics
  of other operators?
- Which DAG operator's outputs were most spilled over the PCIe interface, when
  and why?
- Which operator causes most pressure on the host memory pool, when and why?
- TODO: gather more examples

## What

![Overview](docs/figures/overview.svg)

### Models

This project provides a domain-specific model for data processing engines on top
of which engineers can map application-specific constructs for their specific
engines.

The modeling technique/approach aims to be generic enough such that it can
support various execution paradigms that specific engines may employ, including
but not limited to:

- local and distributed execution
- sequential and concurrent query execution
- engines operating on asynchronous runtimes
- engines running on heterogeneous systems with non-CPU based compute and
  complex memory hierarchies

For more details, please refer to the [book](book/src/README.md).

Some people familiar with OpenTelemetry may think of the domain-specific model
as "semantic conventions" on top of the new telemetry concepts introduced.

### Technology stack

Quent consists of various composable components, according to the following
layers:

1. **Engine**: the (distributed) query engine to be profiled
2. **Instrumentation**: libraries used by target engines to produce telemetry
   events in the engine's native language.
   - The API provided by such a library will be used by engine developers to
     instrument their engine.
   - Typically wraps around a thin efficient minimal-latency Rust-based layer
     which orchestrastes exporting events.
   - May (but not required to) perform (partial) model validation.
   - Implementations live in [instrumentation/](instrumentation/).
3. **Exporter**: provides the means to export telemetry events captured by the
   instrumentation library.
   - Exporter implementations would typically exist to export telemetry events
     in arbitrary ways.
   - Examples include: a local log file, layered on top of OpenTelemetry logs,
     as Parquet files to a cloud-based object store, or to a database.
   - One exporter will be a collector-exporter, which sends telemetry to a
     centralized collector, see below.
   - Implementations live in [crates/exporter/](crates/exporter).
4. **(Collector)**: service that collects telemetry events into a single process
   and exports them using arbitrary exporters.
   - This is optional because a scalable system may choose to export everything
     in a decentralized manner for performance reasons.
   - A less obvious argument for making this optional is that in typical
     use-cases like continuous benchmarking and production, MOST telemetry is
     never accessed, especially if no performance anomalies occur. Therefore,
     spending cycles on collection can be very wasteful if it can be lazily
     retrieved afterwards instead.

   - Implementations live in [crates/collector/](crates/collector).

5. **Analyzer**: service that reads raw events, validates the model, and
   performs useful aggregations of bulk events used in visualization.
   - The reference implementation lives in [crates/analyzer/](crates/analyzer).
   - In this PoC:
   - Some parts of the implementation will provide building blocks and interface
     only for the primitive modeling concepts (shown in red above) without
     domain-specific knowledge. The goal is to leverage this to be able to
     quickly add visualizations in the UI (see below) of things so specific or
     experimental to one specific engine, that it may not warrant writing a
     whole separate code path in the analyzer for. As a consequence, the
     visualization options will be limited and may be imperfect, but they can be
     provided very quickly to the end user without analyzer code changes, after
     the modification of the engine-specific model and its consequences for
     integration into the engine have been processed only.

   - Some parts may or may not use the above in order to simplify other parts
     that do domain-specific things (shown in green), or may have fully custom
     domain-specific parts in this PoC.

6. **Web Server**: service that interacts with the analyzer and performs final
   data wrangling for UI interactions.
   - The reference implementation lives in [webserver/](webserver).
7. **User Interface**: application facing developers and data engineers using
   the query engine, helps to quickly gain performance insights about queries.

## Running the PoC Server & Simulator

### Docker Compose (recommended)

#### Requirements

- Docker + Docker Compose (or [Podman](https://podman.io/) + [Podman Compose](https://docs.podman.io/en/v5.6.2/markdown/podman-compose.1.html))

#### Steps

Assuming you are running this from the repo root, this will spawn a server and
run the simulator to spam some events at the server. For now :tm:, the server
will store event data in `./data` in `<engine id>.ndjson` format.

- 1. Build the images:

```bash
docker compose build
```

- 2. Spawn the containers:

```bash
docker compose up
```

- 3. Use the server, e.g.:

```bash
curl http://localhost:8080/analyzer/list_engines -H "Accept: application/json"
```

This should return a list of valid engine UUIDs:

```text
["019ae957-6af3-71a3-b7a9-5b351a83a2b1"]%
```

- 4. Shut everything down:

```bash
docker compose down
```

For quickly iterating, you can merge step 1 + 2 using
`docker compose up --build`.

## Analyzer Service API

The Analyzer Service (which for now :tm: runs as part of the `quent-server`
executable) provides HTTP endpoints that trigger analysis of raw event files and
delivers information that is validated and easy-to-digest (as in small enough
for snappy interactions through web technologies).

Typically, interactions with the Analyzer start by listing engines it knows, by
hitting: `/analyzer/engine/list`, which returns a JSON array with strings that
represent engine UUIDs.

From there, various other HTTP endpoints (will) exist to continue to explore the
profile of a query engine. (For now this can be figured out from a very nasty
looking [source file](crates/server/src/main.rs) but I will clean this up soon
:tm:).

Type definitions for `application/JSON` type data delivered by those routes can
be generated by running:

```text
cargo build -p quent-server
```

For now, these are bindings are checked in to the repository under
[this folder](crates/server).

### Example

After obtaining an engine ID, like so:

```text
curl http://localhost:8080/analyzer/list_engines -H "Accept: application/json"
["019aee29-42a6-79b3-be5f-903f041b4e95"]%
```

one may hit the endpoint providing high-level information about an engine:

```text
curl http://localhost:8080/analyzer/engine/019aee29-42a6-79b3-be5f-903f041b4e95 -H "Accept: application/json"
{"id":"019aee29-42a6-79b3-be5f-903f041b4e95","timestamps":{"init":1764932277849433000,"operating":1764932277849439000,"finalizing":1764932277850016000,"exit":1764932277850022000}}%
```

and then continue down to find all query groups of said engine:

```text
curl http://localhost:8080/analyzer/engine/019aee29-42a6-79b3-be5f-903f041b4e95/list_query_groups -H "Accept: application/json"
["019aee29-5659-7f81-80e9-924b55dd3756","019aee29-5659-7f81-80e9-925e254fb669"]%
```

and then continue down to find all queries of said engine:

```text
curl http://localhost:8080/analyzer/engine/019aee29-42a6-79b3-be5f-903f041b4e95/query_group/019aee29-5659-7f81-80e9-924b55dd3756/list_queries -H "Accept: application/json"
["019aee29-5659-7f81-80e9-924b55dd3756","019aee29-5659-7f81-80e9-925e254fb669"]%
```

and finally arrive at a query:

```text
curl http://localhost:8080/analyzer/engine/019aee29-42a6-79b3-be5f-903f041b4e95/query/019aee29-5659-7f81-80e9-9271ce782180 -H "Accept: application/json"
{"id":"019aee29-5659-7f81-80e9-9271ce782180","query_group_id":"019aee29-5659-7f81-80e9-924b55dd3756","timestamps":{"init":1764932277849695000,"planning":1764932277849696000,"executing":1764932277849697000,"idle":1764932277849697000,"finalizing":1764932277849697000,"exit":1764932277849697000}}%
```

... and so forth.

This JSON object matches the definition of the `Engine` type in
[this generated source](crates/server/ts-bindings/Engine.ts).

N.B. that the above is a latency-sensitive pattern that will end at some point.
This going back-and-forth with the analyzer is only necessary at these higher
levels because Engines and Coordinators are assumed to be able to run for very
long periods of time. Eventually we want to paginate these endpoints.

When using the simulator as a telemetry source, it will emit handy links that
should work when using the Docker Compose setup described above to quickly
obtain JSON data from Analyzer endpoints.
