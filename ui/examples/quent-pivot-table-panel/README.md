<!-- SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved. -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

# quent-pivot-table-panel

Grafana panel plugin that renders a Quent query's operator statistics in a
pivoted, sortable, virtualized table — backed by `PivotedStatTable` from
[`@quent/components`](../../packages/@quent/components).

| | |
|--|--|
| **Host**           | Grafana 12+ (ships React 19) |
| **Plugin id**      | `quent-pivottable-panel` |
| **Quent component**| `PivotedStatTable` + `PivotTableToolbar` |
| **Data source**    | Direct fetch via `@quent/client` (`useQueryBundle`) — does **not** use Grafana datasources |

## What it shows

For a configured `(engineId, queryId)` pair, the panel:

1. Fetches the `QueryBundle` from the Quent API.
2. Flattens it into one row per operator (`buildOperatorRows.ts`).
3. Renders a pivot table grouped by `Worker / Plan → Operator Type → Operator`,
   with a heatmap on numeric stat cells, drag-to-reorder columns, click-to-sort,
   and aggregation (sum / mean / min / max / stdev) when groups collapse rows.

The toolbar exposes:

- **Group by** — toggle/reorder index dimensions.
- **Aggregate** — pick the aggregation mode (only meaningful when an index is hidden).
- **Columns** — multi-select of which stat columns are visible.

## Panel options

| Option       | Default | Notes |
|--------------|---------|-------|
| API base URL | `/api`  | Quent server root, e.g. `https://quent.example.com/api`. |
| Engine ID    | _(empty)_ | Required. Until set, the panel shows a "configure me" message. |
| Query ID     | _(empty)_ | Required. |
| Theme mode   | `auto`  | `auto` follows Grafana's `theme.isDark`; `light`/`dark` force a mode. |

## Running locally

This example is **opt-in**: the root `ui/pnpm-workspace.yaml` deliberately excludes
`examples/*`, so `pnpm install` from `ui/` does not pull in `@grafana/*` or build
the panel. Work on the example from inside its own folder, where a nested
`pnpm-workspace.yaml` re-references `../../packages/@quent/*` (so `workspace:*`
still resolves to live source):

```sh
cd ui/examples/quent-pivot-table-panel

# 1. install plugin deps (Grafana SDKs, swc-loader, etc.)
pnpm install

# 2. build the panel in watch mode
pnpm dev
```

In a second terminal, boot Grafana with the plugin mounted:

```sh
cd ui/examples/quent-pivot-table-panel
pnpm server   # docker compose up — http://localhost:3000, admin/admin
```

Add a new dashboard, choose **Quent Pivot Table** as the visualization, and fill in
**API base URL**, **Engine ID**, and **Query ID** in the panel options.

> **Note** on the `.config/` directory: A production-ready Grafana plugin would
> typically be scaffolded with `pnpm dlx @grafana/create-plugin@latest`, which
> drops a `.config/` directory containing webpack/jest/cypress configs that
> the official plugin workflow expects. To keep this example readable, we
> ship a single `webpack.config.ts` that mirrors what that scaffold produces
> for the panel-loading bits. Drop the official `.config/` in alongside it
> when you are ready to publish.

## How it consumes `@quent/*`

```
src/QuentPivotTablePanel.tsx
├─ JotaiProvider                     ← per-panel store, isolates state
├─ QueryClientProvider               ← per-panel TanStack Query cache
│  └─ <PivotTableBody>
│     ├─ useQueryBundle()            ← @quent/client
│     ├─ buildOperatorRows()         ← local adapter (QueryEntities → rows)
│     ├─ useStatGroupTableControls() ← @quent/hooks (group/sort/agg state)
│     ├─ <PivotTableToolbar>         ← @quent/components
│     └─ <PivotedStatTable>          ← @quent/components
```

`setApiBaseUrl()` is called in an effect on mount with the panel's configured URL.

## Why a per-panel `QueryClient` and Jotai `Provider`?

Grafana dashboards can host many panels. If we shared a single `QueryClient`
across panels, two panels pointing at different Quent instances would compete
for cache keys; if we shared a single Jotai store, sort/visibility state from
one panel would leak into another. Mounting both providers inside the panel
component scopes everything correctly.
