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
- `list_query_groups`
- `list_queries`
- `get_query`
- `single_timeline`
- `bulk_timelines`
- `data_flow_timeline`
- `list_entities`

Deterministic summaries, rankings, joins, and comparisons belong in
`quent-cli`; interpretation belongs in the calling agent.

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

`quent-open` mounts one MCP endpoint on each generated viewer and prints both
URLs:

```text
ready: MODEL — 1 context(s)  http://127.0.0.1:49152/
mcp: MODEL — 1 context(s)  http://127.0.0.1:49152/mcp
```

`quent-open` includes MCP only when the artifact's pinned Quent revision
contains the `quent-mcp` package, ensuring its tools match that revision's REST
contract. Older revisions still build and start the viewer without an `mcp:`
URL; the host checkout is not used as a fallback.

## Validation

```sh
cargo test -p quent-mcp
cargo clippy -p quent-mcp --all-targets -- -D warnings
```
