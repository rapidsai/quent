# Phase 4: Extract @quent/components and Migrate App Shell - Research

**Researched:** 2026-04-13
**Domain:** TypeScript monorepo package extraction — React component library, Jotai atoms, ECharts/XYFlow wrappers, Tailwind CSS
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** Page-level compositions stay in `ui/src/` — `QueryPlan.tsx`, `QueryResourceTree.tsx`, `EngineSelectionPage.tsx`, route files, `NavBarNavigator.tsx`. Only reusable, composable components go into `@quent/components`: primitives (`ui/`), DAG components, timeline components, resource-tree components, operator-timeline components, query-plan node components.
- **D-02:** `ThemeContext` stays in the app shell (`ui/src/contexts/`). It is NOT moved to `@quent/components`.
- **D-03:** Components that need dark/light awareness accept an explicit `isDark?: boolean` prop (or equivalent `theme` prop) at their API boundary — they do NOT import from `ThemeContext` directly.
- **D-04:** Internal hook `useTheme` calls within components are replaced at the package boundary with the `isDark` prop. Components that use `useTimelineChartColors` or similar hooks pass the derived values as props or call the hook inside the component with the prop value.
- **D-05:** `@/lib/` and `@/services/query-plan/` utilities move into `@quent/components` as **internal** (non-exported) modules. Specific file mapping documented in CONTEXT.md.
- **D-06:** After extraction, the app shell imports utilities from `@quent/components` ONLY if they are public exports. For utilities that stay internal, the app shell should not need them directly.
- **D-07:** `EntityTypeValue`, `EntityRefKey`, `EntityTypeKey` from `ui/src/types.ts` move to `@quent/utils`. After this, `ui/src/types.ts` becomes a thin re-export shim or is deleted.
- **D-08:** DAGChart gets a controlled-first API: accepts `selectedNodeIds?: string[]` and `onSelectionChange?: (nodeIds: string[]) => void`. Falls back to `@quent/hooks` atoms when controlled props are absent.
- **D-09:** Components in `@quent/components` (v1): all `ui/` primitives, DAG, timeline, query-plan nodes, resource-tree columns, operator-timeline.
- **D-10:** Page-level compositions stay in app shell.
- **D-11:** Source-first dev — `package.json` `"main": "src/index.ts"`. No build step needed in dev loop.
- **D-12:** `peerDependencies` for `@quent/components`: react, @tanstack/react-query, @tanstack/react-router (for any router-aware components), jotai. `@quent/utils` and `@quent/hooks` as direct `dependencies`.
- **D-13:** `resolve.dedupe` in `vite.config.ts` already covers react, jotai, @tanstack/* — no change needed.
- **D-14:** Tailwind `@source` directive in `ui/src/index.css` already covers `ui/packages/**/*.{ts,tsx}` — no change needed.

### Claude's Discretion

- Internal import style within the package: use relative paths (not package barrel) for intra-package imports to avoid circular dependencies.
- Order of extraction: primitives first (no deps), then query-plan utils, then DAG, then timeline, then resource-tree. Each extraction step should leave the app working.
- Whether `ui/src/types.ts` becomes a re-export shim or is deleted — delete is cleaner if all consumers can be updated.
- Whether to export `useTimelineChartColors` from the package or keep it internal — keep internal unless a consumer needs it.

### Deferred Ideas (OUT OF SCOPE)

- Storybook / component catalog — V2
- Per-package README with usage examples — V2
- `QueryPlanTree`, `ResourceTree` as standalone exported components — V2
- npm publishability (exports field, versioning) — V2
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| COMP-01 | All Radix UI + CVA UI primitives extracted from `ui/src/components/ui/` with props interfaces and CVA variant objects exported | Primitives use `cn()` from `@quent/utils` already; no ThemeContext coupling; safe to move as-is with path updates |
| COMP-02 | `DAGChart` extracted with controlled-first API: `selectedNodeIds?: string[]`, `onSelectionChange?` props; falls back to `@quent/hooks` atoms | DAGChart currently uses `useSelectedNodeIds/useSetSelectedNodeIds` from `@quent/hooks`; controlled prop needs to shadow the atom read |
| COMP-03 | `TimelineController` and associated timeline components extracted from `ui/src/components/timeline/` | Timeline components depend on `@/lib/echarts`, `@/lib/timeline.utils`, `useTheme` — all three couplings need resolution |
| COMP-04 | Every exported component has JSDoc comment with purpose, `@param` for non-obvious props, and `@returns` description | Standard TypeScript JSDoc practice; no blockers |
| COMP-05 | Every exported component's props interface is exported alongside the component | Simple naming convention: `DAGChartProps`, `TimelineControllerProps`, etc. |
| COMP-06 | `className?: string` accepted and applied via `cn()` at root element of every visual component | Must audit each component for root element `className` prop application |
| COMP-07 | `index.ts` barrel export lists all public exports by name (no `export *`) | Pattern established in `@quent/utils` and `@quent/hooks` |
| MIG-01 | All `@/components/*`, `@/atoms/*`, `@/lib/*`, `@/services/*` imports in `ui/src/` updated to import from appropriate `@quent/*` package | 101 total `@/` import lines in `ui/src/`; routes/pages import from `@quent/components` after extraction |
| MIG-02 | `vite build` completes without errors; bundle output comparable to pre-refactor baseline | ECharts custom build pattern must stay in `@quent/components`; XYFlow CSS import handled via sideEffects |
| MIG-03 | All existing `vitest` tests pass after migration; no test regressions | 4 test files; `QueryResourceTree.test.tsx` mocks `@/components/ui/tree-table` and `@/hooks/useExpandedIds` — these mocks need path updates after migration |
</phase_requirements>

## Summary

Phase 4 is a file-move + import-rewrite operation. The components are well-isolated from each other and their internal logic is sound. The work is large in file count but mechanically repetitive: move files into the package, rewrite `@/` imports to relative paths or `@quent/*` package imports, then update all app-shell consumers.

Three genuine complexity clusters require attention. First, five components (`DAGChart`, `VariableWidthEdge`, `DAGSettingsPopover`, `DAGLegend`, `useTimelineChartColors`) call `useTheme()` from `@/contexts/ThemeContext` — per D-03/D-04 these must be refactored to accept an `isDark: boolean` prop at the package boundary; the app passes the theme value from its own context. Second, `dagControls.ts` (all visual-only atoms: `nodeColorPaletteAtom`, `edgeColorPaletteAtom`, `selectedColorField`, etc.) and `useDagControls.ts` were deliberately left in the app layer in Phase 3 — but DAGChart/DAGLegend/DAGControls/DAGSettingsPopover all import from `@/atoms/dagControls`, so these atoms and hooks must move to `@quent/hooks` as part of this phase. Third, `operator-timeline/utils.ts` still has `~quent/types` imports for `QueryBundle`, `EntityRef`, `Operator`, `PlanTree` — these must be migrated to `@quent/utils` before the file is moved.

**Primary recommendation:** Execute in layers — (1) move atoms/hooks to `@quent/hooks`, (2) migrate entity types to `@quent/utils`, (3) move utility files to the package, (4) move primitives, (5) move complex components with ThemeContext → isDark prop refactor, (6) update all app-shell imports, (7) update test mocks, (8) build verification.

## Standard Stack

### Core (all already installed in the workspace)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `@xyflow/react` | `^12.10.1` | DAGChart (ReactFlow-based graph layout) | Project dependency; must be `peerDependency` in package |
| `elkjs` | `^0.11.1` | DAG layout algorithm | Already aliased to `elkjs/lib/elk.bundled.js` in vite.config |
| `echarts` | `^5.6.0` | Timeline/Gantt chart rendering | Custom tree-shaken build pattern in `echarts.ts` |
| `echarts-for-react` | `^3.0.6` | React wrapper for ECharts | Used by `Timeline`, `TimelineController`, `OperatorGanttChart` |
| `jotai` | `^2.0.0` | Atom state (dagControls atoms moving to `@quent/hooks`) | Already a peerDependency in `@quent/hooks` |
| `@quent/utils` | workspace | `cn()`, colors, formatters, Rust types | Direct dependency |
| `@quent/hooks` | workspace | Timeline/DAG atom hooks | Direct dependency |

### Package.json Updates Required

Current `@quent/components/package.json` is minimal (only `react` peer, no external deps). It needs:

```json
{
  "peerDependencies": {
    "react": "^19.0.0",
    "react-dom": "^19.0.0",
    "@xyflow/react": "^12.10.1",
    "echarts": "^5.6.0",
    "echarts-for-react": "^3.0.6",
    "jotai": "^2.0.0",
    "@tanstack/react-query": "^5.0.0"
  },
  "dependencies": {
    "@quent/utils": "workspace:*",
    "@quent/hooks": "workspace:*"
  }
}
```

The `elkjs` dependency is consumed inside DAGChart only — it can stay as a direct dependency of `@quent/components` (not a peer) since consumers don't need to control its version. The vite.config alias `elkjs -> elkjs/lib/elk.bundled.js` applies app-wide so the package inherits it automatically.

### ECharts CSS Side Effects

The ECharts `echarts.ts` custom build file (`lib/echarts.ts`) uses `echarts.use([...])` which registers modules globally. When moved to the package, this file has no CSS imports — it is pure JS side effects via `echarts.use()`. The `sideEffects: false` in `@quent/components/package.json` must be changed to `["**/echarts.ts"]` or `true` to prevent tree-shaking from dropping the `echarts.use()` registration call.

### XYFlow CSS Import

`DAGChart.tsx` has `import '@xyflow/react/dist/style.css'`. This CSS import works in Vite's bundler because Vite handles CSS imports in JS. When the file moves to the package (still processed by the app's Vite instance), this import will continue to work — no change needed. The `sideEffects` issue above applies here too if the package is ever built with tsup, but for source-first dev it is a non-issue.

## Architecture Patterns

### Recommended Package Structure

```
ui/packages/@quent/components/src/
├── ui/                          # Primitive components (Button, Card, etc.)
│   ├── button.tsx
│   ├── card.tsx
│   └── ...
├── dag/                         # DAG chart components
│   ├── DAGChart.tsx
│   ├── DAGControls.tsx
│   ├── DAGLegend.tsx
│   └── DAGSettingsPopover.tsx
├── timeline/                    # Timeline chart components
│   ├── Timeline.tsx
│   ├── TimelineController.tsx
│   ├── TimelineSkeleton.tsx
│   ├── TimelineToolbar.tsx
│   ├── TimelineTooltip.tsx
│   ├── ResourceTimeline.tsx
│   ├── useTimelineChartColors.ts
│   └── types.ts
├── query-plan/                  # Query plan node components
│   ├── QueryPlanNode.tsx
│   └── OperatorStatisticsPopup.tsx
├── resource-tree/               # Resource tree column components
│   ├── ResourceColumn.tsx
│   ├── ResourceGroupRow.tsx
│   ├── ResourceRow.tsx
│   ├── InlineSelector.tsx
│   ├── UsageColumn.tsx
│   └── types.ts
├── operator-timeline/           # Operator Gantt chart
│   ├── OperatorGanttChart.tsx
│   ├── types.ts
│   └── utils.ts
├── lib/                         # Internal utilities (NOT exported)
│   ├── echarts.ts
│   ├── timeline.utils.ts
│   ├── resource.utils.ts
│   └── queryBundle.utils.ts
├── services/                    # Internal services (NOT exported)
│   └── query-plan/
│       ├── dagFieldProcessing.ts
│       ├── query-bundle-transformer.ts
│       ├── operationTypes.ts
│       └── types.ts
└── index.ts                     # Public barrel (named exports only)
```

### Pattern 1: ThemeContext → isDark Prop Refactor

**What:** Components that called `useTheme()` from `@/contexts/ThemeContext` must accept `isDark: boolean` as a prop instead.

**Affected components:**
- `DAGChart.tsx` → `VariableWidthEdge` sub-component needs `isDark: boolean` (passed from `DAGChart` which receives it as prop)
- `DAGLegend.tsx` → `ContinuousLegend` sub-component needs `isDark: boolean`
- `DAGSettingsPopover.tsx` → needs `isDark: boolean`
- `useTimelineChartColors.ts` → refactor signature to accept `isDark: boolean` instead of calling `useTheme()`

**Pattern (verified from existing code):**
```typescript
// BEFORE (app-coupled):
import { useTheme, THEME_DARK } from '@/contexts/ThemeContext';
export const DAGLegend = () => {
  const { theme } = useTheme();
  const isDarkMode = theme === THEME_DARK;
  ...
};

// AFTER (package-clean):
export interface DAGLegendProps {
  isDark?: boolean;
  className?: string;
}
export const DAGLegend = ({ isDark = false }: DAGLegendProps) => {
  // use isDark directly
};
```

**App-side usage (in QueryPlan.tsx):**
```typescript
import { useTheme, THEME_DARK } from '@/contexts/ThemeContext';
const { theme } = useTheme();
const isDark = theme === THEME_DARK;
// Pass down:
<DAGChart isDark={isDark} ... />
```

### Pattern 2: dagControls Atoms → @quent/hooks

**What:** The `@/atoms/dagControls.ts` file contains visual-only DAG atoms that serve as internal state for the extracted DAG components. Per D-09, DAGChart/DAGLegend/DAGControls/DAGSettingsPopover all go into the package, and they all read/write these atoms. The atoms must move to `@quent/hooks` as internal atoms (HOOKS-02: no raw atom exports).

**Atoms to move:**
```typescript
// From ui/src/atoms/dagControls.ts → @quent/hooks/src/atoms/dagControls.ts
export const selectedColorField = atom<string | null>(null);
export const nodeColoringAtom = atom<NodeColoring>(null);
export const selectedEdgeWidthFieldAtom = atom<string | null>(null);
export const edgeWidthConfigAtom = atom<EdgeWidthConfig>(null);
export const selectedEdgeColorFieldAtom = atom<string | null>(null);
export const edgeColoringAtom = atom<EdgeColoring>(null);
export const selectedNodeLabelFieldAtom = atom<NodeLabelField>(NODE_LABEL_FIELD.NAME);
export const nodeColorPaletteAtom = atom<ContinuousPaletteName>('blue');
export const edgeColorPaletteAtom = atom<ContinuousPaletteName>('teal');
```

**Hooks to move:**
```typescript
// From ui/src/hooks/useDagControls.ts → @quent/hooks/src/dag/useDagControls.ts
// From ui/src/hooks/useNodeColoring.ts → @quent/hooks/src/dag/useNodeColoring.ts
```

**Exports:** Hooks are exported from `@quent/hooks/src/index.ts`. Atoms remain private. The types `NodeLabelField`, `NODE_LABEL_FIELD` that DAGControls needs must also be accessible — they can live in the atoms file and be exported as types only from `@quent/hooks`.

### Pattern 3: Internal Imports within Package

**What:** Once all files are in the package, intra-package imports use relative paths, NOT `@quent/components` barrel. This avoids circular dependencies and keeps the package self-contained.

**Example:**
```typescript
// In @quent/components/src/dag/DAGChart.tsx:
import { QueryPlanNode } from '../query-plan/QueryPlanNode';  // relative
import { nanosToMs } from '../lib/timeline.utils';            // relative (internal)
import { cn } from '@quent/utils';                            // package import (external dep)
import { useSelectedNodeIds } from '@quent/hooks';            // package import (external dep)
```

### Pattern 4: @quent/utils Entity Types Migration

**What:** `EntityTypeValue`, `EntityRefKey`, `EntityTypeKey` from `ui/src/types.ts` move to `@quent/utils`.

**Current consumers (must update after move):**
- `ui/src/lib/resource.utils.ts` → imports `EntityTypeKey`
- `ui/src/lib/queryBundle.utils.ts` → imports `EntityRefKey`
- `ui/src/components/resource-tree/UsageColumn.tsx` → imports `EntityTypeKey`
- `ui/src/components/resource-tree/types.ts` → imports `EntityTypeValue`
- `ui/src/components/timeline/ResourceTimeline.tsx` → imports `EntityTypeKey`
- `ui/src/components/QueryResourceTree.tsx` → imports `EntityRefKey`

**Target location:** `ui/packages/@quent/utils/src/entityTypes.ts` (new file), exported from `@quent/utils/src/index.ts`.

### Pattern 5: ~quent/types → @quent/utils Migration

**What:** `ui/src/components/operator-timeline/utils.ts` still has `~quent/types/QueryBundle`, `~quent/types/EntityRef`, `~quent/types/Operator`, `~quent/types/PlanTree` imports. These must be migrated to `@quent/utils` imports before the file is moved to the package (the package's tsconfig has no `~quent/types` path alias).

**Verified available in `@quent/utils`:** All Rust-generated types are re-exported via `@quent/utils` (UTILS-02 complete). QueryBundle, EntityRef, Operator, PlanTree are available as `import type { QueryBundle, EntityRef, Operator, PlanTree } from '@quent/utils'`.

### Pattern 6: index.ts Barrel Export Structure

Based on the established `@quent/hooks` and `@quent/utils` barrel pattern:
```typescript
// ui/packages/@quent/components/src/index.ts — named exports only, no export *

// Primitives
export { Button, type ButtonProps } from './ui/button';
export { Card, CardContent, CardHeader, type CardProps } from './ui/card';
// ... all 16 primitives

// DAG components
export { DAGChart, type DAGChartProps } from './dag/DAGChart';
export { DAGControls, type DAGControlsProps } from './dag/DAGControls';
export { DAGLegend, type DAGLegendProps } from './dag/DAGLegend';
export { DAGSettingsPopover, type DAGSettingsPopoverProps } from './dag/DAGSettingsPopover';

// Timeline components
export { Timeline, type TimelineProps } from './timeline/Timeline';
export { TimelineController, type TimelineControllerProps } from './timeline/TimelineController';
// ... etc

// Types that consumers need
export type { DAGData, DAGNode, DAGEdge, NodeColoring, EdgeColoring, StatValue } from './services/query-plan/types';
```

Internal utilities (`lib/`, `services/`) are NOT re-exported.

### Anti-Patterns to Avoid

- **`export *` in barrel:** The planner MUST use named exports. `export *` breaks tree-shaking and masks the public API surface.
- **Circular imports:** If `timeline/Timeline.tsx` imports from `operator-timeline/`, there will be a circular dep. Keep the dependency direction: `operator-timeline` → `timeline` (OperatorGanttChart imports `CHART_GROUP` from `Timeline.tsx` and `useTimelineChartColors` — this is fine as long as `timeline/` doesn't import from `operator-timeline/`).
- **Moving `QueryPlan.tsx`, `QueryResourceTree.tsx` to the package:** These are page-level compositions (D-01/D-10). They stay in `ui/src/components/`.
- **Moving ThemeContext:** It stays in the app (D-02).
- **Exporting internal atoms from `@quent/hooks`:** Raw atom exports break the abstraction boundary (HOOKS-02).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| DAG graph layout | Custom layout algorithm | ELK (`elkjs/lib/elk.bundled.js`) | Already integrated; ELK handles layered layout with spacing options |
| DAG node rendering | Custom SVG/canvas graph | `@xyflow/react` ReactFlow | Already integrated; handles pan/zoom/minimap/fit-view |
| Timeline chart | Custom canvas timeline | `echarts-for-react` + custom `echarts.ts` build | Already integrated; tree-shaken custom build reduces bundle ~67% |
| Class merging for components | String concatenation | `cn()` from `@quent/utils` | Handles Tailwind conflict resolution via `tailwind-merge` |
| Component theming for ECharts | CSS variables | Computed props from `useTimelineChartColors` | ECharts is canvas-based and ignores CSS variables; all colors must be computed |

## Common Pitfalls

### Pitfall 1: sideEffects Field in package.json

**What goes wrong:** Tree-shaker eliminates `echarts.use([...])` calls in `echarts.ts`, causing "Series type not registered" runtime errors in production builds.

**Why it happens:** `@quent/components/package.json` currently has `"sideEffects": false`. The `echarts.ts` file has side effects (calling `echarts.use()`). When Vite/Rollup sees `sideEffects: false`, it may eliminate the echarts registration calls in the production bundle.

**How to avoid:** Change `package.json` to `"sideEffects": ["**/echarts.ts", "**/*.css"]`. This tells bundlers those files have side effects that must be preserved. Verify in `vite build` output that the echarts chunk appears.

**Warning signs:** `vite preview` shows blank chart areas; browser console has "Series [line] is not installed" or similar ECharts errors.

### Pitfall 2: `@/atoms/dag` Still Referenced in OperatorGanttChart

**What goes wrong:** `OperatorGanttChart.tsx` imports `selectedNodeIdsAtom, selectedOperatorLabelAtom, selectedPlanIdAtom` from `@/atoms/dag`. These are atoms that are now in `@quent/hooks` (internal). The component should use the hooks (`useSelectedNodeIds`, etc.) instead of direct atom imports.

**Why it happens:** The Phase 3 merge brought `OperatorGanttChart` in, but it was not fully migrated to use the `@quent/hooks` hook API.

**How to avoid:** Replace all `useAtomValue(selectedNodeIdsAtom)` with `useSelectedNodeIds()` from `@quent/hooks` before moving to the package.

### Pitfall 3: ThemeContext Coupling Breaks Package Independence

**What goes wrong:** If `useTimelineChartColors.ts` is moved to the package without refactoring (it currently calls `useTheme()` from `@/contexts/ThemeContext`), the package gains a transitive dependency on the app shell's context.

**Why it happens:** The `@/contexts/ThemeContext` path alias resolves correctly in the app, but the `@quent/components` package has no `@/` alias configured in its tsconfig — so `tsc --noEmit` on the package will fail immediately.

**How to avoid:** Refactor `useTimelineChartColors` signature before moving: replace `const { theme } = useTheme()` with `isDark: boolean` parameter. Components that call `useTimelineChartColors()` receive `isDark` prop and pass it to the hook.

### Pitfall 4: `services/query-plan/types.ts` Has a Dependency on `tree-view` Primitive

**What goes wrong:** `ui/src/services/query-plan/types.ts` imports `TreeDataItem` from `@/components/ui/tree-view`:
```typescript
import type { TreeDataItem } from '@/components/ui/tree-view';
export interface QueryPlanDataItem extends TreeDataItem { ... }
```
When this types file moves to `services/query-plan/types.ts` inside the package, the `@/components/ui/tree-view` import must become a relative path to `../../ui/tree-view`.

**Why it happens:** The import worked fine in the app because `@/components/ui/tree-view` resolved to the same file. In the package, the path alias no longer applies.

**How to avoid:** When moving `services/query-plan/types.ts`, immediately update the import to `../../ui/tree-view` (relative within the package).

### Pitfall 5: Test Mocks Use `@/` Paths That Change After Migration

**What goes wrong:** `QueryResourceTree.test.tsx` contains:
```typescript
vi.mock('@/hooks/useExpandedIds', ...)
vi.mock('@/components/ui/tree-table', ...)
```
After migration, `TreeTable` comes from `@quent/components` and `useExpandedIds` may move or stay. If the mock paths are not updated, the mocks won't intercept the correct module, causing test failures.

**Why it happens:** Vitest module mocking is path-sensitive — the mocked path must match exactly what the component-under-test imports.

**How to avoid:** After moving `TreeTable` to `@quent/components` and updating `QueryResourceTree.tsx` to import from `@quent/components`, update the mock in the test to:
```typescript
vi.mock('@quent/components', async importOriginal => {
  const actual = await importOriginal();
  return { ...actual, TreeTable: ({ columns }) => ... };
});
```

### Pitfall 6: `useQueryPlanVisualization` and `useDagControls` Import from Moved Files

**What goes wrong:** `ui/src/hooks/useQueryPlanVisualization.ts` imports from `@/services/query-plan/types` and `@/services/query-plan/query-bundle-transformer`. After those files move to `@quent/components`, these hooks in the app need to import from `@quent/components`.

**How to avoid:** Ensure that `QueryPlanDataItem`, `DAGNode`, `DAGEdge` types and `getTreeData`/`getPlanDAG` functions are exported from `@quent/components` index if any app-layer code needs them. Based on D-06, if page-level code calls these directly, they must be exported.

### Pitfall 7: `resource.utils.ts` Imports `TreeTableItem` from Component

**What goes wrong:** `ui/src/lib/resource.utils.ts` imports `TreeTableItem` from `@/components/resource-tree/types`. When both files move to the package, internal import paths must be updated.

**How to avoid:** Use relative imports within the package (`../resource-tree/types` from `lib/resource.utils.ts`).

## Code Examples

### DAGChart Controlled API Pattern (COMP-02)

```typescript
// Source: CONTEXT.md D-08, verified against current DAGChart.tsx

export interface DAGChartProps {
  data: DAGData;
  height?: string;
  isDark?: boolean;
  /** Controlled selection: when provided, component uses these instead of @quent/hooks atoms */
  selectedNodeIds?: string[];
  /** Called when user clicks a node; only fires in controlled mode */
  onSelectionChange?: (nodeIds: string[]) => void;
  className?: string;
}

export const DAGChart = ({
  data,
  height = '100%',
  isDark = false,
  selectedNodeIds: controlledNodeIds,
  onSelectionChange,
  className,
}: DAGChartProps) => {
  // Atom-backed (uncontrolled) hooks:
  const atomNodeIds = useSelectedNodeIds();
  const setAtomNodeIds = useSetSelectedNodeIds();

  // If controlled props provided, use them; otherwise fall back to atoms:
  const isControlled = controlledNodeIds !== undefined;
  const activeNodeIds = isControlled ? new Set(controlledNodeIds) : atomNodeIds;
  const handleSelection = isControlled
    ? (ids: Set<string>) => onSelectionChange?.(Array.from(ids))
    : (ids: Set<string>) => setAtomNodeIds(ids);
  ...
};
```

### ThemeContext Decoupling for useTimelineChartColors

```typescript
// BEFORE (current, in ui/src/components/timeline/useTimelineChartColors.ts):
export function useTimelineChartColors() {
  const { theme } = useTheme();          // <-- app-shell coupling
  return useMemo(() => {
    const isDark = theme === THEME_DARK;
    ...
  }, [theme]);
}

// AFTER (in @quent/components/src/timeline/useTimelineChartColors.ts):
export function useTimelineChartColors(isDark: boolean) {
  return useMemo(() => {
    // use isDark directly
    const timelineMarkupColor = isDark ? TIMELINE_MARKUP_COLOR_DARK : TIMELINE_MARKUP_COLOR;
    ...
  }, [isDark]);
}
```

### Moving dagControls Atoms to @quent/hooks

```typescript
// New file: ui/packages/@quent/hooks/src/atoms/dagControls.ts
// (types imported from @quent/utils/components services once moved)
import { atom } from 'jotai';
import type { NodeColoring, EdgeColoring, EdgeWidthConfig } from '@quent/components';  // WRONG
// Actually: types stay in package; hooks import from the package? No — circular.
// Correct approach: dagControls types (NodeColoring, EdgeWidthConfig, EdgeColoring)
// live in @quent/components/src/services/query-plan/types.ts.
// Atoms in @quent/hooks that reference those types create a circular dep:
//   @quent/hooks → @quent/components → @quent/hooks (peerDep, not direct)
// RESOLUTION: Move NodeColoring/EdgeColoring/EdgeWidthConfig types to @quent/utils
// (they have no React dependency). This is the cleanest solution.
```

**This is a critical finding.** See Open Questions #1 for full analysis.

### Barrel Export Pattern (COMP-07)

```typescript
// ui/packages/@quent/components/src/index.ts
// Pattern from @quent/utils and @quent/hooks — named only, no export *

export { Button } from './ui/button';
export type { ButtonProps } from './ui/button';
export { DAGChart } from './dag/DAGChart';
export type { DAGChartProps } from './dag/DAGChart';
// ... etc
```

## Don't Hand-Roll (Supplemental)

The type coupling between `dagControls.ts` atoms and the `types.ts` types is the highest-risk area. Rather than inventing a new pattern, move `NodeColoring`, `EdgeWidthConfig`, `EdgeColoring`, `NodeLabelField`, `NODE_LABEL_FIELD` types to `@quent/utils` so both `@quent/hooks` (atoms) and `@quent/components` (components) can import them without circular dependency.

## Open Questions

### 1. dagControls Atom Types — Circular Dependency Risk (CRITICAL)

**What we know:** `dagControls.ts` atoms are typed with `NodeColoring`, `EdgeWidthConfig`, `EdgeColoring` from `@/services/query-plan/types`. These types will move to `@quent/components/src/services/query-plan/types.ts`. The atoms need to move to `@quent/hooks`. But `@quent/hooks` cannot import from `@quent/components` because `@quent/components` lists `@quent/hooks` as a direct dependency — creating a circular dependency.

**What's unclear:** Where exactly to put these pure-data types so both packages can use them.

**Recommendation:** Move `NodeColoring`, `EdgeWidthConfig`, `EdgeColoring`, `NodeLabelField`, and `NODE_LABEL_FIELD` from `services/query-plan/types.ts` to `@quent/utils` (e.g., `dagTypes.ts`). These are pure TypeScript interfaces with no React dependency. Then:
- `@quent/hooks/src/atoms/dagControls.ts` imports from `@quent/utils`
- `@quent/components/src/services/query-plan/types.ts` re-exports or imports from `@quent/utils`
- No circular dependency

### 2. useDeferredReady Destination

**What we know:** `ResourceTimeline.tsx` imports `useDeferredReady` from `@/hooks/useDeferredReady`. This is a generic utility hook (uses `requestIdleCallback` to defer a boolean). `ResourceTimeline` is being extracted to `@quent/components`.

**What's unclear:** Should `useDeferredReady` move to `@quent/hooks`, or is it simple enough to inline/duplicate inside `@quent/components`?

**Recommendation:** Move to `@quent/hooks` as it is a clean, stateless hook with no app-shell coupling. Export it from `@quent/hooks/index.ts`. This is consistent with the pattern of hooks in `@quent/hooks`.

### 3. useExpandedIds and useQueryPlanVisualization — App-Layer Hooks

**What we know:** `QueryResourceTree.tsx` (stays in app) imports `useExpandedIds`. `QueryPlan.tsx` (stays in app) imports `useQueryPlanVisualization`. These hooks import from `@/services/query-plan/*` which moves to the package. After extraction, these app-layer hooks need to import from `@quent/components`.

**Recommendation:** Keep both hooks in `ui/src/hooks/` (they are page-level orchestration hooks, not reusable component hooks). Update their imports from `@/services/query-plan/*` to `@quent/components` after extraction. This requires that `DAGNode`, `DAGEdge`, `QueryPlanDataItem`, `getTreeData`, and `getPlanDAG` are all exported from `@quent/components` index.

### 4. `queryClient.ts` in `ui/src/lib/`

**What we know:** Routes import `queryClient` from `@/lib/queryClient`. This file is NOT in the D-05 migration list (correctly — it's app infrastructure, not a component utility).

**Recommendation:** Leave `ui/src/lib/queryClient.ts` in place. Routes import it directly. No change needed.

## Environment Availability

Step 2.6: SKIPPED (no external tool dependencies beyond the project's own pnpm workspace; all libraries are already installed in `ui/node_modules`).

The `vite build` and `pnpm test` commands are the only execution dependencies, and both are available in the current environment.

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Vitest (v3.x, via workspace config) |
| Config file | `ui/vitest.config.ts` (app tests); `ui/vitest.workspace.ts` (workspace root) |
| Quick run command | `cd ui && pnpm test -- --run --reporter=verbose 2>&1 \| tail -30` |
| Full suite command | `cd ui && pnpm test -- --run` |
| Build check command | `cd ui && pnpm build` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| COMP-01 | UI primitives import from `@quent/components` | smoke | `grep -r "from '@quent/components'" ui/src/ --include="*.tsx" --include="*.ts"` | manual grep |
| COMP-02 | DAGChart controlled API works | manual/visual | n/a — no existing DAGChart unit test | ❌ Wave 0 gap (optional) |
| COMP-03 | Timeline renders without ThemeContext | smoke | `cd ui && pnpm build` | existing build |
| COMP-04 | JSDoc comments present | manual | code review | manual |
| COMP-05 | Props interfaces exported | type check | `cd ui && pnpm typecheck` or `tsc --noEmit` | existing typecheck |
| COMP-06 | className prop at root | manual | code review | manual |
| COMP-07 | index.ts has all named exports | smoke | verify by importing in test | manual |
| MIG-01 | No `@/components/*`, `@/atoms/*`, `@/lib/*`, `@/services/*` in `ui/src/` | automated grep | `grep -rn "@/components\|@/atoms\|@/lib\|@/services" ui/src/ --include="*.tsx" --include="*.ts" \| grep -v "node_modules"` | automated |
| MIG-02 | vite build completes | build check | `cd ui && pnpm build 2>&1 \| tail -20` | existing build script |
| MIG-03 | All vitest tests pass | unit | `cd ui && pnpm test -- --run` | ✅ `QueryResourceTree.test.tsx` exists |

### Sampling Rate

- **Per task commit:** `cd /Users/johallaron/Projects/quent/ui && pnpm typecheck 2>&1 | tail -20`
- **Per wave merge:** `cd /Users/johallaron/Projects/quent/ui && pnpm test -- --run 2>&1 | tail -20`
- **Phase gate:** Full suite green + `pnpm build` passes + grep for `@/` imports returns zero results in `ui/src/`

### Wave 0 Gaps

- No new test files required — existing tests cover the critical behavior (`QueryResourceTree.test.tsx`).
- The mock paths in `QueryResourceTree.test.tsx` must be updated as part of MIG-03 (not a new file but a modification).

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `~quent/types/*` path alias | `@quent/utils` re-exports | Phase 2 (UTILS-02) | operator-timeline/utils.ts still uses old alias — must be updated |
| `atomFamily` for per-query atoms | Record-based atoms (`atom<Record<string, T>>`) | Phase 3 (HOOKS-01) | DAG atoms use plain atoms (not atomFamily), which is correct |
| `@/lib/utils.ts` for `cn()` | `@quent/utils` | Phase 2 (UTILS-01) | All primitives already use `cn()` from `@quent/utils` — no work needed |

## Sources

### Primary (HIGH confidence)

- Direct source code inspection: `ui/src/components/`, `ui/src/atoms/`, `ui/src/hooks/`, `ui/src/lib/`, `ui/packages/@quent/`
- CONTEXT.md decisions D-01 through D-14 (verified against implementation)
- Vite config alias and dedupe configuration (verified at `ui/vite.config.ts`)
- Tailwind `@source` directive (verified at `ui/src/index.css` line 5)
- Test infrastructure (verified at `ui/vitest.workspace.ts`, `ui/vitest.config.ts`, `ui/src/test/setup.ts`)

### Secondary (MEDIUM confidence)

- ECharts `sideEffects` behavior: standard Rollup/Vite tree-shaking behavior; documented in Rollup docs and ECharts handbook. Pattern: any file with `echarts.use()` is a side-effect file.
- XYFlow CSS import in monorepo: standard Vite behavior for CSS imports in workspace packages processed by app's Vite instance.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all versions directly verified from `ui/package.json`
- Architecture: HIGH — all patterns verified from existing code and CONTEXT.md decisions
- Pitfalls: HIGH — identified from direct code inspection of import patterns and type dependencies
- Circular dependency risk: HIGH — verified by tracing dagControls atom types through the dependency graph

**Research date:** 2026-04-13
**Valid until:** 2026-05-13 (stable libraries, internal codebase — 30 day horizon)
