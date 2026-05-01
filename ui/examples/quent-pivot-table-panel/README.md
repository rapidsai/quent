<!-- SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved. -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

# quent-pivot-table-panel

Grafana panel plugin that renders any tabular dataset as a pivoted, sortable,
virtualized table — backed by `PivotedStatTable` from
[`@quent/components`](../../packages/@quent/components).

|                     |                                                                                                                                        |
| ------------------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| **Host**            | Grafana 12+ (ships React 19)                                                                                                           |
| **Plugin id**       | `quent-pivottable-panel`                                                                                                               |
| **Quent component** | `PivotedStatTable` + `PivotTableToolbar`                                                                                               |
| **Data source**     | Any Grafana datasource that returns the documented columns. The repo provisions Grafana's built-in `TestData` datasource for the demo. |

## What it shows

For every row Grafana returns from the panel's query, the panel produces one
operator entry. The toolbar exposes:

- **Group by** — toggle/reorder index dimensions (`Worker / Plan` →
  `Operator Type` → `Operator`).
- **Aggregate** — pick the aggregation mode (sum / mean / min / max / stdev),
  applied when an index dimension is hidden.
- **Columns** — multi-select of which stat columns are visible.

Cells get a heatmap, columns are drag-to-reorder, and headers are
click-to-sort.

## Expected data shape

The panel adapts each row of `props.data.series[*]` (Grafana `DataFrame`s) into
an `OperatorRow`. Field names are matched case-insensitively; both
snake_case and camelCase work.

| Column               | Required | Purpose                                                                              |
| -------------------- | -------- | ------------------------------------------------------------------------------------ |
| `item_id`            | yes      | Stable identifier for the row. Without it the row is dropped.                        |
| `item_name`          | no       | Display name for the operator (defaults to `item_id`).                               |
| `item_type`          | no       | Used by the operator-type group and color (defaults to `-`).                         |
| `partition_id`       | no       | Outer group identity (defaults to `-`).                                              |
| `partition_label`    | no       | Human label for the partition (defaults to `partition_id`).                          |
| `scope_id`           | no       | Plan/scope identity (defaults to `partition_id`).                                    |
| `scope_label`        | no       | Human label for the scope (defaults to `scope_id`).                                  |
| **all other fields** | —        | Become stats keyed by field name; numeric fields drive the heatmap and aggregations. |

See `provisioning/dashboards/quent-pivot-demo.json` for a worked example
using the `TestData` `csv_content` scenario.

## Panel options

| Option          | Default         | Notes                                                                                                        |
| --------------- | --------------- | ------------------------------------------------------------------------------------------------------------ |
| Theme mode      | `auto`          | `auto` follows Grafana's `theme.isDark`; `light`/`dark` force a mode.                                        |
| Partition label | `Worker / Plan` | Header for the outer (`partition`) index dimension. Override to re-skin the panel for non-operator datasets. |
| Item-type label | `Operator Type` | Header for the middle (`item_type`) index dimension.                                                         |
| Item label      | `Operator`      | Header for the innermost (`item`) index dimension.                                                           |

The panel intentionally has no datasource-aware options — everything else is
the responsibility of the panel's query.

### Two demo panels in one dashboard

`provisioning/dashboards/quent-pivot-demo.json` boots with two panels driven
by the same plugin to show the label overrides in action:

1. **Operator statistics** — the original Quent operator dataset (defaults).
2. **Car price dataset** — a 40-row subset of `car_price_dataset.csv`, mapped
   so `Brand → Fuel type → Car` becomes the `partition / item_type / item`
   hierarchy. The numeric columns (`mileage`, `horsepower`, `price`,
   `engine_size`, `model_year`, `doors`, `owner_count`) become aggregatable
   stats. Toggle off the `Fuel type` index to compare brands directly, or
   switch the aggregator to **mean** to see fleet-wide averages.

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

In a second terminal, boot Grafana with the plugin and demo dashboard mounted:

```sh
pnpm server   # docker compose up — http://localhost:3000, admin/admin
```

The `TestData` datasource and the demo dashboard are provisioned automatically
via `provisioning/`. After login the dashboard appears under the **Quent
Examples** folder at `/d/quent-pivot-demo`.

> **Note** on the `.config/` directory: A production-ready Grafana plugin would
> typically be scaffolded with `pnpm dlx @grafana/create-plugin@latest`, which
> drops a `.config/` directory containing webpack/jest/cypress configs that
> the official plugin workflow expects. To keep this example readable, we
> ship a single `webpack.config.cjs` that mirrors what that scaffold produces
> for the panel-loading bits. Drop the official `.config/` in alongside it
> when you are ready to publish.

## How it consumes `@quent/*`

```
src/QuentPivotTablePanel.tsx
├─ JotaiProvider                     ← per-panel store, isolates state
├─ QueryClientProvider               ← per-panel TanStack Query cache
│  └─ <PivotTableBody>
│     ├─ frameToOperatorRows()       ← local adapter (DataFrame[] → rows)
│     ├─ useStatGroupTableControls() ← @quent/hooks (group/sort/agg state)
│     ├─ <PivotTableToolbar>         ← @quent/components
│     └─ <PivotedStatTable>          ← @quent/components
```

Note: `@quent/client` is **not** a dependency. The panel reads `props.data`
directly from Grafana's data pipeline; no Quent server is needed for the
component to render.

## Why a per-panel `QueryClient` and Jotai `Provider`?

Grafana dashboards can host many panels. If we shared a single Jotai store,
sort/visibility state from one panel would leak into another. Mounting the
provider inside the panel component scopes everything correctly. The
`QueryClient` is unused today (the panel reads from Grafana, not from an
HTTP API), but `useStatGroupTableControls` lives in `@quent/hooks` which
declares `@tanstack/react-query` as a peer dep, so we keep the provider in
the tree to satisfy that contract and to leave a hook in place for future
features (e.g. cross-panel data fetching from a Quent app plugin).

## Pointing it at a different datasource

Anything that returns the documented columns works. Some examples:

- **CSV core datasource** — point the panel at a checked-in CSV in the
  plugin's `public/` folder.
- **Infinity / JSON API datasource** — fetch a JSON endpoint that already
  returns operator rows.
- **PostgreSQL / MySQL** — `SELECT plan_id AS partition_id, …` from your
  warehouse.
- **A future "Quent datasource" plugin** — would do the
  `QueryBundle → OperatorRow[]` adapter server-side and let this panel
  remain identical.

For each, the only thing that changes is the dashboard's panel `targets[*]` —
no plugin code edits required.
