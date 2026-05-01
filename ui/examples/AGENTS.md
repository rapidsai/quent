<!-- SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved. -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

# examples/ — Agent Instructions

This directory holds **consumer apps** built on top of the `@quent/*` workspace packages:
custom web apps, Grafana panel plugins, Superset visualization plugins, and similar.

Read `ui/AGENTS.md` first for workspace-wide conventions, then read this file before
creating or modifying anything under `examples/`.

## When to add an example here

Add a new folder under `ui/examples/` when you need to:

- Demonstrate how to embed a `@quent/components` visualization in a host environment
  (Grafana, Superset, Streamlit, plain HTML, etc.).
- Build a focused single-purpose app (e.g. a kiosk dashboard) that reuses the packages
  but is not part of the main Quent UI.
- Validate that the packages work as a portable library, not just inside the main app.

Each example must be a self-contained workspace package picked up automatically by the
`examples/*` glob in `ui/pnpm-workspace.yaml`.

## Folder layout

```
ui/examples/
├── AGENTS.md                 # this file
├── <example-name>/
│   ├── AGENTS.md             # per-example agent notes (always required)
│   ├── README.md             # human-facing setup + run instructions
│   ├── package.json          # name: "@quent-examples/<name>"
│   └── src/                  # entry + glue code
└── ...
```

Naming: use `kebab-case` directory names and `@quent-examples/<name>` for the package
`name` field. This keeps the public `@quent/*` namespace reserved for library packages.

## What every consumer needs

Regardless of the host environment, every consumer of `@quent/components` must wire up
the same five things:

1. **React 19 + react-dom 19** — peer dep of `@quent/components`. If the host pins an
   older React (Grafana 11 still ships React 18), see "React-version mismatches" below.
2. **`QueryClientProvider`** from `@tanstack/react-query` — required by `@quent/client`
   hooks.
3. **`<Provider>`** from `jotai` — required by `@quent/hooks`. Wrap each isolated
   visualization in its own provider so atom state does not leak between instances
   (this matters in Grafana, where one dashboard may render many panels).
4. **`setApiBaseUrl(url)`** from `@quent/client` — must be called once before any data
   hook fires. In hosts without Vite env vars, derive this from the host's settings or
   datasource config.
5. **`isDark` boolean** — every visualization accepts it as a prop. Resolve it from the
   host's theme system (`useTheme2()` in Grafana, `useTheme()` in Superset, Tailwind
   `dark` class in plain web apps).

The minimum viable shell for any consumer:

```tsx
import { Provider as JotaiProvider } from 'jotai';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { setApiBaseUrl, DEFAULT_STALE_TIME } from '@quent/client';

const queryClient = new QueryClient({
  defaultOptions: { queries: { staleTime: DEFAULT_STALE_TIME } },
});

setApiBaseUrl('https://my-quent-server/api');

export function QuentRoot({ isDark, children }: { isDark: boolean; children: React.ReactNode }) {
  return (
    <JotaiProvider>
      <QueryClientProvider client={queryClient}>
        {children}
      </QueryClientProvider>
    </JotaiProvider>
  );
}
```

## Choose your host

Pick the section that matches the example you are building.

### A. Custom web app (Vite)

Use this when there is no host application — you are shipping a standalone SPA.

1. **Scaffold** with Vite + React + TypeScript:
   ```
   pnpm create vite@latest <name> -- --template react-ts
   ```
   Move the result under `ui/examples/<name>/`.

2. **package.json** — set the name and add workspace deps:
   ```jsonc
   {
     "name": "@quent-examples/<name>",
     "private": true,
     "type": "module",
     "dependencies": {
       "@quent/components": "workspace:*",
       "@quent/client":     "workspace:*",
       "@quent/hooks":      "workspace:*",
       "@quent/utils":      "workspace:*",
       "@tanstack/react-query": "^5.90.0",
       "@xyflow/react":         "^12.10.0",
       "echarts":               "^5.6.0",
       "echarts-for-react":     "^3.0.6",
       "jotai":                 "^2.18.0",
       "react":                 "^19.2.0",
       "react-dom":             "^19.2.0"
     }
   }
   ```
   Match the versions the main app uses (`ui/package.json`) to keep `pnpm dedupe` happy.

3. **Vite config** — copy the relevant bits from `ui/vite.config.ts`:
   - `dedupe: ['react', 'react-dom', 'jotai', '@tanstack/react-query']`
   - `optimizeDeps.exclude: ['@quent/components', '@quent/hooks', '@quent/client', '@quent/utils']`
     so Vite serves workspace source on-demand and HMR works across packages.
   - `optimizeDeps.include: ['echarts-for-react']` (it is CJS and needs ESM conversion).
   - If you use the DAG, alias `elkjs` to `'elkjs/lib/elk.bundled.js'`.
   - Add `tailwindcss()` from `@tailwindcss/vite` if you want Tailwind classes to apply
     to the components.

4. **Tailwind** — copy `ui/src/index.css` (the `@import 'tailwindcss'` + CSS variables +
   `@source "../../packages/@quent/**/*.{ts,tsx}"` directive). Without the `@source`
   directive Tailwind will not scan the workspace packages and component styles will be
   missing.

5. **Entry point** (`src/main.tsx`) — wrap your app in the providers from "What every
   consumer needs" above and render your visualizations.

6. **Run from the workspace root**: `pnpm --filter @quent-examples/<name> dev`.

### B. Grafana panel plugin

Use this when you want the visualization to appear as a panel option inside Grafana.

#### Scaffolding

Grafana plugins use their own toolchain (webpack, not Vite) and a strict folder layout:

```
pnpm dlx @grafana/create-plugin@latest
# choose: panel, TypeScript, name = quent-<thing>-panel
```

Move the generated folder under `ui/examples/quent-<thing>-panel/` and rename the
package to `@quent-examples/quent-<thing>-panel`.

#### React-version mismatches

Grafana 11.x ships React 18. `@quent/components` declares `"react": "^19.0.0"` as a
peer dep. Two options:

- **Preferred:** require Grafana 12+ in `plugin.json`
  (`"dependencies": { "grafanaDependency": ">=12.0.0" }`) — Grafana 12 ships React 19.
- **Fallback:** add a webpack alias mapping `react` and `react-dom` to the version
  Grafana provides, and add a top-level `peerDependencies` override in your example's
  `package.json` so pnpm does not error. Test carefully — some `@quent/components`
  features rely on React 19 hooks (`useDeferredValue` semantics).

#### Webpack config

Extend the `@grafana/create-plugin` webpack config to:

- Mark `react`, `react-dom`, `@grafana/ui`, `@grafana/data`, `@grafana/runtime` as
  externals (already handled by the template).
- Add an alias for `elkjs` → `elkjs/lib/elk.bundled.js` if you use the DAG.
- Tell webpack to follow symlinks into `node_modules/@quent/*` so the workspace source
  is resolved (`resolve.symlinks: true`, default in Grafana's config).

Workspace packages ship **TypeScript source** as `main`. Grafana's webpack/SWC config
already transpiles TS, so no extra loader is needed — just make sure `transpileOnly`
is true in `ts-loader` (the template default) so cross-package types are not re-checked
inside the plugin build.

#### Styles

Grafana panels are isolated by webpack but share the host's Emotion theme. Two ways to
get `@quent/components` styles in:

1. **Tailwind extracted at build time** — add `@tailwindcss/postcss` and the same
   `@source "node_modules/@quent/**/*.{ts,tsx}"` directive as the main app.
   Import the resulting `styles.css` from your panel module's entry. This is the
   recommended path.
2. **Inline via Grafana theme** — wrap visualizations in a div with `className="dark"`
   when `theme.isDark` is true so the CSS variables defined in our base CSS resolve
   correctly.

#### Panel module

A typical Grafana panel that hosts a Quent component looks like this:

```tsx
// src/module.ts
import { PanelPlugin } from '@grafana/data';
import { QuentPanel } from './QuentPanel';
import './styles.css';

export const plugin = new PanelPlugin(QuentPanel);
```

```tsx
// src/QuentPanel.tsx
import { useMemo } from 'react';
import { PanelProps } from '@grafana/data';
import { useTheme2 } from '@grafana/ui';
import { Provider as JotaiProvider } from 'jotai';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { setApiBaseUrl, DEFAULT_STALE_TIME } from '@quent/client';
import { Timeline } from '@quent/components';

export function QuentPanel({ data, width, height, options }: PanelProps) {
  const theme = useTheme2();
  // One QueryClient + Jotai store *per panel instance* so dashboards with
  // multiple Quent panels do not share zoom/selection state.
  const queryClient = useMemo(
    () => new QueryClient({ defaultOptions: { queries: { staleTime: DEFAULT_STALE_TIME } } }),
    []
  );
  setApiBaseUrl(options.apiBaseUrl ?? '/api');

  return (
    <JotaiProvider>
      <QueryClientProvider client={queryClient}>
        <div style={{ width, height }} className={theme.isDark ? 'dark' : ''}>
          <Timeline {...buildPropsFromGrafanaData(data)} isDark={theme.isDark} />
        </div>
      </QueryClientProvider>
    </JotaiProvider>
  );
}
```

#### Data: Grafana DataFrame ⇄ Quent types

The component library expects domain types from `@quent/utils` (`QueryBundle`,
`TimelineSeries`, etc.), not Grafana `DataFrame`s. Two patterns:

- **Direct fetch:** ignore the Grafana datasource and call `fetchQueryBundle()` /
  `useQueryBundle()` from `@quent/client` directly. Simpler, but bypasses Grafana's
  data source plugin system.
- **Adapter:** convert the `PanelProps.data.series` (DataFrames) into the shape the
  Quent component expects in a small `adapter.ts` module. Use this when the panel must
  honor the user's selected datasource and time range.

Document which pattern the example uses in its per-example `AGENTS.md`.

#### Run / dev loop

```
cd ui/examples/quent-<thing>-panel
pnpm dev          # webpack watch
pnpm server       # docker compose up grafana with the plugin mounted
```

Grafana hot-reloads panel JS but not panel options schemas — restart `pnpm server`
after changing `plugin.json` or option editors.

### C. Superset visualization plugin

Use this when the visualization should be a Superset chart type.

Superset plugins live as packages under `superset-frontend/plugins/` in the upstream
Superset repo, but you can develop them in this workspace and link them at install
time.

#### Scaffolding

```
pnpm dlx yo @superset-ui/superset
# choose: plugin (chart), TypeScript, name = quent-<thing>
```

Move under `ui/examples/quent-<thing>-superset/` and rename the package.

#### Required structure

A Superset chart plugin exports two things:

```ts
// src/index.ts
export { default } from './plugin';     // ChartPlugin instance
export * from './types';

// src/plugin/index.ts
import { ChartPlugin } from '@superset-ui/core';
import buildQuery from './buildQuery';
import controlPanel from './controlPanel';
import transformProps from './transformProps';
import thumbnail from './images/thumbnail.png';
import { QuentChart } from '../QuentChart';

export default class QuentChartPlugin extends ChartPlugin {
  constructor() {
    super({
      buildQuery,
      controlPanel,
      loadChart: () => Promise.resolve({ default: QuentChart }),
      metadata: { name: 'Quent <Thing>', thumbnail },
      transformProps,
    });
  }
}
```

The `QuentChart` React component then mounts the providers + the `@quent/components`
visualization, the same way the Grafana panel does.

#### Theme + isDark

Resolve `isDark` from `useTheme()` in `@superset-ui/core`:

```tsx
import { useTheme } from '@superset-ui/core';
const theme = useTheme();
const isDark = theme.colors.grayscale.dark2 === theme.colors.grayscale.light5; // ish
```

In practice, expose `isDark` as a chart control and let the user toggle it until
Superset's theme detection stabilizes.

#### Data adapter

Superset passes data as an array of records (`queriesData[0].data`). Write a
`transformProps.ts` that converts these into `TimelineSeries` / `PivotedRow[]` shaped
inputs for the Quent component. Keep the adapter pure and unit-testable.

#### Registering with Superset

Add to Superset's `MainPreset.ts`:

```ts
import QuentChartPlugin from '@quent-examples/quent-<thing>-superset';
new QuentChartPlugin().configure({ key: 'quent-<thing>' }).register();
```

Document the registration step in the example's README.

## Workspace conventions (apply to every example)

These are non-negotiable; CI will fail otherwise.

1. **SPDX header** on every new source file:
   ```ts
   // SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
   // SPDX-License-Identifier: Apache-2.0
   ```

2. **Import only from package roots** — use `import { Timeline } from '@quent/components'`,
   never `'@quent/components/src/timeline/Timeline'`.

3. **Atoms only inside React** — never import a Jotai atom from a `.ts` utility file.
   Pass values as plain arguments. (See `ui/AGENTS.md` "Atom usage".)

4. **Run from `ui/` after every change**:
   ```
   pnpm format
   pnpm ci:check
   ```
   The example's own `pnpm --filter @quent-examples/<name> build` must also succeed.

5. **Per-example AGENTS.md** — drop a short AGENTS.md at the example root that lists:
   what host it targets, how data flows in, which `@quent/*` components it uses, and
   anything host-specific an agent needs to know to modify it safely.

## Quick reference: which component for which use case

| Goal | Import from `@quent/components` | Notes |
|------|----------------------------------|-------|
| Render a query plan as a graph | `DAGChart`, `getPlanDAG`, `getTreeData` | Needs `@xyflow/react` peer dep + `elkjs` alias |
| Stacked-area utilization timeline | `Timeline`, `TimelineController`, `TimelineToolbar` | Coordinate multiple via `TimelineController`; share zoom via `useZoomRange` |
| Per-resource timeline strip | `ResourceTimeline` | Uses the same ECharts theme as `Timeline` |
| Operator gantt across workers | `OperatorGanttChart`, `stackOperatorsIntoRows` | Heavy; virtualize the surrounding scroll container |
| Pivoted statistics table | `PivotedStatTable`, `PivotTableToolbar`, `buildPivotedRows` | Pair with `GroupedDataTable` for rendering |
| Generic grouped table with row-span groups | `GroupedDataTable` | Lower-level than `PivotedStatTable`; bring your own `ColumnDef[]` |
| Resource hierarchy sidebar | `ResourceColumn`, `ResourceRow`, `ResourceGroupRow`, `TreeTable` | Also needs `transformResourceTree` from the timeline utils |

For the full export surface, read `ui/packages/@quent/components/src/index.ts`.
