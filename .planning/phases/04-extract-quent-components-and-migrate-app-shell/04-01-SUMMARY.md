---
phase: 04-extract-quent-components-and-migrate-app-shell
plan: "01"
subsystem: ui
tags: [typescript, jotai, react, dag, types, packages]

# Dependency graph
requires:
  - phase: 03-extract-quent-client-and-quent-hooks
    provides: "@quent/hooks baseline with timeline and dag hooks; @quent/utils with ZoomRange and color utilities"
provides:
  - "EntityTypeValue, SingleEntity, EntityRefKey, EntityTypeKey in @quent/utils (entityTypes.ts)"
  - "NodeColoring, EdgeColoring, EdgeWidthConfig, NodeLabelField, NODE_LABEL_FIELD, DAGNode, DAGEdge, StatValue in @quent/utils (dagTypes.ts)"
  - "dagControls atoms (9 atoms) private to @quent/hooks"
  - "useDagNodeColoring, useDagEdgeWidthConfig, useDagEdgeColoring, useOperatorStatFields, usePortStatFields in @quent/hooks (dependency injection pattern)"
  - "useNodeColoring (isDark: boolean) in @quent/hooks — no ThemeContext dependency"
  - "10 dagControl selector hooks in @quent/hooks (HOOKS-02 compliant)"
  - "useDeferredReady in @quent/hooks"
  - "@quent/components package.json configured with all peer/direct deps and sideEffects"
affects: ["04-02", "04-03"]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Dependency injection for compute functions (useDagNodeColoring accepts ComputeNodeColoringFn) — avoids circular dep @quent/hooks → @quent/components → @quent/hooks"
    - "isDark: boolean parameter replaces useTheme() calls in extracted hooks"
    - "Re-export shim pattern: ui/src/types.ts and ui/src/services/query-plan/types.ts become thin wrappers re-exporting from @quent/utils"

key-files:
  created:
    - "ui/packages/@quent/utils/src/entityTypes.ts — EntityTypeValue, SingleEntity, EntityRefKey, EntityTypeKey"
    - "ui/packages/@quent/utils/src/dagTypes.ts — NodeColoring, EdgeColoring, EdgeWidthConfig, NodeLabelField, NODE_LABEL_FIELD, DAGNode, DAGEdge, StatValue"
    - "ui/packages/@quent/hooks/src/atoms/dagControls.ts — 9 private dag control atoms"
    - "ui/packages/@quent/hooks/src/dag/useDagControls.ts — useDagNodeColoring, useDagEdgeWidthConfig, useDagEdgeColoring with dependency injection"
    - "ui/packages/@quent/hooks/src/dag/useNodeColoring.ts — useNodeColoring(operatorId, isDark)"
    - "ui/packages/@quent/hooks/src/dag/useDeferredReady.ts — useDeferredReady"
    - "ui/packages/@quent/hooks/src/dag/dagControlSelectors.ts — 10 selector hooks"
  modified:
    - "ui/packages/@quent/utils/src/index.ts — added entity and DAG type exports"
    - "ui/packages/@quent/hooks/src/index.ts — added DAG control and utility hook exports"
    - "ui/packages/@quent/components/package.json — added deps, peerDeps, sideEffects"
    - "ui/src/types.ts — converted to re-export shim from @quent/utils"
    - "ui/src/services/query-plan/types.ts — converted DAG types to re-exports from @quent/utils"

key-decisions:
  - "DAGNode, DAGEdge, StatValue moved to @quent/utils (not @quent/components) to break the @quent/hooks → @quent/components → @quent/hooks circular dependency"
  - "useDagNodeColoring/useDagEdgeWidthConfig/useDagEdgeColoring accept compute functions via injection — follows Phase 3 useBulkTimelines pattern"
  - "useNodeColoring accepts isDark: boolean instead of calling useTheme() — makes hook usable outside ThemeContext"
  - "ui/src/atoms/dagControls.ts kept intact (not shimmed) since DAG components still in app shell; will be deleted in Plan 04"

patterns-established:
  - "Dependency injection pattern for hooks that need app-layer compute functions"
  - "isDark parameter pattern for theme-aware hooks extracted to packages"

requirements-completed: [COMP-02, COMP-05, COMP-06]

# Metrics
duration: 8min
completed: 2026-04-13
---

# Phase 04 Plan 01: Pre-condition Types and Hooks Migration Summary

**DAG coloring types relocated to @quent/utils and dag control hooks extracted to @quent/hooks using dependency injection, eliminating the circular dependency risk before @quent/components extraction**

## Performance

- **Duration:** 8 min
- **Started:** 2026-04-13T20:21:02Z
- **Completed:** 2026-04-13T20:28:46Z
- **Tasks:** 2
- **Files modified:** 12

## Accomplishments

- Moved entity types (EntityTypeValue, EntityTypeKey, etc.) and DAG coloring types (NodeColoring, EdgeColoring, DAGNode, DAGEdge, etc.) from app-layer files into @quent/utils — eliminating the future circular dep between @quent/hooks and @quent/components
- Extracted 9 private dagControls atoms into @quent/hooks with 10 HOOKS-02-compliant selector hooks; `useDagNodeColoring`, `useDagEdgeWidthConfig`, `useDagEdgeColoring` use dependency injection for compute functions
- Configured @quent/components package.json with all peer/direct deps (elkjs, @xyflow/react, echarts, jotai) and sideEffects for CSS/ECharts — ready for Plan 02 extraction

## Task Commits

Each task was committed atomically:

1. **Task 1: Move entity types and DAG coloring types to @quent/utils** - `3cfd97f1` (feat)
2. **Task 2: Move dagControls atoms, DAG hooks, and useDeferredReady to @quent/hooks; update @quent/components package.json** - `bb410013` (feat)

**Plan metadata:** (docs commit — see below)

## Files Created/Modified

- `ui/packages/@quent/utils/src/entityTypes.ts` - EntityTypeValue, SingleEntity, EntityRefKey, EntityTypeKey
- `ui/packages/@quent/utils/src/dagTypes.ts` - All DAG coloring types, DAGNode, DAGEdge, StatValue, NODE_LABEL_FIELD
- `ui/packages/@quent/utils/src/index.ts` - Appended entity and DAG type exports
- `ui/packages/@quent/hooks/src/atoms/dagControls.ts` - 9 private atoms (selectedColorField, nodeColoringAtom, edgeWidthConfigAtom, edgeColoringAtom, selectedNodeLabelFieldAtom, nodeColorPaletteAtom, edgeColorPaletteAtom, etc.)
- `ui/packages/@quent/hooks/src/dag/useDagControls.ts` - useDagNodeColoring, useDagEdgeWidthConfig, useDagEdgeColoring, useOperatorStatFields, usePortStatFields (with dependency injection)
- `ui/packages/@quent/hooks/src/dag/useNodeColoring.ts` - useNodeColoring(operatorId, isDark: boolean)
- `ui/packages/@quent/hooks/src/dag/useDeferredReady.ts` - useDeferredReady (verbatim copy)
- `ui/packages/@quent/hooks/src/dag/dagControlSelectors.ts` - 10 selector hooks for dag control atoms
- `ui/packages/@quent/hooks/src/index.ts` - Added all new DAG hook exports
- `ui/packages/@quent/components/package.json` - Full dependency config
- `ui/src/types.ts` - Re-export shim from @quent/utils
- `ui/src/services/query-plan/types.ts` - Re-export shim for DAG types from @quent/utils

## Decisions Made

- **DAGNode/DAGEdge moved to @quent/utils** (not @quent/components) — plan's Task 2 REVISED section identified that useDagControls.ts needs these types, and @quent/hooks cannot import @quent/components; moving to @quent/utils was the cleanest resolution
- **Dependency injection for compute functions** — useDagNodeColoring and siblings accept ComputeNodeColoringFn/ComputeEdgeWidthConfigFn/ComputeEdgeColoringFn callbacks, following the established Phase 3 pattern from useBulkTimelines
- **isDark: boolean replaces useTheme()** — extracted useNodeColoring accepts a boolean parameter instead of calling useTheme() from app's ThemeContext, making the hook publishable without app coupling
- **ui/src/atoms/dagControls.ts left intact** — plan determined shimming it would require importing a non-exported subpath; consumers (DAGControls, DAGSettingsPopover, etc.) remain in the app shell and will be updated in Plan 04

## Deviations from Plan

None - plan executed exactly as written (including the REVISED DAGNode/DAGEdge placement in dagTypes.ts).

## Issues Encountered

- pnpm packages (@quent/utils, etc.) were not installed in the worktree's node_modules initially; ran `pnpm install --frozen-lockfile` to link workspace packages before tsc could resolve them.

## Next Phase Readiness

- @quent/utils now has all shared types needed by both @quent/hooks and @quent/components — no circular dep risk for Plan 02-03
- @quent/hooks has all DAG atom and hook infrastructure ready for @quent/components to consume
- @quent/components package.json is configured; Plan 02 can begin extracting DAG, timeline, and tree components immediately

---
*Phase: 04-extract-quent-components-and-migrate-app-shell*
*Completed: 2026-04-13*
