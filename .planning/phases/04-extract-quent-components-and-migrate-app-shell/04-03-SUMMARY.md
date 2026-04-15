---
phase: 04-extract-quent-components-and-migrate-app-shell
plan: "03"
subsystem: ui
tags: [react, typescript, vite, vitest, quent-components, quent-hooks, quent-utils, quent-client, migration]

requires:
  - phase: 04-02
    provides: "@quent/components barrel export with all UI primitives, DAG, timeline, query-plan, resource-tree, operator-timeline components and utils"
  - phase: 04-01
    provides: "@quent/hooks with useDagNodeColoring, useDagEdgeWidthConfig, useDagEdgeColoring, useOperatorStatFields, usePortStatFields, useDeferredReady"

provides:
  - "App shell imports exclusively from @quent/* packages — zero @/components/ui, @/components/dag, @/components/timeline, @/atoms, @/services/query-plan imports"
  - "All old source files deleted from ui/src/ (ui, dag, timeline, resource-tree, query-plan, operator-timeline directories)"
  - "vite build passes (MIG-02)"
  - "All 37 vitest tests pass (MIG-03)"
  - "MIG-01 grep returns zero results"
  - "@quent/components barrel extended with DropdownMenu, NavigationMenu, Select exports"

affects:
  - downstream consumers of ui/ build artifact
  - any agent reading ui/src/ for implementation patterns (now clean package-based imports)

tech-stack:
  added: []
  patterns:
    - "isDark: boolean passed as prop from ThemeContext (useTheme hook) to components that need dark mode awareness"
    - "Dependency injection for compute functions: useDagNodeColoring(nodes, computeNodeColoring) avoids circular deps"
    - "ThemeContext mocked in tests that render components using useTheme()"
    - "Lazy DAGChart import via import('@quent/components') with .then(mod => ({ default: mod.DAGChart })) preserves code splitting"

key-files:
  created: []
  modified:
    - "ui/packages/@quent/components/src/index.ts — added DropdownMenu, NavigationMenu, Select barrel exports"
    - "ui/packages/@quent/components/src/dag/DAGChart.tsx — fixed VariableWidthEdgeProps type (removed spurious isDark field)"
    - "ui/packages/@quent/utils/src/colors.ts — removed duplicate function declarations"
    - "ui/src/components/QueryPlan.tsx — migrated to @quent/* imports; useTheme + isDark; injected compute fns"
    - "ui/src/components/QueryResourceTree.tsx — migrated to @quent/* imports; isDark passed to TimelineController, OperatorGanttChart, UsageColumn"
    - "ui/src/components/NavBarNavigator.tsx — DropdownMenu, DataText from @quent/components"
    - "ui/src/components/ThemeToggle.tsx — Button from @quent/components"
    - "ui/src/pages/EngineSelectionPage.tsx — Select from @quent/components"
    - "ui/src/routes/__root.tsx — Button, NavigationMenu from @quent/components"
    - "ui/src/routes/profile.engine.$engineId.tsx — Resizable from @quent/components"
    - "ui/src/routes/profile.engine.$engineId.query.$queryId.tsx — @quent/client, @quent/utils (drops ~quent/types and @/hooks/useQueryBundle)"
    - "ui/src/hooks/useQueryPlanVisualization.ts — @quent/components for QueryPlanDataItem, DAGData, getTreeData, getPlanDAG"
    - "ui/src/components/QueryResourceTree.test.tsx — single @quent/components mock; added ThemeContext mock"

key-decisions:
  - "isDark boolean computed from useTheme() in QueryPlan and QueryResourceTree, then passed explicitly to components — maintains ThemeContext in app shell only, components stay decoupled"
  - "useDagNodeColoring/useDagEdgeWidthConfig/useDagEdgeColoring called with compute functions as arguments (dependency injection pattern) — avoids @quent/hooks -> @quent/components circular dep"
  - "DropdownMenu, NavigationMenu, Select added to @quent/components barrel — were present in package but missing from index.ts"
  - "Test mocks consolidated into single vi.mock('@quent/components') with spread-then-override pattern instead of scattered relative path mocks"

requirements-completed: [MIG-01, MIG-02, MIG-03]

duration: 42min
completed: "2026-04-13"
---

# Phase 04 Plan 03: Final App-Shell Migration Summary

**App shell exclusively imports from @quent/* packages; 55 old source files deleted; build and all 37 tests green; MIG-01/02/03 satisfied**

## Performance

- **Duration:** 42 min
- **Started:** 2026-04-13T21:27:16Z
- **Completed:** 2026-04-13T22:09:00Z
- **Tasks:** 2
- **Files modified:** 13 app-shell files + 3 package files modified; 52 source files deleted

## Accomplishments

- Migrated all app-shell imports to `@quent/*` packages — zero `@/components/ui`, `@/atoms`, `@/services/query-plan` imports remain
- Deleted all extracted source directories and files from `ui/src/` (55 files across 6 component directories and lib/services/hooks)
- Extended `@quent/components` barrel with 3 missing UI families: DropdownMenu (14 exports), NavigationMenu (9 exports), Select (10 exports)
- Fixed 3 pre-existing bugs introduced in Plan 02 that blocked the build: duplicate functions in colors.ts, wrong VariableWidthEdgeProps interface, missing ThemeContext mock in test

## Task Commits

Each task was committed atomically:

1. **Task 1: Migrate all app-shell imports from @/ to @quent/* packages** - `7aa363d4` (feat)
2. **Task 2: Delete old source files and run full build + test verification** - `2ec2c643` (chore)

**Plan metadata:** (committed after summary creation)

## Files Created/Modified

- `ui/packages/@quent/components/src/index.ts` - Added DropdownMenu, NavigationMenu, Select barrel exports
- `ui/packages/@quent/components/src/dag/DAGChart.tsx` - Fixed VariableWidthEdgeProps type (Rule 1 bug fix)
- `ui/packages/@quent/utils/src/colors.ts` - Removed duplicate function declarations (Rule 1 bug fix)
- `ui/src/components/QueryPlan.tsx` - All @/components/* imports → @quent/components; useDagControls hooks now use dependency injection; isDark passed to DAGChart/DAGControls
- `ui/src/components/QueryResourceTree.tsx` - All @/components/* and @/lib/* imports → @quent/components; isDark passed to TimelineController/OperatorGanttChart/UsageColumn
- `ui/src/components/NavBarNavigator.tsx` - DropdownMenu, DataText from @quent/components
- `ui/src/components/ThemeToggle.tsx` - Button from @quent/components
- `ui/src/pages/EngineSelectionPage.tsx` - Select from @quent/components
- `ui/src/routes/__root.tsx` - Button, NavigationMenu from @quent/components
- `ui/src/routes/profile.engine.$engineId.tsx` - Resizable from @quent/components
- `ui/src/routes/profile.engine.$engineId.query.$queryId.tsx` - queryBundleQueryOptions from @quent/client; types from @quent/utils
- `ui/src/hooks/useQueryPlanVisualization.ts` - QueryPlanDataItem, DAGData, getTreeData, getPlanDAG from @quent/components
- `ui/src/components/QueryResourceTree.test.tsx` - Consolidated mocks; added ThemeContext mock
- **Deleted:** 52 files across ui/src/components/ui, dag, timeline, resource-tree, query-plan, operator-timeline, lib/{echarts,timeline.utils,resource.utils,queryBundle.utils}.ts, services/query-plan/, atoms/dagControls.ts, hooks/{useDagControls,useNodeColoring,useDeferredReady}.ts, types.ts

## Decisions Made

- isDark boolean computed in QueryPlan/QueryResourceTree via `useTheme()` then passed explicitly to components — ThemeContext stays in app shell only, package components get a simple boolean
- Dependency injection maintained for compute functions per Phase 04-01/02 pattern (useDagNodeColoring takes computeNodeColoring as arg)
- Test consolidated to single `vi.mock('@quent/components')` with spread-then-override rather than scattered relative path mocks

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Duplicate function declarations in @quent/utils/src/colors.ts**
- **Found during:** Task 2 (running full build)
- **Issue:** `createCapacitiesColorFn`, `createFsmTypeColorFn`, `buildFsmStateIndexMap` were declared twice in colors.ts — copy-paste error from Plan 02 extraction. TypeScript reported TS2323/TS2393 errors.
- **Fix:** Removed the second duplicate block (lines ~265-298)
- **Files modified:** `ui/packages/@quent/utils/src/colors.ts`
- **Verification:** `tsc --noEmit` passes, build passes
- **Committed in:** `2ec2c643` (Task 2 commit)

**2. [Rule 1 - Bug] VariableWidthEdgeProps interface declared isDark as direct prop instead of data prop**
- **Found during:** Task 2 (running TypeScript check)
- **Issue:** `VariableWidthEdgeProps extends EdgeProps { isDark: boolean }` caused type incompatibility with XYFlow `EdgeTypes` because the component reads isDark from `data.isDark` at runtime (not from direct prop). The interface was wrong — isDark is passed via the data object, not as a prop.
- **Fix:** Changed `interface VariableWidthEdgeProps extends EdgeProps { isDark: boolean }` to `type VariableWidthEdgeProps = EdgeProps`
- **Files modified:** `ui/packages/@quent/components/src/dag/DAGChart.tsx`
- **Verification:** `tsc --noEmit` passes, build passes
- **Committed in:** `2ec2c643` (Task 2 commit)

**3. [Rule 1 - Bug] QueryResourceTree.test.tsx failing due to missing ThemeProvider**
- **Found during:** Task 2 (running test suite)
- **Issue:** `QueryResourceTree` now calls `useTheme()` from ThemeContext (added in Task 1 to get isDark). The test renders QueryResourceTree without ThemeProvider, causing "useTheme must be used within ThemeProvider" error.
- **Fix:** Added `vi.mock('@/contexts/ThemeContext', ...)` returning stub useTheme with theme='light'
- **Files modified:** `ui/src/components/QueryResourceTree.test.tsx`
- **Verification:** All 37 tests pass
- **Committed in:** `2ec2c643` (Task 2 commit)

---

**Total deviations:** 3 auto-fixed (3 Rule 1 bugs)
**Impact on plan:** All pre-existing bugs from Plan 02 extraction. No scope creep. The third bug was directly caused by our Task 1 change (adding useTheme call) and required a corresponding test update.

## Issues Encountered

None beyond the 3 auto-fixed bugs documented above.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Phase 04 complete: @quent/components, @quent/hooks, @quent/client, @quent/utils fully extracted
- App shell exclusively consumes @quent/* packages — zero stale imports
- Build and all tests green
- The Quent UI modularization milestone is structurally complete
- No blockers for milestone completion

---
*Phase: 04-extract-quent-components-and-migrate-app-shell*
*Completed: 2026-04-13*
