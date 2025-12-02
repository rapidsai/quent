# Quent: Query Engine Telemetry

A PoC for (distributed) query engine telemetry.
We aim to build this in one month.

TODO, super rudimentary, simple, easy and quick to duct-tape together poor man's Quenta stack:
- [ ]: Query Engine Model specification
  - See [model.md](./model.md)
  - It seems infeasible to do the entire model in the time given, but we will iterate from top to bottom as far as we can.
- [ ]: Client API
  - This is the thing engines use in their code paths to capture telemetry according to the model.
  - Will write this in Rust with C-style API so it will be easy to generate bindings to anything (e.g. Python, C, C++, Java, Go).
  - We should aim  to define this in a way that makes it easy to re-use later when we rip out the entire rest of the stuff below.
- [ ]: Distributed transport and collection layer
  - To be able to do this within a VERY short amount of time, we will just get the client API to send simple gRPC messages to a single server that dumps the output to a file. This is not decentralized / scalable. The messages will just be serialized records.
  - This is absolutely the first thing that should be replaced later.
  - [ ]: Figure out the state of OTel collectors, probably sink logs to a file (hopefully something more efficient than JSON but if that's our only choice, so be it)
- [ ]: Post-processing
  - [ ]: Could just be a Python script parsing and transforming the OTel collector output.
- [ ]: Visualization
  - [ ]: Could just be a Python script.

## Model

Quent provides a query engine model onto which engine implementers need to map the constructs of their engine.
The model aims to be generic enough such that it supports various execution paradigms, including but not limited to:

- local and distributed execution
- sequential and concurrent query execution
- engines operating on asynchronous runtimes
- engines running on heterogeneous systems with non-CPU based compute

For more details, please refer to the model specification: [model.md](./model.md)

## Technology stack

Quent consists of various composable components, according to the following layers:

1. **Engine**: the (distributed) query engine to be profiled. Anything for which the top-level query can be expressed as a data-flow system processing a Directed Acyclic Graph (DAG) is a potential candidate.
2. **Instrumentation**: libraries used by target engines to produce telemetry events in the engine's native language.
- The API provided by such a library will be used by engine developers to instrument their engine.
- Typically wraps around a thin efficient minimal-latency Rust-based layer which orchestrastes exporting events.
- May (but not required to) perform (partial) model validation.
3. **Exporter**: provides the means to export telemetry events captured by the instrumentation library.
  - Exporter implementations would typically exist to export telemetry events in arbitrary ways.
  - Examples include: a local log file, layered on top of OpenTelemetry logs, as Parquet files to a cloud-based object store, or to a database.
  - One exporter will be a collector-exporter, which sends telemetry to a centralized collector, see below.
4. **(Collector)**: service that collects telemetry events into a single process and exports them using arbitrary exporters.
  - This is optional because a scalable system may choose to export everything in a decentralized manner for performance reasons.
  - A less obvious argument for making this optional is that in typical use-cases like continuous benchmarking and production, MOST telemetry is never accessed, especially if no performance anomalies occur. Therefore, spending cycles on collection can be very wasteful.
5. **Analzer**: service that reads raw events, validates the model, and performs useful aggregations of bulk events used in visualization.
6. **Web Server**: service that interacts with the analyzer and performs final data wrangling for UI interactions.
7. **User Interface**: application facing developers and data engineers using the query engine, helps to quickly gain performance insights about queries.
