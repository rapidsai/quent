# Quent: Query Engine Telemetry

A PoC for (distributed) query engine telemetry.
The current goal is to write up a thin vertical slice of the technology stack shown below.

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

## Running the Quent Server & Simulator

### Docker Compose (recommended)

#### Requirements
- Docker + Docker Compose (or [Podman](https://podman.io/) + [Podman Compose](https://docs.podman.io/en/v5.6.2/markdown/podman-compose.1.html))

#### Steps

Assuming you are running this from the repo root, this will spawn a server and run the simulator to spam some events at the server.
For now :tm:, the server will store event data in `./data` in `<engine id>.ndjson` format.

1. Build the images:

```bash
docker compose build
```

2. Spawn the containers:

```bash
docker compose up
```

3. Use the server, e.g.: 

```bash
curl http://localhost:8080/analyzer/list_engines -H "Accept: application/json"
```
This should return a list of valid engine UUIDs:
```
["019ae957-6af3-71a3-b7a9-5b351a83a2b1"]%
```

4. Shut everything down:

```bash
docker compose down
```

For quickly iterating, you can merge step 1 + 2 using `docker compose up --build`.
