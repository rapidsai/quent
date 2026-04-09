---
phase: 03-extract-quent-client-and-quent-hooks
plan: 02
subsystem: ui
tags: [jotai, react-hooks, typescript, pnpm-workspace, @quent/hooks, @quent/client, @quent/utils]

# Dependency graph
requires:
  - phase: 03-extract-quent-client-and-quent-hooks
    plan: 01
    provides: "@quent/client fully populated with fetch functions and DEFAULT_STALE_TIME; ZoomRange in @quent/utils; @quent/hooks package.json with correct dependencies"

provides:
  - "@quent/hooks fully populated: 4 DAG hooks + setters, 16 timeline hooks, useBulkTimelines, useBulkTimelineFetch, useHighlightedItemIds, timelineCacheKey"
  - "Record-based timelineDataMapAtom replaces atomFamily (jotai-family dependency removed)"
  - "Named barrel index.ts — no raw atom exports (HOOKS-02)"
  - "timeline.utils.ts with getResourceTypeName, getFsmTypeName, setOperatorOnEntry (pure domain utils, no app coupling)"
  - "useBulkTimelines accepts TreeNode interface — app passes TreeTableItem via structural typing; app-layer tree utilities injected"

affects: [03-03-migrate-app-imports, 04-extract-quent-components, any consumer of @quent/hooks]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Record-based atom pattern: atom<Record<string, T>> replaces atomFamily — single atom, key-addressed reads/writes"
    - "Dependency injection for app-layer tree utilities: package hooks accept callback functions (collectVisibleEntriesFn, buildBulkParamsFn, findItemByIdFn) so package avoids importing component-layer types"
    - "Structural typing for tree nodes: local TreeNode interface in package; callers pass TreeTableItem (superset) without casting"
    - "vite-env.d.ts pattern: same fix applied to @quent/hooks as was applied to @quent/client for import.meta.env transitively via @quent/client"

key-files:
  created:
    - ui/packages/@quent/hooks/src/atoms/dag.ts
    - ui/packages/@quent/hooks/src/atoms/timeline.ts
    - ui/packages/@quent/hooks/src/dag/useSelectedNodeIds.ts
    - ui/packages/@quent/hooks/src/dag/useSelectedOperatorLabel.ts
    - ui/packages/@quent/hooks/src/dag/useSelectedPlanId.ts
    - ui/packages/@quent/hooks/src/dag/useHoveredWorkerId.ts
    - ui/packages/@quent/hooks/src/timeline/useTimelineAtoms.ts
    - ui/packages/@quent/hooks/src/timeline/timeline.utils.ts
    - ui/packages/@quent/hooks/src/timeline/useBulkTimelineFetch.ts
    - ui/packages/@quent/hooks/src/timeline/useBulkTimelines.ts
    - ui/packages/@quent/hooks/src/timeline/useHighlightedItemIds.ts
    - ui/packages/@quent/hooks/src/vite-env.d.ts
  modified:
    - ui/packages/@quent/hooks/src/index.ts

key-decisions:
  - "Dependency injection for tree utilities: useBulkTimelines accepts collectVisibleEntriesFn, buildBulkParamsFn, findItemByIdFn rather than importing from app layer — keeps package boundary clean"
  - "Only 3 timeline.utils functions copied (getResourceTypeName, getFsmTypeName, setOperatorOnEntry) — others stay in app layer per plan, passed via injection"
  - "vite-env.d.ts added to @quent/hooks/src/ because tsc includes @quent/client source transitively and import.meta.env requires vite type augmentation"
  - "useBulkTimelines uses generic TreeNode<T extends TreeNode> for structural compatibility with app's TreeTableItem without importing app-layer types"

patterns-established:
  - "Record-based atom: use atom<Record<string, T>>({}) and store.set(atom, prev => ({ ...prev, updates })) instead of atomFamily"
  - "Package boundary via injection: when a hook needs app-layer utilities, accept them as typed function parameters rather than importing from app"
  - "Structural tree node interface: define minimal interface { id: string; children?: T[] } in package; app passes richer type via structural subtyping"

requirements-completed: [HOOKS-01, HOOKS-02, HOOKS-03, HOOKS-04]

# Metrics
duration: 5min
completed: 2026-04-09
---

# Phase 03 Plan 02: Populate @quent/hooks Summary

**@quent/hooks populated with Jotai atoms (internal) and all named hook exports: 4 DAG hooks, 16 timeline hooks, useBulkTimelines (with DI), useBulkTimelineFetch, useHighlightedItemIds; atomFamily fully replaced with record-based atom**

## Performance

- **Duration:** 5 min
- **Started:** 2026-04-09T17:55:54Z
- **Completed:** 2026-04-09T18:00:42Z
- **Tasks:** 2
- **Files modified:** 13

## Accomplishments

- Created all @quent/hooks source files: dag and timeline atom modules, 4 DAG hook wrappers, useTimelineAtoms (16 hooks), timeline.utils (3 domain functions), useBulkTimelineFetch (record-based), useBulkTimelines (with DI), useHighlightedItemIds, vite-env.d.ts
- Replaced atomFamily (jotai-family) with record-based `timelineDataMapAtom = atom<Record<string, SingleTimelineResponse>>({})` — `applyBulkTimelineResponse` writes via `store.set(timelineDataMapAtom, prev => ({ ...prev, ...updates }))`
- Named barrel index.ts exports all 28+ hooks and helpers; no raw atom exports; package typechecks independently

## Task Commits

Each task was committed atomically:

1. **Task 1: Create internal atoms — dag atoms and record-based timeline atoms (HOOKS-01)** - `7a5bf240` (feat)
2. **Task 2: Create hook wrappers, move Jotai-aware hooks, build barrel (HOOKS-02, HOOKS-03)** - `e2e2b0d0` (feat)

## Files Created/Modified

- `ui/packages/@quent/hooks/src/atoms/dag.ts` - Internal dag atoms: selectedNodeIdsAtom, selectedOperatorLabelAtom, selectedPlanIdAtom, hoveredWorkerIdAtom
- `ui/packages/@quent/hooks/src/atoms/timeline.ts` - Internal timeline atoms: timelineDataMapAtom (record-based), timelineCacheKey, TimelineCacheParams, and 7 more atoms
- `ui/packages/@quent/hooks/src/dag/useSelectedNodeIds.ts` - useSelectedNodeIds, useSetSelectedNodeIds
- `ui/packages/@quent/hooks/src/dag/useSelectedOperatorLabel.ts` - useSelectedOperatorLabel, useSetSelectedOperatorLabel
- `ui/packages/@quent/hooks/src/dag/useSelectedPlanId.ts` - useSelectedPlanId, useSetSelectedPlanId
- `ui/packages/@quent/hooks/src/dag/useHoveredWorkerId.ts` - useHoveredWorkerId, useSetHoveredWorkerId
- `ui/packages/@quent/hooks/src/timeline/useTimelineAtoms.ts` - 16 timeline hooks (useTimelineData, useIsTimelineHovered, useZoomRange, etc.)
- `ui/packages/@quent/hooks/src/timeline/timeline.utils.ts` - getResourceTypeName, getFsmTypeName, setOperatorOnEntry (no app coupling)
- `ui/packages/@quent/hooks/src/timeline/useBulkTimelineFetch.ts` - useBulkTimelineFetch, applyBulkTimelineResponse, buildMergedBulkEntries
- `ui/packages/@quent/hooks/src/timeline/useBulkTimelines.ts` - useBulkTimelines with generic TreeNode<T> and dependency injection
- `ui/packages/@quent/hooks/src/timeline/useHighlightedItemIds.ts` - useHighlightedItemIds with local TreeNode interface
- `ui/packages/@quent/hooks/src/vite-env.d.ts` - Triple-slash reference for import.meta.env support
- `ui/packages/@quent/hooks/src/index.ts` - Named barrel with all exports (no raw atoms)

## Decisions Made

- Dependency injection for tree utilities: `useBulkTimelines` signature extended with `collectVisibleEntriesFn`, `buildBulkParamsFn`, `findItemByIdFn` parameters. This avoids importing `TreeTableItem`, `QueryEntities`-dependent tree traversal, and other app-layer types into the package.
- Only 3 functions copied to `timeline.utils.ts` (`getResourceTypeName`, `getFsmTypeName`, `setOperatorOnEntry`) — the functions that only depend on `TimelineRequest<TaskFilter>` types from `@quent/utils`. All other timeline.utils functions remain in the app.
- `vite-env.d.ts` added to `@quent/hooks/src/` because `@quent/client` uses `import.meta.env` in `api.ts` and the hooks package typecheck traverses client source via workspace symlink.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added vite-env.d.ts to @quent/hooks for transitively required import.meta.env**
- **Found during:** Task 2 (typecheck @quent/hooks)
- **Issue:** `../client/src/api.ts(21,34): error TS2339: Property 'env' does not exist on type 'ImportMeta'` — hooks tsconfig traverses client source, which uses `import.meta.env`, and the vite type reference wasn't in scope
- **Fix:** Created `ui/packages/@quent/hooks/src/vite-env.d.ts` with `/// <reference types="vite/client" />`
- **Files modified:** ui/packages/@quent/hooks/src/vite-env.d.ts (new file)
- **Verification:** `pnpm --filter @quent/hooks exec tsc --noEmit` exits 0
- **Committed in:** e2e2b0d0 (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** Same pattern as Plan 01 fix. Necessary for independent typecheck. No scope creep.

## Issues Encountered

- Merge was required before executing: worktree branch was on main, while modularize-timeline had the packages from Plan 01. Resolved with `git merge modularize-timeline --no-ff` and resolved 6 content conflicts by taking modularize-timeline versions.
- pnpm install needed for tsc availability in the worktree (ran offline from cached store).

## Known Stubs

None - all hooks wire to real atoms and data sources. No placeholder data.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- @quent/hooks fully populated and typechecks independently
- Plan 03 Plan 03 (migrate app imports to @quent/hooks) can now proceed
- App consumers of useBulkTimelines will need to add the 3 DI parameters (collectVisibleEntriesFn, buildBulkParamsFn, findItemByIdFn) when switching to @quent/hooks

---
*Phase: 03-extract-quent-client-and-quent-hooks*
*Completed: 2026-04-09*
