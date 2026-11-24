# Agent instructions

## Query sources

Every `pnpm ask` command can use:

- The local Quent API (implicit outside database mode, or explicit with
  `--api-base` / `QUENT_API_BASE_URL`).
- One or more benchmark database runs via repeatable `--db-run`.
- Local and database sources together when `--api-base` and `--db-run` are
  combined.

The endpoint flags are not interchangeable:

- `--db-api-base-url` / `QUENT_OPEN_API_BASE_URL` selects the Benchmarking API
  used to resolve `--db-run` and start temporary `quent-open` viewers.
- `--api-base` / `QUENT_API_BASE_URL` selects an already-running local Quent
  REST API. Never pass the Benchmarking API URL through `--api-base`.
- With `--db-run`, include `--api-base` only when the command intentionally
  needs both the local source and database-run sources.

Use `--source` for commands that operate on one source. `engines` lists all
registered sources by default. For `query-diff`, use `--baseline-source` and
repeatable `--candidate-source` instead.

For deterministic agent calls, pass explicit source, engine, query, and metric
IDs with `--json`. Prefer `QUENT_OPEN_API_BASE_URL` and `QUENT_OPEN_TOKEN` over
putting credentials in command arguments or logs.

Keep database startup, historical viewer API compatibility, and source cleanup
centralized in `src/querySources.ts`. Validate changes with:

```sh
pixi run pnpm --dir experimental/vibe/quent-cli check
```
