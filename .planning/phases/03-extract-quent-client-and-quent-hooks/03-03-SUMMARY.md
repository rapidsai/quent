---
phase: 03-extract-quent-client-and-quent-hooks
plan: "03"
subsystem: ui-import-migration
tags: [migration, import-cleanup, @quent/client, @quent/hooks, jotai, tanstack-query]
dependency_graph:
  requires: ["03-01", "03-02"]
  provides: ["clean-import-surface", "no-direct-atom-imports"]
  affects: ["all-ui-components", "all-ui-routes"]
tech_stack:
  added: []
  patterns:
    - "All app code imports atoms via hook wrappers from @quent/hooks (no direct atom access)"
    - "All API calls and query options via @quent/client"
    - "DI pattern for useBulkTimelines: collectVisibleEntriesFn, buildBulkParamsFn, findItemByIdFn"
    - "useHydrateTimelineAtoms encapsulates atom initialization (replaces useHydrateAtoms)"
key_files:
  created:
    - ui/src/atoms/dagControls.ts
    - ui/packages/@quent/hooks/src/testing.ts
  modified:
    - ui/src/lib/queryClient.ts
    - ui/src/pages/EngineSelectionPage.tsx
    - ui/src/components/NavBarNavigator.tsx
    - ui/src/components/QueryResourceTree.tsx
    - ui/src/components/QueryResourceTree.test.tsx
    - ui/src/components/QueryPlan.tsx
    - ui/src/components/dag/DAGChart.tsx
    - ui/src/components/dag/DAGControls.tsx
    - ui/src/components/dag/DAGLegend.tsx
    - ui/src/components/dag/DAGSettingsPopover.tsx
    - ui/src/components/query-plan/QueryPlanNode.tsx
    - ui/src/components/timeline/TimelineController.tsx
    - ui/src/components/timeline/TimelineToolbar.tsx
    - ui/src/components/timeline/Timeline.tsx
    - ui/src/components/timeline/ResourceTimeline.tsx
    - ui/src/components/resource-tree/UsageColumn.tsx
    - ui/src/hooks/useNodeColoring.ts
    - ui/src/hooks/useDagControls.ts
    - ui/src/hooks/useHighlightedItemIds.ts
    - ui/src/routes/profile.engine.$engineId.query.$queryId.index.tsx
    - ui/src/routes/profile.engine.$engineId.query.$queryId.node.$nodeId.tsx
    - ui/packages/@quent/hooks/src/timeline/useTimelineAtoms.ts
    - ui/packages/@quent/hooks/src/index.ts
    - ui/packages/@quent/hooks/package.json
    - ui/package.json
  deleted:
    - ui/src/services/api.ts
    - ui/src/atoms/dag.ts
    - ui/src/atoms/timeline.ts
    - ui/src/hooks/useQueryBundle.ts
    - ui/src/hooks/useBulkTimelines.ts
    - ui/src/hooks/useBulkTimelineFetch.ts
    - ui/src/hooks/useHighlightedItemIds.ts
decisions:
  - "Created ui/src/atoms/dagControls.ts to house visual-only control atoms (edgeWidthConfigAtom, edgeColoringAtom, etc.) that were in atoms/dag.ts but not migrated to @quent/hooks"
  - "Added useHydrateTimelineAtoms to @quent/hooks to encapsulate jotai useHydrateAtoms usage, keeping atoms private to the package"
  - "Added @quent/hooks/testing subpath export to expose timelineDataMapAtom for test assertions only"
  - "Shim files for useBulkTimelines.ts and useBulkTimelineFetch.ts re-export from @quent/hooks to ease Task 1 migration"
metrics:
  duration: "~3 hours"
  completed: "2026-04-09T19:51:39Z"
  tasks_completed: 2
  files_changed: 26
---

# Phase 3 Plan 3: Migrate App Imports to @quent/client and @quent/hooks Summary

All app code in ui/src/ now imports from @quent/client and @quent/hooks instead of local service/atom files; seven old source files deleted and jotai-family dependency removed.

## Objective

Migrate every import site in ui/src/ from @/services/api, @/atoms/dag, @/atoms/timeline, and the four moved hooks to @quent/client and @quent/hooks. Delete the now-redundant source files. Complete the Phase 3 extraction.

## Tasks Completed

### Task 1: Migrate all app import sites to @quent/client and @quent/hooks

**Commit:** `921851f4`

Updated 21 files across components, routes, hooks, and tests:

- `ui/src/lib/queryClient.ts` — DEFAULT_STALE_TIME from @quent/client
- `ui/src/pages/EngineSelectionPage.tsx` — fetch functions from @quent/client
- `ui/src/components/NavBarNavigator.tsx` — queryBundleQueryOptions + fetch functions from @quent/client
- `ui/src/components/QueryResourceTree.tsx` — useBulkTimelines, useHighlightedItemIds from @quent/hooks; fetchSingleTimeline from @quent/client; useHydrateTimelineAtoms replaces useHydrateAtoms
- `ui/src/components/QueryResourceTree.test.tsx` — mocks updated to @quent/client and @quent/hooks
- `ui/src/components/QueryPlan.tsx` — useSelectedPlanId, useSetSelectedPlanId, useSetHoveredWorkerId from @quent/hooks
- `ui/src/components/dag/DAGChart.tsx` — useSelectedNodeIds, useSetSelectedNodeIds, useSetSelectedOperatorLabel from @quent/hooks; visual atoms from @/atoms/dagControls
- `ui/src/components/dag/DAGControls.tsx`, `DAGLegend.tsx`, `DAGSettingsPopover.tsx` — visual atoms from @/atoms/dagControls
- `ui/src/components/query-plan/QueryPlanNode.tsx` — useSelectedNodeIds from @quent/hooks
- `ui/src/components/timeline/TimelineController.tsx` — useZoomRange from @quent/hooks
- `ui/src/components/timeline/TimelineToolbar.tsx` — 7 hooks from @quent/hooks replacing all raw atom usage
- `ui/src/components/timeline/Timeline.tsx` — useZoomRange from @quent/hooks
- `ui/src/components/timeline/ResourceTimeline.tsx` — all timeline hooks from @quent/hooks; fetchSingleTimeline from @quent/client
- `ui/src/components/resource-tree/UsageColumn.tsx` — useIsTimelineHovered, useSetHoveredTimelineId from @quent/hooks
- `ui/src/hooks/useNodeColoring.ts` — useSelectedNodeIds from @quent/hooks; visual atoms from @/atoms/dagControls
- `ui/src/hooks/useDagControls.ts` — visual atoms from @/atoms/dagControls
- `ui/src/hooks/useHighlightedItemIds.ts` — useHoveredWorkerId from @quent/hooks
- Two route files — queryBundleQueryOptions from @quent/client

**Also added to @quent/hooks package:**
- `useHydrateTimelineAtoms` function in useTimelineAtoms.ts
- `@quent/hooks/testing` subpath export for test-only atom access

### Task 2: Delete old source files and verify full build + tests

**Commit:** `df30ae25`

Deleted 7 files whose content now lives in @quent/client or @quent/hooks:
- `ui/src/services/api.ts`
- `ui/src/atoms/dag.ts`
- `ui/src/atoms/timeline.ts`
- `ui/src/hooks/useQueryBundle.ts`
- `ui/src/hooks/useBulkTimelines.ts`
- `ui/src/hooks/useBulkTimelineFetch.ts`
- `ui/src/hooks/useHighlightedItemIds.ts`

Removed `jotai-family: ^1.0.1` from `ui/package.json` (was only used by atoms/timeline.ts).

**Verification:**
- No remaining imports from any of the deleted paths
- TypeScript errors unchanged from baseline (pre-existing @/services/colors and @/lib/utils errors unrelated to this plan)
- Test results unchanged from baseline: 1 suite (19 tests) passes; 3 suites fail due to pre-existing `@/lib/utils` missing in data-text.tsx

## Deviations from Plan

### Auto-added Issues

**1. [Rule 2 - Missing Critical Functionality] Created ui/src/atoms/dagControls.ts**
- **Found during:** Task 1 (DAGChart.tsx, DAGControls.tsx, DAGLegend.tsx, DAGSettingsPopover.tsx, useNodeColoring.ts, useDagControls.ts)
- **Issue:** `atoms/dag.ts` contained both state atoms migrated to @quent/hooks AND visual control atoms (edgeWidthConfigAtom, edgeColoringAtom, nodeColoringAtom, etc.) that were not migrated. Deleting the file without providing a new home for visual atoms would have broken 6 files.
- **Fix:** Created `ui/src/atoms/dagControls.ts` containing all visual-only atoms. Updated the 6 consumer files to import from `@/atoms/dagControls` instead of `@/atoms/dag`.
- **Files created:** `ui/src/atoms/dagControls.ts`
- **Files modified:** `DAGChart.tsx`, `DAGControls.tsx`, `DAGLegend.tsx`, `DAGSettingsPopover.tsx`, `useNodeColoring.ts`, `useDagControls.ts`

**2. [Rule 2 - Missing Critical Functionality] Added useHydrateTimelineAtoms to @quent/hooks**
- **Found during:** Task 1 (QueryResourceTree.tsx)
- **Issue:** `QueryResourceTree.tsx` used `useHydrateAtoms` from jotai/utils directly with raw atoms from @/atoms/timeline. After migration, the atoms are private to @quent/hooks, making direct `useHydrateAtoms` calls impossible from app code.
- **Fix:** Added `useHydrateTimelineAtoms({ zoomRange, debouncedZoomRange, startTimeMs })` to @quent/hooks, exported from the main barrel. Updated QueryResourceTree.tsx to use it.
- **Files modified:** `ui/packages/@quent/hooks/src/timeline/useTimelineAtoms.ts`, `ui/packages/@quent/hooks/src/index.ts`

**3. [Rule 2 - Missing Critical Functionality] Added @quent/hooks/testing subpath export**
- **Found during:** Task 1 (QueryResourceTree.test.tsx)
- **Issue:** The test needed to inspect `timelineDataMapAtom` directly to verify atom state. The atom is internal to @quent/hooks — no export path existed.
- **Fix:** Created `ui/packages/@quent/hooks/src/testing.ts` with `export { timelineDataMapAtom }` and added `"./testing": "./src/testing.ts"` to package.json exports. Test imports changed to `@quent/hooks/testing`.
- **Files created:** `ui/packages/@quent/hooks/src/testing.ts`
- **Files modified:** `ui/packages/@quent/hooks/package.json`

## Known Stubs

None.

## Pre-existing Issues (Out of Scope)

The following TypeScript errors and test failures existed before this plan and are unchanged:
- `Cannot find module '@/services/colors'` — referenced by dagFieldProcessing.ts, dagControls.ts, DAGChart.tsx, DAGLegend.tsx, DAGSettingsPopover.tsx, useNodeColoring.ts
- `Cannot find module '@/services/formatters'` — referenced by dagFieldProcessing.ts
- `Cannot find module '@/lib/utils'` — referenced by data-text.tsx, scroll-area.tsx, select-field.tsx
- 3 test suites fail due to `@/lib/utils` missing (QueryResourceTree.test.tsx, profile.index.test.tsx, example.test.tsx)

These are documented in `.planning/deferred-items.md` as pre-existing issues to be addressed in a future phase.

## Self-Check: PASSED

- `ui/src/atoms/dagControls.ts` — FOUND
- `ui/packages/@quent/hooks/src/testing.ts` — FOUND
- Commit `921851f4` — FOUND
- Commit `df30ae25` — FOUND
- `ui/src/services/api.ts` — correctly absent
- `ui/src/atoms/dag.ts` — correctly absent
- `ui/src/atoms/timeline.ts` — correctly absent
