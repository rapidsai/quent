# Query-engine simulator

This is the repository's query-engine simulator. It uses the current Quent
query-engine analyzer, instrumentation model, server, UI bindings, and UI while
keeping its experimental execution model under `experimental/vibe`.

The workload models compressed scans, GPU decoding, partitioned and local
joins, host and GPU memory, storage, and network transfers.
Most operators reduce data volume as execution approaches the output; one early
many-to-many join expands its input, decoding expands compressed input, sorting
preserves cardinality, and the query-wide limit emits at most 42 rows across all
workers.

Worker threads execute query partitions concurrently within bounded pipeline
phases. Shuffle exchanges, aggregations, and sorts are query-wide barriers
across all workers; scans and ordinary transforms continue to overlap within
each phase.

The analyzer exposes the current data-flow timeline protocol and rolls
physical task series up to logical parent operators. The animation therefore
works in both logical and worker-specific physical plan views.

## Run

Run the server from the repository root:

```bash
cargo run -p quent-simulator-server -- --cors-address http://localhost:5173
```

Generate a dataset from another shell:

```bash
cargo run -p quent-simulator -- --exporter collector
```

The simulator uses the same workspace package names and commands as the
upstream simulator. For the frontend development and bundled-UI workflows, see
[`DEVELOPMENT.md`](../../../DEVELOPMENT.md).

The complete Docker example can be started from the repository root:

```bash
docker compose -f experimental/vibe/simulator/docker-compose.yml up --build
```

## Verify

```bash
cargo fmt --all -- --check
cargo test -p quent-simulator
cargo test -p quent-simulator-analyzer
pixi run cargo check -p quent-simulator-server --features ui
```
