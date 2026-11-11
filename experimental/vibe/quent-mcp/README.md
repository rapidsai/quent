<!-- SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved. -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

# Quent MCP

Experimental Model Context Protocol bridge for a running Quent REST API. It is
mounted at `/mcp` on the analyzer's existing Axum server. The standalone
`quent-mcp` executable provides stdio transport for clients that require it.

The tools mirror REST operations and return raw JSON facts:

- `list_engines`
- `get_engine`
- `list_engine_contexts`
- `list_queries`
- `get_query`
- `single_timeline`
- `bulk_timelines`
- `data_flow_timeline`
- `list_entities`

`list_queries` returns every query in an engine. Its
`initial_group_by_attribute` field identifies the preferred query custom
attribute for grouping when one is available; the simulator uses `workload`.

Deterministic summaries, rankings, joins, and comparisons belong in
`quent-cli`; interpretation belongs in the calling agent. Every CLI command can
start `quent-open` viewers for benchmark database runs and can mix those runs
with an explicitly configured local API.

## Stdio

Start Quent's analyzer server, then configure an MCP client to run:

```sh
pixi run cargo run -p quent-mcp -- \
  --api-base http://localhost:8080/api
```

Logs go to stderr because stdout carries the MCP protocol.

## Streamable HTTP

Start the normal analyzer server. Its MCP endpoint shares the analyzer listener:
`http://127.0.0.1:8080/mcp` by default.

The underlying MCP transport keeps its loopback-only Host allowlist by default.
For a non-local deployment, set `QUENT_MCP_ALLOWED_HOSTS` to a comma-separated
allowlist. Setting it to `*` disables the check and should only be used behind
an authenticating proxy. Browser clients use the analyzer's existing CORS
configuration.

The endpoint does not add authentication; do not expose it or the Quent API to
untrusted networks.

## Quent Open

`quent-open` mounts one MCP endpoint on each generated viewer when the
artifact's pinned Quent revision contains the MCP routes:

```text
ready: MODEL — 1 context(s)  http://127.0.0.1:49152/
mcp: MODEL — 1 context(s)  http://127.0.0.1:49152/mcp
```

Older revisions continue to build and start without an `mcp:` URL. The
generated wrapper always uses the artifact's pinned Quent revision.

## Validation

```sh
cargo test -p quent-mcp
cargo clippy -p quent-mcp --all-targets -- -D warnings
```
