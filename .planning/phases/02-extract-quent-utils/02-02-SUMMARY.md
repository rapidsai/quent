---
phase: 02-extract-quent-utils
plan: 02
subsystem: ui
tags: [imports, migration, cleanup, build]
dependency_graph:
  requires: [02-01]
  provides: [UTILS-01, UTILS-02, UTILS-03, UTILS-04, UTILS-05]
  affects: [ui/src/**]
tech_stack:
  added: []
  patterns:
    - All app imports consolidated to @quent/utils barrel export
    - No legacy path aliases (~quent/types) remain
key_files:
  created: []
  modified:
    - ui/vite.config.ts
    - ui/tsconfig.json
    - ui/src/services/api.ts
    - ui/src/services/api.test.ts
    - ui/src/types.ts
    - ui/src/lib/timeline.utils.ts
    - ui/src/lib/queryBundle.utils.ts
    - ui/src/services/query-plan/query-bundle-transformer.ts
    - ui/src/atoms/timeline.ts
    - ui/src/hooks/useQueryBundle.ts
    - ui/src/hooks/useBulkTimelines.ts
    - ui/src/hooks/useBulkTimelineFetch.ts
    - ui/src/hooks/useQueryPlanVisualization.ts
    - ui/src/routes/__root.tsx
    - ui/src/routes/profile.engine.$engineId.query.$queryId.index.tsx
    - ui/src/routes/profile.engine.$engineId.query.$queryId.node.$nodeId.tsx
    - ui/src/pages/EngineSelectionPage.tsx
    - ui/src/components/NavBarNavigator.tsx
    - ui/src/components/QueryResourceTree.tsx
    - ui/src/components/QueryResourceTree.test.tsx
    - ui/src/components/query-plan/QueryPlanNode.tsx
    - ui/src/components/timeline/Timeline.tsx
    - ui/src/components/timeline/TimelineController.tsx
    - ui/src/components/timeline/TimelineTooltip.tsx
    - ui/src/components/timeline/ResourceTimeline.tsx
    - ui/src/components/timeline/useTimelineChartColors.ts
    - ui/src/components/resource-tree/ResourceColumn.tsx
    - ui/src/components/resource-tree/ResourceRow.tsx
    - ui/src/components/resource-tree/ResourceGroupRow.tsx
    - ui/src/components/resource-tree/UsageColumn.tsx
    - ui/src/components/resource-tree/InlineSelector.tsx
    - ui/src/components/ui/button.tsx
    - ui/src/components/ui/card.tsx
    - ui/src/components/ui/dropdown-menu.tsx
    - ui/src/components/ui/hover-card.tsx
    - ui/src/components/ui/input.tsx
    - ui/src/components/ui/navigation-menu.tsx
    - ui/src/components/ui/popover.tsx
    - ui/src/components/ui/resizable.tsx
    - ui/src/components/ui/select.tsx
    - ui/src/components/ui/skeleton.tsx
    - ui/src/components/ui/tree-table.tsx
    - ui/src/components/ui/tree-view.tsx
  deleted:
    - ui/src/lib/utils.ts
    - ui/src/services/colors.ts
    - ui/src/services/formatters.ts
decisions:
  - "Consolidated mixed type/value imports from ~quent/types/* into single @quent/utils import lines per file"
  - "Used import type for pure type imports and value imports for runtime-used values (Operator, Plan, etc.)"
metrics:
  duration_minutes: 11
  tasks_completed: 2
  files_modified: 41
  files_deleted: 3
  completed_date: "2026-04-01"
---

# Phase 02 Plan 02: Migrate App Imports to @quent/utils Summary

**One-liner:** Migrated all 41 app files from ~quent/types/*, @/lib/utils, @/services/colors, and @/services/formatters to @quent/utils; removed the ~quent/types alias and deleted the 3 legacy source files; build and all 37 tests pass.

## What Was Done

### Task 1: Migrate all app imports to @quent/utils (commit f37e3fe3)

Executed 5 sweeps across the app source tree:

1. **Sweep 1 (~quent/types/* → @quent/utils):** 22 files with Rust-generated type imports consolidated into single `import type { ... }` or `import { ... }` statements. Files with both type and value imports received two separate lines. `parseJsonWithBigInt` removed from `api.ts` and imported from `@quent/utils` instead.

2. **Sweep 2 (@/lib/utils → @quent/utils):** 18 files using `cn()` updated. Where a file already had a `@quent/utils` import from Sweep 1, `cn` was merged into the existing import.

3. **Sweep 3 (@/services/colors → @quent/utils):** 5 timeline-related files updated. Symbols merged into existing `@quent/utils` imports where applicable.

4. **Sweep 4 (@/services/formatters → @quent/utils):** 3 files updated. Symbols merged.

5. **Sweep 5 (api.test.ts):** Updated to import `parseJsonWithBigInt` from `@quent/utils` instead of `./api`.

Result: Zero legacy import paths remain in `ui/src/` (excluding the 3 source files that were deleted in Task 2).

### Task 2: Remove aliases, delete source files, verify build (commit 30549a30)

1. **vite.config.ts:** Removed the `~quent/types` alias line and the TODO comment. The `@` alias and `elkjs` bundled alias remain.

2. **tsconfig.json:** Removed `~quent/types/*` from `compilerOptions.paths` and `../examples/simulator/server/ts-bindings` from the `include` array. Include now contains only `["src"]`.

3. **Deleted 3 legacy source files:**
   - `ui/src/lib/utils.ts` (9-line cn wrapper — moved to @quent/utils)
   - `ui/src/services/colors.ts` (291 lines — moved to @quent/utils)
   - `ui/src/services/formatters.ts` (143 lines — moved to @quent/utils)

4. **Build verification:** `pnpm test:run` — 37 tests pass across 4 test files. `pnpm build` — production bundle builds successfully with zero errors.

## Deviations from Plan

None — plan executed exactly as written.

## Known Stubs

None. All imports are live, wired to the @quent/utils package created in Plan 01.

## Self-Check: PASSED

- SUMMARY.md: found at .planning/phases/02-extract-quent-utils/02-02-SUMMARY.md
- Commit f37e3fe3: found (feat(02-02): migrate all app imports to @quent/utils)
- Commit 30549a30: found (feat(02-02): remove aliases, delete legacy source files, verify build)
- ui/src/lib/utils.ts: deleted (confirmed)
- ui/src/services/colors.ts: deleted (confirmed)
- ui/src/services/formatters.ts: deleted (confirmed)
- pnpm test:run: 37 tests pass
- pnpm build: zero errors
