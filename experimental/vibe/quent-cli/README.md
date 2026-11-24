<!-- SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved. -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

# Question CLI

Experimental Quent CLI for discovering query-engine captures and answering
deterministic questions from one query bundle.

This project has the unstable, agent-authored status described by the parent
[`vibe`](../README.md) directory.

## Setup

From the repository root:

```sh
pixi run pnpm --dir experimental/vibe/quent-cli install
```

## Usage

List available commands:

```sh
pixi run pnpm --dir experimental/vibe/quent-cli ask --help
```

Pass explicit IDs with `--json` for non-interactive automation:

```sh
pixi run pnpm --silent --dir experimental/vibe/quent-cli ask engines --json
pixi run pnpm --silent --dir experimental/vibe/quent-cli ask queries \
  --engine ENGINE_ID --json
```

Question commands use the same interactive query selector when engine or query
IDs are omitted:

```sh
pixi run pnpm --dir experimental/vibe/quent-cli ask query-lifecycle
pixi run pnpm --dir experimental/vibe/quent-cli ask query-structure
pixi run pnpm --dir experimental/vibe/quent-cli ask operator-facts
pixi run pnpm --dir experimental/vibe/quent-cli ask operator-statistics-summary
```

`--json` changes the output format without disabling interactive selection.

Compare one baseline query with one or more candidates:

```sh
pixi run pnpm --dir experimental/vibe/quent-cli ask query-diff
```

See [Query diff CLI](./query-diff-cli.md) for the detailed comparison contract.

## Database sources

Every command accepts database runs as query sources. Repeat `--db-run`, or use
`--db` to enter run IDs interactively:

The two API options serve different systems:

- `--db-api-base-url` selects the Benchmarking API that resolves `--db-run`
  (`QUENT_OPEN_API_BASE_URL` is the environment equivalent).
- `--api-base` selects an already-running local Quent REST API
  (`QUENT_API_BASE_URL` is the environment equivalent). It does not configure
  database-run lookup.

```sh
pixi run pnpm --dir experimental/vibe/quent-cli ask engines \
  --db-run RUN_ID \
  --db-api-base-url https://benchmark.example.test
pixi run pnpm --dir experimental/vibe/quent-cli ask query-lifecycle \
  --db-run RUN_ID \
  --db-api-base-url https://benchmark.example.test
```

Runs can be mixed freely with each other and with a local API:

```sh
pixi run pnpm --dir experimental/vibe/quent-cli ask engines \
  --api-base http://localhost:8080/api \
  --db-api-base-url https://benchmark.example.test \
  --db-run BASELINE_RUN \
  --db-run CANDIDATE_RUN
```

`--db-token`, `--db-trust`, and `--db-trust-all` are also passed to the
temporary `quent-open` viewers. When several sources are available,
single-source commands prompt for one; pass `--source RUN_ID` to choose it
explicitly. `engines` lists every source unless `--source` limits it.

Database mode excludes the implicit localhost API. Pass `--api-base` or set
`QUENT_API_BASE_URL` to include a local API alongside database runs.

After an interactive database command finishes, the CLI prompts for another
command. All commands in that session reuse the same `quent-open` viewers;
select `Exit` when finished. JSON and non-interactive invocations remain
one-shot.

### Agent and JSON usage

Prefer environment variables for database credentials, discover source and
engine IDs, then pass all selections explicitly:

```sh
export QUENT_OPEN_API_BASE_URL=https://benchmark.example.test
export QUENT_OPEN_TOKEN=...

pixi run pnpm --silent --dir experimental/vibe/quent-cli ask engines \
  --db-run RUN_ID --json
pixi run pnpm --silent --dir experimental/vibe/quent-cli ask query-lifecycle \
  --db-run RUN_ID --source RUN_ID --engine ENGINE_ID --query QUERY_ID --json
```

Database-mode JSON output includes source IDs and labels. `query-diff` uses
`--baseline-source` and `--candidate-source` because each selected query may
come from a different local or database source.

Agents can keep the same viewers running with JSON session mode. Send one
request per line; each response is one compact JSON line in the same order:

```sh
printf '%s\n' \
  '{"command":"engines"}' \
  '{"command":"queries","args":["--source","RUN_ID","--engine","ENGINE_ID"]}' \
  '{"command":"exit"}' |
  pixi run pnpm --silent --dir experimental/vibe/quent-cli ask session \
    --db-run RUN_ID --json
```

Each request receives `--json` automatically. Request errors are returned as
machine-error objects without ending the session. Closing stdin or sending the
`exit` command stops every `quent-open` viewer.

## Validation

```sh
pixi run pnpm --dir experimental/vibe/quent-cli check
```
