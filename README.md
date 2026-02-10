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
distributed systems with GPUs) is hard, because the software driving this
hardware is complex.

While software complexity has rapidly increased, it can be argued that
traditional profiling tools have yet to catch up to the increase in software
complexity by providing better abstractions.

For example, imagine a software engineer happily using some asynchronous
execution library to overlap I/O and computation, and is mainly focused on
high-level abstractions and orchestration around it. As soon as they open a
profiler, they are met with minute details of thread pools, queues, schedulers,
and whatnot, and they will have to spend a lot of time to understand how they
can improve their use of these abstractions.

Engineers may also spend countless hours reproducing regressions
seen in obscure Continuous Benchmarking infrastructure running in the cloud. Not
to mention all the time spent writing brittle log analysis scripts that break
the second someone makes a commit to refactor a small portion of the code. In
other words, they will experience a high time-to-conclusion (TTC).

This all is especially true for people working with distributed and GPU
accelerated data processing engines, which is what this PoC will focus on.

> Because this a PoC with a narrower scope than the full potential of this
> framework, in the rest of the README, we'll assume to be in the domain of data
> processing engines exclusively.

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

The modeling technique/approach aims to be generic enough such that it can
support various execution paradigms that specific engines may employ, including
but not limited to:

- local and distributed execution
- sequential and concurrent execution
- engines operating on asynchronous runtimes
- engines running on heterogeneous systems with non-CPU based compute and
  complex memory hierarchies

For more details, please refer to the [book](book/src/README.md).

This PoC project provides a domain-specific model for query engines to which an
application-specific model entities can be added. Some people familiar with
OpenTelemetry may think of the domain-specific model as "semantic conventions"
on top of the new telemetry concepts introduced.

### Tech stack

Building blocks are provided for the following layers of a high-performance,
always on telemetry pipeline that allows tracking live progress of an
application.

- Instrumentation
- Exporting / Importing
- Collecting
- Analysis
- Visualization

As a part of this PoC, domain-specific building blocks are provided for Query
Engines.

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
