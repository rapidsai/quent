# Architecture Research

**Domain:** Internal pnpm workspace package extraction — monorepo component/state/client split
**Researched:** 2026-04-01
**Confidence:** HIGH (grounded in actual codebase; no web search used — patterns verified against existing code)

---

## Standard Architecture

### System Overview

```
┌────────────────────────────────────────────────────────────────┐
│                      ui/src/ (App Shell)                       │
│  routes/, pages/, main.tsx, index.css                         │
│  Consumes packages only via package names, no relative escape  │
├──────────┬──────────────┬────────────────┬─────────────────────┤
│@quent/   │ @quent/      │ @quent/        │ @quent/             │
│components│ hooks        │ client         │ utils               │
│          │              │                │                     │
│React     │ Jotai atoms  │ TanStack Query │ cn(), types,        │
│components│ wrapped in   │ hooks + query  │ formatters,         │
│(DAG,     │ named hooks  │ option fns +   │ colors, constants   │
│timeline, │ No raw atom  │ raw fetchers   │ No React dep        │
│tree, ui) │ exports      │                │                     │
└──────────┴──────┬───────┴────────┬───────┴──────────┬──────────┘
                  │                │                  │
                  └────────────────┴──────────────────┘
                             depends on
                        @quent/utils (types only)

External type source (no package boundary crossed):
~quent/types/* → examples/simulator/server/ts-bindings/
(Rust-generated, re-exported through @quent/utils)
```

### Dependency Graph (no circular deps)

```
@quent/utils        — no internal deps, no React
@quent/client       — depends on @quent/utils only
@quent/hooks        — depends on @quent/utils only (NOT @quent/client)
@quent/components   — depends on @quent/utils only
ui/src (app shell)  — depends on all four packages
```

The critical rule: `@quent/hooks` and `@quent/client` are **siblings**, not dependent on each other.
Hooks that need server data accept data as props/parameters rather than calling `@quent/client` directly.
The app shell composes them: it calls a client hook, passes the result to a state hook, passes state to components.

### Component Responsibilities

| Package | Responsibility | Key Contents |
|---------|---------------|--------------|
| `@quent/utils` | Shared primitives with no React/Jotai/TanStack dependency | `cn()`, `formatDuration()`, `formatQuantity()`, color utilities (`PALETTES`, `getColorForKey()`, etc.), all `~quent/types/*` re-exports, `parseJsonWithBigInt()`, constants (`DEFAULT_STALE_TIME`) |
| `@quent/client` | HTTP layer: raw fetchers + TanStack Query option factories | `apiFetch()`, `fetchQueryBundle()`, `fetchBulkTimelines()`, etc.; `queryBundleQueryOptions()` and peer option factories; re-exports query keys as named constants |
| `@quent/hooks` | Jotai state wrapped as named React hooks; no raw atom exports | `useSelectedNodeIds()`, `useSelectedPlanId()`, `useHoveredWorkerId()`, `useZoomRange()`, `useTimelineData()`, `useHideTasksAtom()` — each wraps an atom and returns a typed `[value, setter]` tuple |
| `@quent/components` | Stateless or self-contained React components | DAGChart, Timeline, QueryPlanTree, ResourceTree, NodeDetailView, all Radix UI primitives (button, card, etc.), TreeView, TreeTable |

---

## Recommended Project Structure

```
ui/
├── src/                              # App shell — the only consumer
│   ├── main.tsx                      # Providers: Router, QueryClient, theme
│   ├── index.css                     # Tailwind entry point (ONE place only)
│   ├── routes/                       # TanStack Router file routes
│   └── pages/                        # (migrating into routes/)
│
└── packages/
    ├── utils/                        # @quent/utils
    │   ├── package.json
    │   ├── tsconfig.json
    │   └── src/
    │       ├── index.ts              # Barrel: everything the package exports
    │       ├── cn.ts                 # twMerge + clsx
    │       ├── colors.ts             # PALETTES, getColorForKey, etc.
    │       ├── formatters.ts         # formatDuration, formatQuantity
    │       ├── constants.ts          # DEFAULT_STALE_TIME, API_BASE_URL
    │       ├── bigint.ts             # parseJsonWithBigInt
    │       └── types/                # Re-export ~quent/types/* here
    │           └── index.ts          # export type { QueryBundle } from '...'
    │
    ├── client/                       # @quent/client
    │   ├── package.json
    │   ├── tsconfig.json
    │   └── src/
    │       ├── index.ts              # Barrel: all hooks + query option fns
    │       ├── api.ts                # apiFetch(), fetch* raw functions
    │       ├── query-keys.ts         # QUERY_KEYS const — single source of truth
    │       └── hooks/
    │           ├── useQueryBundle.ts
    │           ├── useListEngines.ts
    │           ├── useListCoordinators.ts
    │           ├── useListQueries.ts
    │           └── useTimelines.ts
    │
    ├── hooks/                        # @quent/hooks
    │   ├── package.json
    │   ├── tsconfig.json
    │   └── src/
    │       ├── index.ts              # Barrel: all hooks
    │       ├── atoms/                # Internal — NOT exported in index.ts
    │       │   ├── dag.ts            # selectedNodeIdsAtom, etc.
    │       │   └── timeline.ts       # zoomRangeAtom, timelineDataAtom, etc.
    │       └── hooks/
    │           ├── useDAGSelection.ts
    │           ├── useTimelineZoom.ts
    │           ├── useTimelineData.ts
    │           └── useBulkTimelines.ts
    │
    └── components/                   # @quent/components
        ├── package.json
        ├── tsconfig.json
        └── src/
            ├── index.ts              # Barrel: all public components + types
            ├── dag/
            │   ├── DAGChart.tsx
            │   └── DAGControls.tsx
            ├── timeline/
            │   ├── Timeline.tsx
            │   └── TimelineController.tsx
            ├── query-plan/
            │   └── QueryPlanTree.tsx
            ├── resource-tree/
            │   └── ResourceTree.tsx
            └── ui/                   # Radix primitives
                ├── button.tsx
                ├── card.tsx
                └── ...
```

### Structure Rationale

- **`atoms/` is internal to `@quent/hooks`:** Atoms are an implementation detail. Exporting raw atoms from `index.ts` locks the public API to Jotai and prevents future state-library swaps. Only hooks appear in `index.ts`.
- **`query-keys.ts` in `@quent/client`:** Query keys are the contract between fetchers and components that use `queryClient.invalidateQueries()`. Making them a named export prevents key drift between producers and consumers.
- **`types/` subfolder in `@quent/utils`:** The `~quent/types/*` alias currently points to Rust-generated bindings. Re-exporting through `@quent/utils` insulates consumers from future binding source changes (e.g., switching from simulator bindings to `crates/server/ts-bindings`). Consumers import `QueryBundle` from `@quent/utils`, not `~quent/types/QueryBundle`.

---

## Architectural Patterns

### Pattern 1: Flat Barrel with Explicit Named Exports

**What:** Each package's `index.ts` exports every public symbol by name — no `export * from './subdir'` wildcards.

**When to use:** Always, for all four packages.

**Why:** AI coding agents (and TypeScript language servers) read `index.ts` to discover available exports. Wildcard re-exports force them to trace multiple files. Named exports make the API surface scannable in one file.

**Example:**
```typescript
// packages/hooks/src/index.ts — explicit, named, complete
export { useDAGSelection } from './hooks/useDAGSelection';
export { useTimelineZoom } from './hooks/useTimelineZoom';
export { useTimelineData } from './hooks/useTimelineData';
export { useBulkTimelines } from './hooks/useBulkTimelines';
export type { ZoomRange } from './hooks/useTimelineZoom';
// NOTE: atoms/ is intentionally NOT exported
```

**Trade-offs:** More lines of maintenance in `index.ts`; worth it because a broken re-export fails loudly at import time rather than silently exporting undefined.

### Pattern 2: Atoms Hidden, Hooks Exported

**What:** Jotai atoms live in `packages/hooks/src/atoms/` and are never re-exported from `index.ts`. Each atom gets a named hook wrapper.

**When to use:** For all Jotai state in `@quent/hooks`.

**Why:** Components that import `useSelectedNodeIds()` don't need to know Jotai exists. The hook signature (`[Set<string>, (ids: Set<string>) => void]`) is the API contract, not the atom. This also prevents the "atom spaghetti" pattern where a component reads 4 different atoms directly and the call site becomes impossible to refactor.

**Example:**
```typescript
// packages/hooks/src/hooks/useDAGSelection.ts
import { useAtom } from 'jotai';
import { selectedNodeIdsAtom, selectedOperatorLabelAtom } from '../atoms/dag';

export interface DAGSelection {
  selectedNodeIds: Set<string>;
  selectedOperatorLabel: string | null;
  setSelectedNodeIds: (ids: Set<string>) => void;
  setSelectedOperatorLabel: (label: string | null) => void;
}

export function useDAGSelection(): DAGSelection {
  const [selectedNodeIds, setSelectedNodeIds] = useAtom(selectedNodeIdsAtom);
  const [selectedOperatorLabel, setSelectedOperatorLabel] = useAtom(selectedOperatorLabelAtom);
  return { selectedNodeIds, selectedOperatorLabel, setSelectedNodeIds, setSelectedOperatorLabel };
}
```

**Trade-offs:** Slightly more boilerplate than calling `useAtom(selectedNodeIdsAtom)` directly; the API surface is far easier to scan and stub in tests.

### Pattern 3: Query Options Factory + Hook Pair in @quent/client

**What:** For each data resource, export both a `queryOptions` factory (for route loaders and cache priming) and a `useXxx` hook (for components). Both reference the same query key.

**When to use:** Every TanStack Query data source in `@quent/client`.

**Why:** Route loaders in TanStack Router call `queryClient.ensureQueryData(queryBundleQueryOptions(...))` to pre-populate the cache. Components call `useQueryBundle(...)` which reads from the same cache entry. If only a hook is exported, loaders must duplicate the query key and fetch function. The current `useQueryBundle.ts` already demonstrates this pattern correctly — preserve it.

**Example:**
```typescript
// packages/client/src/hooks/useQueryBundle.ts
import { queryOptions, useQuery } from '@tanstack/react-query';
import { fetchQueryBundle } from '../api';
import { QUERY_KEYS } from '../query-keys';
import type { QueryBundle, EntityRef } from '@quent/utils';

export const queryBundleQueryOptions = (engineId: string, queryId: string) =>
  queryOptions({
    queryKey: QUERY_KEYS.queryBundle(engineId, queryId),
    queryFn: () => fetchQueryBundle(engineId, queryId),
    staleTime: 5 * 60 * 1000,
    retry: 2,
  });

export const useQueryBundle = (engineId: string, queryId: string) =>
  useQuery(queryBundleQueryOptions(engineId, queryId));
```

```typescript
// packages/client/src/query-keys.ts
export const QUERY_KEYS = {
  queryBundle:    (engineId: string, queryId: string) => ['queryBundle', engineId, queryId] as const,
  listEngines:    () => ['engines'] as const,
  listCoordinators: (engineId: string) => ['coordinators', engineId] as const,
  listQueries:    (engineId: string, coordinatorId: string) => ['queries', engineId, coordinatorId] as const,
  bulkTimelines:  (engineId: string, queryId: string, zoom: unknown, key: string) =>
                    ['bulkTimelines', engineId, queryId, zoom, key] as const,
} as const;
```

**Trade-offs:** Slightly more surface area per resource; eliminates subtle bugs where a route loader and a hook use different stale times or accidentally-different query keys.

### Pattern 4: CSS Stays in the App Shell — Components are Classname-Only

**What:** `@quent/components` exports React components that accept and apply Tailwind class names via `cn()`. The package ships zero CSS files. The Tailwind stylesheet (`index.css`) lives exclusively in `ui/src/` and is imported only by `ui/src/main.tsx`.

**When to use:** This is the mandatory pattern for Tailwind v4 with the `@tailwindcss/vite` plugin.

**Why:** Tailwind v4's Vite plugin does a single-pass scan of the entire project for class names and generates one CSS bundle for the consuming app. Component packages do not need their own Tailwind config or CSS entry points. Attempting to bundle CSS in `@quent/components` would either create duplicate stylesheets or require the consuming app to deduplicate them. The existing `index.css` with `@import 'tailwindcss'` at the app root is the correct and complete setup — no changes needed.

**What this means for the extraction:** When moving components from `ui/src/components/` to `ui/packages/components/src/`, do NOT add `index.css` or `tailwind.config.ts` to the components package. The Vite plugin in `ui/vite.config.ts` will continue to scan `ui/packages/*/src/**` for class names automatically once the packages are workspace-linked.

**Verification:** The current `vite.config.ts` uses `tailwindcss()` as a Vite plugin (not PostCSS). This plugin scans source files by content. It will pick up class names in `ui/packages/*/src/**/*.tsx` with no additional configuration as long as the workspace symlinks exist.

**Example — correct pattern:**
```typescript
// packages/components/src/dag/DAGChart.tsx
import { cn } from '@quent/utils';

interface DAGChartProps {
  className?: string;
}

export function DAGChart({ className }: DAGChartProps) {
  return (
    <div className={cn('relative w-full h-full bg-background', className)}>
      {/* ... */}
    </div>
  );
}
```

**Do NOT do this:**
```typescript
// packages/components/src/index.ts
import './index.css'; // wrong — don't ship CSS from a component package
```

### Pattern 5: Shared Types Flow Through @quent/utils

**What:** The `~quent/types/*` Vite/TS alias currently points to `examples/simulator/server/ts-bindings/`. All four packages and the app shell import domain types (`QueryBundle`, `EntityRef`, `Plan`, etc.) via `@quent/utils`, not via the raw `~quent/types/*` alias.

**When to use:** Any import of a Rust-generated type.

**Why:** This is the key mechanism that eliminates circular dependencies and makes the package graph acyclic. Without it, `@quent/hooks` and `@quent/client` both need the same Rust-generated types; if either imported directly from `~quent/types/*`, each would need its own alias configuration. By routing through `@quent/utils`, only one package owns the alias, and the others just depend on `@quent/utils`.

**Also:** This insulates against the TODO comment in `vite.config.ts`:
```
// TODO: Using ts bindings from quent for now this will need to change
// to get bindings from webserver when we go that direction
```
When the binding source changes, only `@quent/utils/src/types/index.ts` needs updating.

**Example:**
```typescript
// packages/utils/src/types/index.ts
// Single place that knows the source of truth for generated types
export type { QueryBundle } from '~quent/types/QueryBundle';
export type { EntityRef } from '~quent/types/EntityRef';
export type { Plan } from '~quent/types/Plan';
export type { Engine } from '~quent/types/Engine';
// ... all used types

// packages/client/src/api.ts
import type { QueryBundle, EntityRef, Engine } from '@quent/utils'; // NOT ~quent/types/...
```

---

## Data Flow

### How the App Shell Composes Packages

```
ui/src/routes/profile.engine.$engineId.query.$queryId.tsx
    │
    ├── [loader] queryClient.ensureQueryData(queryBundleQueryOptions(engineId, queryId))
    │            from @quent/client
    │
    └── [component]
          │
          ├── useQueryBundle(engineId, queryId)        ← @quent/client
          │     returns { data: QueryBundle }
          │
          ├── useDAGSelection()                        ← @quent/hooks
          │     returns { selectedNodeIds, setSelectedNodeIds }
          │
          ├── useTimelineZoom()                        ← @quent/hooks
          │     returns { zoomRange, setZoomRange }
          │
          └── <DAGChart                                ← @quent/components
                 bundle={data}
                 selectedNodeIds={selectedNodeIds}
                 onNodeSelect={setSelectedNodeIds}
              />
```

### State Management Data Flow

```
@quent/hooks (atoms — internal)
    │
    ├── selectedNodeIdsAtom  ←──── useDAGSelection() exposes setter
    │        │
    │        └── DAGChart calls setSelectedNodeIds on click
    │                │
    │                └── Timeline reads useDAGSelection() to filter by operator
    │
    └── zoomRangeAtom ←────── useTimelineZoom() exposes setter
             │
             └── useBulkTimelines (in @quent/hooks) reads via store.get()
                  calls fetchBulkTimelines from @quent/client indirectly
                  (app shell passes the fetch function as a dependency, or
                   @quent/hooks accepts it as a parameter — see note below)
```

**Important note on useBulkTimelines:** The current `useBulkTimelines` hook imports `fetchBulkTimelines` directly from `@/services/api`. After extraction, this hook either:

- Option A (recommended): Moves entirely into `@quent/client` since it is fundamentally a data-fetching hook orchestrated by Jotai atoms. The hook depends on both TanStack Query (`useQuery`, `useQueryClient`) and Jotai (`useStore`, `useAtomValue`). Place it in `@quent/client` and have it import atoms from `@quent/hooks/internal` — but this creates the circular dep we are avoiding.
- Option B (correct): Move `useBulkTimelines` to `@quent/hooks` and accept `fetchBulkTimelines` as a parameter.
- Option C (correct, simpler): Keep `useBulkTimelines` in the app shell (`ui/src/hooks/`) since it is a composition of `@quent/client` fetch + `@quent/hooks` atoms. It is a coordinator, not a reusable primitive. Only extract it to a package if a second consumer emerges.

**Recommendation: Option C.** The app shell is the right home for hooks that orchestrate multiple packages. `useBulkTimelines` is complex coordination logic that has only one call site.

---

## Package Configuration

### Each Package's package.json Pattern

```json
{
  "name": "@quent/utils",
  "version": "0.0.0",
  "private": true,
  "type": "module",
  "exports": {
    ".": {
      "types": "./src/index.ts",
      "default": "./src/index.ts"
    }
  },
  "dependencies": {}
}
```

**Key decisions:**
- `"exports": { ".": { "default": "./src/index.ts" } }` — no build step. TypeScript source is the package entry. Vite resolves TypeScript directly via workspace links. This works because the consuming app (Vite) transpiles everything.
- `"private": true` — workspace-only; prevents accidental publish.
- `"version": "0.0.0"` — no semver until npm publish decision. All deps use `workspace:*`.

### pnpm-workspace.yaml (new file at ui/)

```yaml
packages:
  - 'packages/*'
```

### Each Package's tsconfig.json Pattern

```json
{
  "extends": "../../tsconfig.json",
  "compilerOptions": {
    "baseUrl": ".",
    "paths": {
      "@quent/utils": ["../utils/src/index.ts"],
      "@quent/hooks": ["../hooks/src/index.ts"],
      "@quent/client": ["../client/src/index.ts"],
      "@quent/components": ["../components/src/index.ts"],
      "~quent/types/*": ["../../../examples/simulator/server/ts-bindings/*"]
    }
  },
  "include": ["src"]
}
```

**Note on `~quent/types/*` in package tsconfigs:** Only `@quent/utils` needs this path — it is the only package that imports directly from the bindings. Other packages import from `@quent/utils`. However, TypeScript `extends` inheritance means the paths from `ui/tsconfig.json` propagate; the alias should continue to work without duplication if `tsconfig.json` references are set up correctly. Explicitly define it only in `@quent/utils/tsconfig.json` to make ownership clear.

---

## Agent-Legibility Design

The primary goal from PROJECT.md is: "an agent can read the package exports and assemble a functional UI without reading implementation details." These decisions directly support that:

### Rule 1: index.ts is the complete API surface

Every package's `index.ts` must be self-describing. An agent that reads only `index.ts` should know:
- What components/hooks/functions are available
- What their TypeScript signatures are (via exported types)
- What parameters they require

This means: export types alongside functions. If a hook returns `DAGSelection`, export `DAGSelection` from `index.ts`.

```typescript
// packages/hooks/src/index.ts
export { useDAGSelection } from './hooks/useDAGSelection';
export type { DAGSelection } from './hooks/useDAGSelection'; // type visible at surface
```

### Rule 2: Prop types must be exported from @quent/components

Components with complex prop shapes must export their prop type. An agent building a route that renders `<DAGChart />` needs to know what `DAGChartProps` looks like without reading the component file.

```typescript
// packages/components/src/index.ts
export { DAGChart } from './dag/DAGChart';
export type { DAGChartProps } from './dag/DAGChart';
```

### Rule 3: No default exports

All exports are named. Default exports require the importer to choose a name, creating inconsistency across call sites. Named exports match across all uses.

### Rule 4: Package names are path-prefix-aligned

`@quent/components` contains components. `@quent/hooks` contains hooks. `@quent/client` contains API calls. `@quent/utils` contains utilities. An agent can predict which package to look in without reading all four `index.ts` files.

---

## Anti-Patterns to Avoid

### Anti-Pattern 1: Exporting Atoms from @quent/hooks

**What people do:** Export `selectedNodeIdsAtom` from `@quent/hooks/index.ts` because it's "more flexible."

**Why it's wrong:** Locks the public API to Jotai. Any component that imports an atom must also use `useAtom`, `useAtomValue`, or `useStore`, which means it must understand Jotai's provider model. The hooks abstraction exists precisely to hide this.

**Do this instead:** Export `useDAGSelection()` which returns plain values and setters. Components are decoupled from Jotai entirely.

### Anti-Pattern 2: Wildcard Re-exports

**What people do:** `export * from './dag'` in `index.ts` to avoid maintenance.

**Why it's wrong:** Wildcards make the API surface invisible. TypeScript hover-over works, but reading `index.ts` as a document no longer tells you what the package exports. Breaks agent-legibility.

**Do this instead:** Explicit named re-exports. Accept the maintenance cost.

### Anti-Pattern 3: @quent/hooks Importing from @quent/client

**What people do:** `useBulkTimelines` in `@quent/hooks` imports `fetchBulkTimelines` from `@quent/client` because "hooks manage state and need to fetch."

**Why it's wrong:** Creates a circular dependency if `@quent/client` hooks ever need to read Jotai atoms (which the current `useBulkTimelineFetch` does). Even one-way, it tightly couples state and data layers.

**Do this instead:** Keep orchestration hooks in the app shell. `@quent/hooks` manages state atoms. `@quent/client` manages server state. The app shell wires them together.

### Anti-Pattern 4: CSS in @quent/components

**What people do:** Add `import './button.css'` or an `index.css` to the component package to ship styles with the components.

**Why it's wrong:** Tailwind v4 with `@tailwindcss/vite` generates a single CSS bundle at the app level. Importing CSS from component packages creates duplicate `@import 'tailwindcss'` invocations and conflicting CSS custom property scopes. The current single-CSS-file architecture is correct.

**Do this instead:** Components use only Tailwind utility classes via `cn()`. The app shell's `index.css` is the single stylesheet.

### Anti-Pattern 5: Using @/... Aliases Inside Packages

**What people do:** A component in `@quent/components` imports `import { cn } from '@/lib/utils'` because it worked in `ui/src/`.

**Why it's wrong:** The `@/` alias resolves to `ui/src/` — it escapes the package boundary. This silently works during development (Vite resolves it) but breaks if the package is ever published or used outside the Vite app context.

**Do this instead:** All cross-package imports use the package name: `import { cn } from '@quent/utils'`.

---

## Integration Points

### Boundary: @quent/utils ↔ Rust-generated bindings

| Boundary | Communication | Notes |
|----------|---------------|-------|
| `@quent/utils` → `~quent/types/*` | Re-export (type-only) | Only `@quent/utils` knows the alias; consumers import from `@quent/utils` |
| `@quent/client` → backend API | HTTP via `apiFetch()` | API_BASE_URL stays in `@quent/client`; Vite proxy config in `ui/vite.config.ts` stays unchanged |
| `@quent/components` → `@xyflow/react` | Direct dep in `@quent/components/package.json` | XYFlow must be a dep of the components package, not hoisted-only |
| `@quent/components` → `echarts` | Direct dep | Same rule |
| App shell → TanStack Router | Route loaders call `queryBundleQueryOptions` from `@quent/client` | QueryClient stays in app shell, shared via TanStack's context |

### Internal Boundaries

| Boundary | Communication | Notes |
|----------|---------------|-------|
| `@quent/hooks` atoms ↔ `@quent/components` components | Props + callbacks | Components never import atoms; they receive values and setters via props |
| `@quent/client` query cache ↔ `@quent/hooks` Jotai atoms | Coordinated in app shell | The app shell reads server state from client hooks, writes to hook setters |
| `useBulkTimelines` ↔ Jotai store | `useStore()` + imperative `store.set()` | This pattern is intentional for performance (avoids re-subscribing all timeline rows on zoom) — preserve it in `ui/src/hooks/` |

---

## Sources

- Codebase analysis: `ui/src/atoms/dag.ts`, `ui/src/atoms/timeline.ts` (atom patterns)
- Codebase analysis: `ui/src/hooks/useQueryBundle.ts` (query options + hook pattern)
- Codebase analysis: `ui/src/hooks/useBulkTimelines.ts`, `useBulkTimelineFetch.ts` (orchestration complexity)
- Codebase analysis: `ui/src/services/api.ts` (fetcher structure)
- Codebase analysis: `ui/src/index.css`, `ui/vite.config.ts` (Tailwind v4 single-pass setup)
- Codebase analysis: `ui/tsconfig.json` (path alias structure)
- Codebase analysis: `.planning/PROJECT.md` (agent-legibility requirement, publishability constraint)
- Confidence: HIGH — all patterns grounded in existing code, no speculative external sources required

---

*Architecture research for: Quent UI internal workspace package extraction*
*Researched: 2026-04-01*
