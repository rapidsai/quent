# Intricate query-engine simulator

This experimental simulator uses Quent's query-engine analyzer,
instrumentation model, server, UI bindings, and UI. Its workload models
compressed scans, GPU decoding, partitioned and local joins, host and GPU
memory, storage, and network transfers.

The simulator also emits NVTX domains, categories, marks, nested ranges,
resources, and ranges associated with query execution.

Worker threads execute query partitions concurrently within bounded pipeline
phases. Shuffle exchanges, aggregations, and sorts are query-wide barriers
across all workers; scans and ordinary transforms continue to overlap within
each phase.

## Run

Start the server from the repository root:

```bash
pixi run cargo run -p quent-simulator-server -- --cors-address http://localhost:5173
```

Generate a dataset from another shell:

```bash
pixi run cargo run -p quent-simulator -- --exporter collector
```

The analysis API is available at `http://localhost:8080/api` and the
streamable-HTTP MCP endpoint is available on the same server at
`http://localhost:8080/mcp`.
Run these commands inside `pixi shell`, or prefix each `cargo` command with
`pixi run`.

The simulator uses the same workspace package names and commands as the
upstream simulator. For the frontend development and bundled-UI workflows, see
[`DEVELOPMENT.md`](../../../DEVELOPMENT.md).

The complete Docker example can be started from the repository root:

```bash
docker compose -f experimental/vibe/simulator/docker-compose.yml up --build
```

This starts the analyzer API and MCP endpoint on `http://localhost:8080` and
generates one simulated query group.

## Verify

```bash
pixi run cargo fmt --all -- --check
pixi run cargo test -p quent-simulator -p quent-simulator-analyzer
pixi run cargo check -p quent-simulator-server --features ui
```
