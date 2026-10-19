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
## Log analysis

Each run emits typed messages through one engine-wide `AppLog` sink. Worker
messages include worker references, and query messages include query references.
The analysis endpoint supports engine-relative time windows, severity names or
minimum rank, message substrings and regexes, sink selection, and pagination:

```bash
curl -X POST http://localhost:8080/api/engines/ENGINE_UUID/logs \
  -H 'Content-Type: application/json' \
  -d '{"window":{"start":0,"end":60},"min_rank":2,"message_regex":"(?i)query|spill","page":{"max":100,"page":0}}'
```

Windows are half-open, and supplied filters combine with AND. Severity ranks
follow the schema's ordered levels: `trace`, `debug`, `info`, `warn`, `error`.
Regexes use Rust regex syntax; invalid patterns return HTTP 400. An empty JSON
request returns the first 100 messages. Responses include `items`, the matching
`total` before pagination, and severity declarations in `sink_types`.

Schema-specific payloads remain typed in storage and analysis. Additional fields
are projected to dynamic attributes only for messages on the returned page.
Attribute filtering and UI components are deferred.

The shared logging APIs live in `quent-log::analysis` and `quent-log::ui`.
The `analysis` and `ui` Cargo features control these modules and are enabled by
default. To use analysis without UI types or `ts-rs`, set
`default-features = false` and `features = ["analysis"]` on `quent-log`.
Use the same feature settings on `quent-store-build` to omit generated UI
conversions. Disabling both features keeps only schema and constraint support.

## Verify

```bash
cargo fmt --all -- --check
cargo test -p quent-simulator
cargo test -p quent-simulator-analyzer
pixi run cargo check -p quent-simulator-server --features ui
```
