<!-- SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved. -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

# Deep links

Quent deep links are snapshots created by the **Copy Link** action. Opening a
snapshot restores its saved state, but subsequent interactions do not rewrite
the browser URL.

Deep links store the route and the non-default state needed to reconstruct the
shared view. Expanded row IDs may include stable synthetic IDs such as
`__nvtx__`:

```json
{
  "route": {
    "engineId": "engine-a",
    "queryId": "query-a",
    "tab": "entities"
  },
  "selection": {
    "planId": "plan-a",
    "operatorNodeIds": ["operator-a"]
  },
  "resources": {
    "expandedRowIds": ["__nvtx__", "01a025ff-ea8b-7881-9d31-72a275872c9d", "resource-a"],
    "rootResourceType": "channel"
  },
  "dag": {
    "nodeColorField": "duration_s"
  },
  "dataFlow": {
    "measure": "bytes",
    "dimensions": ["filesystem"]
  },
  "operatorTable": {
    "enabledGroups": ["partition", "item_type"],
    "visibleStats": ["duration_s", "spill_bytes"],
    "aggregation": "sum",
    "sort": [{ "id": "spill_bytes", "desc": true }]
  },
  "entities": {
    "operatorId": "operator-a",
    "entityType": "task",
    "resourceId": "resource-a",
    "minUsageS": 0.25,
    "window": {
      "start": 12.5,
      "end": 48.75
    },
    "sortDir": "Asc",
    "pageSize": 100,
    "page": 2,
    "selectedEntityId": "entity-a"
  }
}
```

The zoom and playhead values are seconds relative to the query start. Engine,
query, and active tab remain readable in the route and are repeated in the
payload so mismatched or transplanted state can be rejected:

```text
/profile/engine/ENGINE/query/QUERY/entities?s=v3.COMPRESSED_STATE
```

Incoming state is treated as untrusted data and validated with the same Zod
schema used by the UI and command-line tool. Limits are established on string
and array lengths (see `deepLink.schema.ts`), and the complete absolute URL is
limited to 2,048 characters. Existing `v1` viewport/resource links remain
decodable, as do `v2` timeline/operator snapshots. The v1
`expandedResourceIds` name is retained for compatibility and may also contain
synthetic row IDs.

Default DAG, resource, data-flow, and table controls are omitted. Hover,
playback, open popovers, and other transient state are not shared.

Entity snapshots preserve the controls that define and order the result set:
the effective operator override, entity and resource filters, minimum usage,
time window, sort direction, non-default page size, current results page, and
selected entity ID. Selection is restored after the matching page loads.
Entity links do not require or store a timeline viewport.

## Agent commands

Create a relative timeline link:

```sh
pixi run pnpm --dir ui deep-link create \
  --engine ENGINE \
  --query QUERY \
  --tab timeline \
  --start 12.5 \
  --end 48.75
```

Add `--base http://localhost:5173` to emit an absolute URL. A JSON state file
may be supplied instead. The command injects the route from `--engine`,
`--query`, and `--tab`, so the state file may contain the remaining v3 fields:

```sh
pixi run pnpm --dir ui deep-link create \
  --engine ENGINE \
  --query QUERY \
  --tab timeline \
  --state state.json
```

Use `--tab entities` with a state file containing an `entities` object to
create an entity snapshot.

Decode a generated link:

```sh
pixi run pnpm --dir ui deep-link decode 'URL'
```

Agents should use these commands rather than recreating the compression format.
