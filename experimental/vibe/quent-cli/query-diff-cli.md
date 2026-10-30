# Query diff CLI

## Command

```sh
pixi run pnpm --dir experimental/vibe/quent-cli ask query-diff \
  [--engine ENGINE] \
  [--baseline-query QUERY] \
  [--baseline-engine ENGINE] \
  [--candidate-query QUERY ...] \
  [--candidate-engine ENGINE ...] \
  [--candidate ENGINE:QUERY ...] \
  [--baseline-source SOURCE] \
  [--candidate-source SOURCE ...] \
  [--metric METRIC ...] \
  [--no-combined-table] \
  [--api-base http://localhost:8080/api] \
  [--db] \
  [--db-run RUN ...] \
  [--db-api-base-url URL] \
  [--db-token TOKEN] \
  [--db-trust REMOTE ...] \
  [--db-trust-all] \
  [--json]
```

Omitted IDs are selected with a searchable Ink interface backed by the Quent
API. Query-diff presents every engine and query in one tree: baseline selection
chooses one query and candidate selection accepts any number. `--engine`
applies one engine to unqualified IDs. Repeat
`--candidate-query` for same-engine comparisons or `--candidate ENGINE:QUERY`
for unambiguous cross-engine comparisons. In the tree, use Space to toggle
candidates, Enter to confirm, and Ctrl+A to select every filtered query.
After fetching the bundles, the CLI offers a metric multiselect containing only
numeric metrics present in every selected query. Choose individual metrics or
the **All metrics** option.

The comparison algorithm is part of the pure TypeScript `@quent/query-analysis`
workspace package. It has no terminal, React, transport, or Node.js dependency,
so the browser app can consume the same result model. The CLI owns API
orchestration, Ink interaction, and terminal formatting.

## Database sources

Database support is opt-in. Repeat `--db-run` to start one temporary
`quent-open` API per benchmark run, or pass `--db` to enter comma-separated run
IDs interactively. `--db-api-base-url` and `--db-token` mirror
`quent-open`; `QUENT_OPEN_API_BASE_URL`, `QUENT_OPEN_TOKEN`, and its `.env`
loading remain valid defaults. Trust prompts are also inherited, with
`--db-trust` and `--db-trust-all` available for explicit passthrough.

Database APIs start serially to avoid races in the shared viewer build cache.
The command currently requires each run to produce exactly one viewer API.
Every child is stopped when the diff finishes or fails.

With database mode active, a local API is included only when `--api-base` or
`QUENT_API_BASE_URL` is explicitly configured. This makes DB-only operation
independent of a server on localhost while preserving the existing implicit
localhost default for local-only commands.

Local-only:

```sh
pixi run pnpm --dir experimental/vibe/quent-cli ask query-diff \
  --api-base http://localhost:8080/api
```

DB-only, comparing queries within one run:

```sh
pixi run pnpm --dir experimental/vibe/quent-cli ask query-diff \
  --db-run 6647 \
  --db-api-base-url https://accel-etl.nvidia.com \
  --db-token "$QUENT_OPEN_TOKEN"
```

Local vs DB:

```sh
pixi run pnpm --dir experimental/vibe/quent-cli ask query-diff \
  --api-base http://localhost:8080/api \
  --db-run 6647 \
  --db-api-base-url https://accel-etl.nvidia.com \
  --db-token "$QUENT_OPEN_TOKEN"
```

DB vs DB:

```sh
pixi run pnpm --dir experimental/vibe/quent-cli ask query-diff \
  --db-run 6647 \
  --db-run 6650 \
  --db-api-base-url https://accel-etl.nvidia.com \
  --db-token "$QUENT_OPEN_TOKEN"
```

The combined query tree labels sources as `local` and `db RUN`. For JSON,
identify sources explicitly when more than one is registered:

```sh
pixi run pnpm --silent --dir experimental/vibe/quent-cli ask query-diff \
  --db-run 6647 \
  --db-run 6650 \
  --db-api-base-url https://accel-etl.nvidia.com \
  --db-token "$QUENT_OPEN_TOKEN" \
  --baseline-source 6647 \
  --baseline-engine BASELINE_ENGINE \
  --baseline-query BASELINE_QUERY \
  --candidate-source 6650 \
  --candidate-engine CANDIDATE_ENGINE \
  --candidate-query CANDIDATE_QUERY \
  --metric all \
  --json
```

## Agent usage

`--json` is the primary automation interface. It disables Ink and
`quent-open` trust prompts regardless of TTY detection and requires engine,
query, metric, and any ambiguous source IDs:

```sh
pixi run pnpm --silent --dir experimental/vibe/quent-cli ask query-diff \
  --engine ENGINE \
  --baseline-query QUERY \
  --candidate-query QUERY \
  --candidate-query QUERY \
  --metric all \
  --json
```

Candidates can also span engines:

```sh
pixi run pnpm --silent --dir experimental/vibe/quent-cli ask query-diff \
  --baseline-engine ENGINE \
  --baseline-query QUERY \
  --candidate ENGINE:QUERY \
  --candidate ENGINE:QUERY \
  --metric active_span_s \
  --metric output_rows \
  --json
```

Use `ask engines` and `ask queries` with `--json` to discover those IDs.
Successful output has the stable top-level shape
`{ "schemaVersion": 2, "command": "query-diff", "data": ... }`. Candidate
comparisons are returned in `data.comparisons`. Failures write
the corresponding versioned error envelope to stderr and exit nonzero. Keep
pnpm's `--silent` option in automation so its lifecycle messages do not alter
the CLI streams.

## Comparison

The CLI fetches the baseline and all candidate bundles, intersects their
available numeric metrics, then groups operators by:

1. Logical or physical scope.
2. Operator type.
3. Numeric custom-statistic name and quantity.

Each metric is summed within its group. Physical rows use the physical
operator's own type. Logical rows roll the same physical metrics up through
`parent_operator_ids`, matching the parent-operator grouping used by the pivot
table. A logical operator with no physical child uses its own metrics.
`active_span_s` is also summed from operator active spans. The output reports
the baseline, candidate, `candidate - baseline` delta, and percentage delta.
Each candidate starts with summary totals per metric and plan type, followed by
one combined operator table with **Plan** and **Operator** columns. Pass
`--no-combined-table` to restore separate tables for each plan/operator type.
Interactive terminal output colors positive deltas red and negative deltas
blue; redirected output and `NO_COLOR` remain free of ANSI codes.

An active span can include idle gaps, so `active_span_s` is not CPU or GPU
execution time. Missing and unit-incompatible metrics are not given a numeric
delta. A physical operator derived from multiple logical operator types is
reported under a composite logical type rather than counted once per parent.
Logical and physical summaries are overlapping views of the same work and
must not be added together.
