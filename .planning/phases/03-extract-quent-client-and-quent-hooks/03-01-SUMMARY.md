---
phase: 03-extract-quent-client-and-quent-hooks
plan: 01
subsystem: ui
tags: [tanstack-query, react-query, typescript, pnpm-workspace, @quent/client, @quent/utils, @quent/hooks]

# Dependency graph
requires:
  - phase: 01-workspace-scaffold
    provides: "@quent/client and @quent/hooks package scaffolds with tsconfig and package.json"
  - phase: 02-extract-quent-utils
    provides: "@quent/utils with all Rust-generated types and utility functions including parseJsonWithBigInt"

provides:
  - "@quent/client fully populated: 6 fetch functions, 6 queryOptions factories, 5 pure TanStack Query hooks, DEFAULT_STALE_TIME constant"
  - "ZoomRange interface exported from @quent/utils (relocated from TimelineController.tsx)"
  - "Both @quent/client and @quent/hooks package.json files have correct dependency declarations"

affects: [03-extract-quent-hooks, 04-extract-quent-components, any consumer of @quent/client]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "queryOptions factory + useQuery hook per resource type (e.g. queryBundleQueryOptions + useQueryBundle)"
    - "apiFetch is internal to @quent/client — not exported from package barrel per D-02"
    - "vite-env.d.ts in package src/ enables import.meta.env to typecheck independently"
    - "staleTime override option on all hooks: options?: { staleTime?: number }"

key-files:
  created:
    - ui/packages/@quent/utils/src/types/ZoomRange.ts
    - ui/packages/@quent/client/src/constants.ts
    - ui/packages/@quent/client/src/api.ts
    - ui/packages/@quent/client/src/queryBundle.ts
    - ui/packages/@quent/client/src/engines.ts
    - ui/packages/@quent/client/src/queryGroups.ts
    - ui/packages/@quent/client/src/queries.ts
    - ui/packages/@quent/client/src/timeline.ts
    - ui/packages/@quent/client/src/bulkTimelines.ts
    - ui/packages/@quent/client/src/vite-env.d.ts
  modified:
    - ui/packages/@quent/utils/src/index.ts
    - ui/packages/@quent/client/src/index.ts
    - ui/packages/@quent/client/package.json
    - ui/packages/@quent/hooks/package.json
    - ui/src/components/timeline/TimelineController.tsx
    - ui/src/atoms/timeline.ts
    - ui/src/hooks/useBulkTimelines.ts
    - ui/src/hooks/useBulkTimelineFetch.ts
    - ui/pnpm-lock.yaml

key-decisions:
  - "vite-env.d.ts added to @quent/client/src/ so import.meta.env typechecks independently without requiring vite in devDependencies"
  - "ZoomRange relocated from TimelineController.tsx to @quent/utils to break circular dep risk and make it importable without React"
  - "apiFetch kept internal (non-exported) per D-02 — consumers use named fetch functions"
  - "Stub types (ChartDataPoint, BarChartData, DashboardMetrics, DAGResponse etc.) excluded from @quent/client per D-03"
  - "useBulkTimelines hook omitted from @quent/client — Jotai-aware version will live in @quent/hooks per D-01"

patterns-established:
  - "Package-level vite-env.d.ts: Add triple-slash reference to vite/client for packages that use import.meta.env"
  - "Named barrel exports only (no export *): Each export line in index.ts is explicit and intentional"
  - "queryOptions factory + useQuery hook pattern: Factory enables route loaders; hook wraps factory for component use"

requirements-completed: [CLIENT-01, CLIENT-02, CLIENT-03, CLIENT-04, CLIENT-05]

# Metrics
duration: 7min
completed: 2026-04-09
---

# Phase 03 Plan 01: Extract @quent/client and Fix Pre-conditions Summary

**@quent/client populated with 6 fetch functions, 6 queryOptions factories, 5 pure TanStack Query hooks and DEFAULT_STALE_TIME; ZoomRange relocated to @quent/utils; both package.json files declare correct workspace dependencies**

## Performance

- **Duration:** 7 min
- **Started:** 2026-04-09T17:45:46Z
- **Completed:** 2026-04-09T17:52:00Z
- **Tasks:** 2
- **Files modified:** 19

## Accomplishments

- Created all @quent/client source files: constants, api, queryBundle, engines, queryGroups, queries, timeline, bulkTimelines, and named barrel index
- Relocated ZoomRange interface from TimelineController.tsx to @quent/utils, updated 3 consumer files to import from @quent/utils
- Updated @quent/client and @quent/hooks package.json with correct workspace dependencies; updated pnpm lockfile

## Task Commits

Each task was committed atomically:

1. **Task 1: Fix pre-conditions — ZoomRange relocation and package.json dependencies** - `9572ac84` (feat)
2. **Task 2: Populate @quent/client — fetch functions, queryOptions factories, hooks, barrel** - `13ae9c83` (feat)

## Files Created/Modified

- `ui/packages/@quent/utils/src/types/ZoomRange.ts` - New ZoomRange interface definition
- `ui/packages/@quent/utils/src/index.ts` - Added `export type { ZoomRange }` export
- `ui/packages/@quent/client/src/constants.ts` - DEFAULT_STALE_TIME = 5 * 60 * 1000
- `ui/packages/@quent/client/src/api.ts` - Internal apiFetch + 6 named fetch functions
- `ui/packages/@quent/client/src/queryBundle.ts` - queryBundleQueryOptions + useQueryBundle
- `ui/packages/@quent/client/src/engines.ts` - enginesQueryOptions + useEngines
- `ui/packages/@quent/client/src/queryGroups.ts` - queryGroupsQueryOptions + useQueryGroups
- `ui/packages/@quent/client/src/queries.ts` - queriesQueryOptions + useQueries
- `ui/packages/@quent/client/src/timeline.ts` - singleTimelineQueryOptions + useTimeline
- `ui/packages/@quent/client/src/bulkTimelines.ts` - bulkTimelineQueryOptions (no hook, per D-01)
- `ui/packages/@quent/client/src/vite-env.d.ts` - Triple-slash reference for import.meta.env support
- `ui/packages/@quent/client/src/index.ts` - Named barrel with 17 exports (apiFetch excluded)
- `ui/packages/@quent/client/package.json` - Added `"@quent/utils": "workspace:*"` dependency
- `ui/packages/@quent/hooks/package.json` - Added @quent/client, @quent/utils deps + @tanstack/react-query peer
- `ui/src/components/timeline/TimelineController.tsx` - Removed ZoomRange definition, added import from @quent/utils
- `ui/src/atoms/timeline.ts` - Updated ZoomRange import to @quent/utils
- `ui/src/hooks/useBulkTimelines.ts` - Updated ZoomRange import to @quent/utils
- `ui/src/hooks/useBulkTimelineFetch.ts` - Updated ZoomRange import to @quent/utils
- `ui/pnpm-lock.yaml` - Updated with new workspace dependency links

## Decisions Made

- Added `vite-env.d.ts` with `/// <reference types="vite/client" />` to @quent/client/src/ because `import.meta.env` is used in api.ts for `VITE_API_BASE_URL`. This lets the package typecheck independently without requiring vite as a devDependency.
- Excluded stub types (ChartDataPoint, BarChartData, DashboardMetrics, DAGResponse etc.) from api.ts — these were legacy scaffolding in the app's api.ts with no real usages per D-03.
- Omitted `useBulkTimelines` hook from @quent/client — the app's useBulkTimelines is deeply Jotai-coupled and belongs in @quent/hooks per D-01.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added vite-env.d.ts to support import.meta.env in package typecheck**
- **Found during:** Task 2 (typecheck @quent/client)
- **Issue:** `src/api.ts(21,34): error TS2339: Property 'env' does not exist on type 'ImportMeta'` — the package tsconfig doesn't include vite/client types
- **Fix:** Created `ui/packages/@quent/client/src/vite-env.d.ts` with `/// <reference types="vite/client" />` which picks up vite from the workspace node_modules
- **Files modified:** ui/packages/@quent/client/src/vite-env.d.ts (new file)
- **Verification:** `pnpm --filter @quent/client exec tsc --noEmit` exits 0
- **Committed in:** 13ae9c83 (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** Necessary for the package to typecheck independently. No scope creep.

## Issues Encountered

- pnpm install in the worktree failed with 403 network errors (sandbox restricts npm registry access). Resolved by using `--store-dir /Users/johallaron/Library/pnpm/store --offline` to use packages already cached from the main project's installation.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- @quent/client is fully populated and typechecks independently
- @quent/hooks package.json has correct dependency declarations — ready for Phase 03 plan 02 (hooks extraction)
- ZoomRange is in @quent/utils and all app consumers updated

---
*Phase: 03-extract-quent-client-and-quent-hooks*
*Completed: 2026-04-09*
