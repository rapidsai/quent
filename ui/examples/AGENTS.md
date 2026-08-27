<!-- SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved. -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

# examples/ — Agent Instructions

This directory holds **consumer apps** built on top of the published `@quent/*`
packages: custom web apps, Grafana panel plugins, Superset visualization plugins,
and similar.

Read `ui/AGENTS.md` first for workspace-wide conventions, then read this file before
creating or modifying anything under `examples/`.

## How a real consumer installs `@quent/*`

The `@quent/*` packages are designed to be **published to npm** and installed
the normal way. A real downstream consumer (in their own repo) does:

```sh
pnpm add @quent/components @quent/hooks @quent/client @quent/utils \
        @tanstack/react-query @xyflow/react echarts echarts-for-react jotai \
        react react-dom
```

…and then imports from the package roots:

```tsx
import { QuentProvider, DAGChart } from '@quent/components';
```

Everything in this AGENTS.md and in the example folders below is written from
that consumer's point of view — assume `pnpm add @quent/components` (or the
equivalent npm/yarn command) has run and that `node_modules/@quent/*` contains
the published tarball.

## How the in-repo examples differ

The examples under `ui/examples/` exist to **validate** that the packages work as
a portable library and to give each host environment (Grafana, Superset, etc.) a
copy-pasteable starting point. Because they live next to the source they need
one workspace-internal trick that real consumers do not:

- Each example references `@quent/*` via **`link:../../packages/@quent/<pkg>`**
  in its `package.json` instead of a semver range.
- That `link:` reference makes pnpm symlink straight at the source dir (no
  publish step, no tarball), so a change in `ui/packages/@quent/components/src/…`
  is picked up by the next `pnpm dev` / `pnpm build` in the example.
- For an external consumer the equivalent line is just
  `"@quent/components": "^0.x.y"` and `pnpm add` does the rest.

Otherwise the package.json shape is identical between in-repo examples and
external consumers, and every code snippet below works unchanged in both.

### Workspace boundary rules

Each in-repo example is a **self-contained, opt-in workspace**. The root
`ui/pnpm-workspace.yaml` intentionally **does not** include `examples/*`, so:

- `cd ui && pnpm install` does **not** install example dependencies.
- `cd ui && pnpm ci:check` / `pnpm build` do **not** lint, typecheck, test, or
  build any example.
- `cd ui/examples/<name> && pnpm install` (or `pnpm dev`, `pnpm build`, etc.)
  is the only entry point for working on an example.

The example's nested `pnpm-workspace.yaml` lists ONLY the example itself
(`packages: [.]`) and never re-references `../../packages/@quent/*`. The
`link:` deps must NOT appear as workspace members — see "Why `link:` and not
`workspace:*`" below for why this matters.

### Why `link:` and not `workspace:*` (in-repo only)

An earlier shape of these examples used a nested `pnpm-workspace.yaml` that
re-referenced `../../packages/@quent/*`, with the example's own `package.json`
declaring `"@quent/components": "workspace:*"`. That made the `@quent/*` packages
workspace members of *both* the root `ui/` workspace and the example's nested
workspace. Each `pnpm install` would re-resolve their peer deps (React,
`@tanstack/*`, `@xyflow/react`, etc.) and write fresh symlinks into
`ui/packages/@quent/*/node_modules/`. Because the example pins React 18
(Grafana 12) while the root workspace pins React 19, the example install would
silently overwrite the root install's React-19 symlinks with React-18 ones —
breaking `pnpm typecheck` from `ui/` until you nuked
`ui/packages/@quent/*/node_modules/` and re-ran a root install.

`link:` sidesteps this entirely: pnpm just creates a symlink to the source dir
and never inspects, installs, or touches anything under
`ui/packages/@quent/*/`. The example's bundler still resolves `@quent/*`
imports through the symlink (with `resolve.symlinks: true`, the default), and
React/`@grafana/*` are externalized at bundle time so the React-version
mismatch between source and host doesn't matter at runtime.

External consumers don't hit any of this — they install published tarballs,
which carry their own resolved peer deps and never touch the source workspace.

## Folder layout

```
ui/examples/
├── AGENTS.md                 # this file
├── <example-name>/
│   ├── AGENTS.md             # per-example agent notes (always required)
│   ├── README.md             # human-facing setup + run instructions
│   ├── package.json          # name: "@quent-examples/<name>"
│   ├── pnpm-workspace.yaml   # one-line nested workspace: `packages: [.]`
│   └── src/                  # entry + glue code
└── ...
```

Naming: use `kebab-case` directory names and `@quent-examples/<name>` for the package
`name` field. This keeps the public `@quent/*` namespace reserved for library packages.

## What every consumer needs

Regardless of the host environment, every consumer of `@quent/components` must wire up
the same three things:

1. **React 19 + react-dom 19** — peer dep of `@quent/components`. If the host pins an
   older React (Grafana 11 still ships React 18), see "React-version mismatches" below.
2. **`<QuentProvider>`** from `@quent/components` (re-exported from `@quent/hooks`) —
   bundles `QueryClientProvider` + `JotaiProvider` and (optionally) calls
   `setApiBaseUrl` for you. Each instance creates its own QueryClient and Jotai store
   by default, so dashboards with multiple Quent panels do not cross-talk on
   sort/zoom/selection state. Pass `queryClient` / `jotaiStore` to opt into shared
   instances.
3. **`isDark` boolean** — every visualization accepts it as a prop. Resolve it from the
   host's theme system (`useTheme2()` in Grafana, `useTheme()` in Superset, Tailwind
   `dark` class in plain web apps).

The minimum viable shell for any consumer:

```tsx
import { QuentProvider, DAGChart } from '@quent/components';

export function QuentRoot({ isDark, children }: { isDark: boolean; children: React.ReactNode }) {
  return (
    <QuentProvider apiBaseUrl="https://my-quent-server/api">
      {children}
    </QuentProvider>
  );
}
```

Need to override defaults? `QuentProvider` accepts:

- `apiBaseUrl?: string` — applied via `setApiBaseUrl` synchronously during render.
- `queryClient?: QueryClient` — replace the default per-instance client.
- `jotaiStore?: JotaiStore` — replace the default per-instance Jotai store.

If you have to wire the providers yourself (e.g. you need a custom devtools placement
or you are sharing a `QueryClient` with non-Quent code), the equivalent low-level
setup looks like this:

```tsx
import { Provider as JotaiProvider } from 'jotai';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { setApiBaseUrl, DEFAULT_STALE_TIME } from '@quent/client';

const queryClient = new QueryClient({
  defaultOptions: { queries: { staleTime: DEFAULT_STALE_TIME } },
});

setApiBaseUrl('https://my-quent-server/api');

export function QuentRoot({ children }: { children: React.ReactNode }) {
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

2. **package.json** — set the name and add the `@quent/*` packages plus their
   peer deps. For an external consumer:
   ```jsonc
   {
     "name": "<your-app>",
     "private": true,
     "type": "module",
     "dependencies": {
       "@quent/components": "^0.1.0",
       "@quent/client":     "^0.1.0",
       "@quent/hooks":      "^0.1.0",
       "@quent/utils":      "^0.1.0",
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
   Match the versions the main app uses (`ui/package.json`) to keep `pnpm dedupe`
   happy.

   **In-repo example only:** swap each `@quent/*` semver range for the
   `link:` form so changes in `ui/packages/@quent/*/src/` are picked up
   without a publish step, name the package `@quent-examples/<name>`, and add a
   one-liner nested `pnpm-workspace.yaml`:
   ```jsonc
   // package.json (in-repo only diff)
   "name": "@quent-examples/<name>",
   "dependencies": {
     "@quent/components": "link:../../packages/@quent/components",
     "@quent/client":     "link:../../packages/@quent/client",
     "@quent/hooks":      "link:../../packages/@quent/hooks",
     "@quent/utils":      "link:../../packages/@quent/utils",
     // …rest unchanged
   }
   ```
   ```yaml
   # pnpm-workspace.yaml (in-repo only)
   packages:
     - .
   ```

3. **Vite config** — copy the relevant bits from `ui/vite.config.ts`:
   - `dedupe: ['react', 'react-dom', 'jotai', '@tanstack/react-query']`
   - `optimizeDeps.include: ['echarts-for-react']` (it is CJS and needs ESM conversion).
   - If you use the DAG, alias `elkjs` to `'elkjs/lib/elk.bundled.js'`.
   - Add `tailwindcss()` from `@tailwindcss/vite` if you want Tailwind classes to apply
     to the components.

   **In-repo example only:** also add
   `optimizeDeps.exclude: ['@quent/components', '@quent/hooks', '@quent/client', '@quent/utils']`
   so Vite serves the linked TypeScript source on-demand and HMR works across
   packages. External consumers can omit this — they import the published
   tarball's pre-built JS through pnpm's normal resolution.

4. **Tailwind** — copy `ui/src/index.css` (the `@import 'tailwindcss'` + CSS
   variables + an `@source` directive pointing at the `@quent/*` packages).
   Without the `@source` directive Tailwind will not scan the package source
   and component styles will be missing.

   - External consumer: `@source "node_modules/@quent/**/*.{ts,tsx}";`
   - In-repo example: `@source "../../packages/@quent/**/*.{ts,tsx}";`
     (matches the `link:` symlink target)

5. **Entry point** (`src/main.tsx`) — wrap your app in the providers from "What every
   consumer needs" above and render your visualizations.

6. **Run from the example folder** (not the workspace root):
   `cd ui/examples/<name> && pnpm install && pnpm dev`. The `link:` deps make
   `pnpm install` symlink straight at the live source dirs, so a change in
   `ui/packages/@quent/*/src/` is picked up by the next reload. The root
   `ui/pnpm-workspace.yaml` is intentionally unaware of examples so the main UI
   install stays slim.

   For an external consumer (no in-repo source), this step is just
   `cd <your-app> && pnpm install && pnpm dev` — pnpm pulls the published
   `@quent/*` tarballs from the registry.

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
- Make sure webpack follows symlinks into `node_modules/@quent/*`
  (`resolve.symlinks: true`, default in Grafana's config). Required for
  in-repo examples where `@quent/*` is a `link:` symlink to source; harmless
  for external consumers where it just resolves the published package.

Out-of-the-box `@quent/*` packages publish a **`tsup`-built `dist/`** as their
`main`, so external consumers' webpack/SWC just consumes pre-compiled JS. The
in-repo `link:` shape instead points `main` at `src/index.ts`, so Grafana's
`swc-loader` (or `ts-loader`) ends up transpiling the TypeScript on the fly —
make sure the loader rule does NOT exclude `node_modules/@quent/*` (e.g.
`exclude: /node_modules\/(?!@quent\/)/` as in this repo's
`quent-pivot-table-panel/webpack.config.cjs`). Keep `transpileOnly: true`
either way so cross-package types are not re-checked inside the plugin build.

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
import { PanelProps } from '@grafana/data';
import { useTheme2 } from '@grafana/ui';
import { QuentProvider, Timeline } from '@quent/components';

export function QuentPanel({ data, width, height, options }: PanelProps) {
  const theme = useTheme2();
  // QuentProvider creates a per-instance QueryClient + Jotai store, so two
  // Quent panels in one dashboard do not share zoom/selection state.
  return (
    <QuentProvider apiBaseUrl={options.apiBaseUrl ?? '/api'}>
      <div style={{ width, height }} className={theme.isDark ? 'dark' : ''}>
        <Timeline {...buildPropsFromGrafanaData(data)} isDark={theme.isDark} />
      </div>
    </QuentProvider>
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
   // SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
   // SPDX-License-Identifier: Apache-2.0
   ```

2. **Import only from package roots** — use `import { Timeline } from '@quent/components'`,
   never `'@quent/components/src/timeline/Timeline'`.

3. **Atoms only inside React** — never import a Jotai atom from a `.ts` utility file.
   Pass values as plain arguments. (See `ui/AGENTS.md` "Atom usage".)

4. **Run checks in two places after every change**:

   From `ui/` (must still pass even though examples are not part of the workspace —
   they import live `@quent/*` source, so a breaking package change still trips the
   example's typecheck):
   ```
   pnpm format
   pnpm ci:check
   ```
   From the example folder (opt-in; not run by root CI):
   ```
   cd ui/examples/<name>
   pnpm install
   pnpm typecheck
   pnpm build
   ```

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
