<!-- SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved. -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

# Deep links

Quent deep links are snapshots created by the **Copy Link** action. Opening a
snapshot restores its saved state, but subsequent interactions do not rewrite
the browser URL.

Deep links currently store the shared timeline viewport and expanded resource
rows:

```json
{
  "zoomRange": {
    "start": 12.5,
    "end": 48.75
  },
  "expandedResourceIds": [
    "01a025ff-ea8b-7881-9d31-72a275872c9d",
    "01a025ff-ea8b-7881-9d31-72a275872c9e"
  ]
}
```

The zoom values are seconds relative to the query start. Engine, query, and
active tab remain in the readable route:

```text
/profile/engine/ENGINE/query/QUERY/timeline?s=v1.COMPRESSED_STATE
```

Incoming state is treated as untrusted data and validated with the same Zod
schema used by the UI and command-line tool. The complete absolute URL is
limited to 2,048 characters.

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
may be supplied instead:

```sh
pixi run pnpm --dir ui deep-link create \
  --engine ENGINE \
  --query QUERY \
  --tab timeline \
  --state state.json
```

Decode a generated link:

```sh
pixi run pnpm --dir ui deep-link decode 'URL'
```

Agents should use these commands rather than recreating the compression format.
