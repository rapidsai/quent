---
phase: "04"
plan: "02"
subsystem: "@quent/components"
tags: [components, extraction, timeline, dag, resource-tree, operator-timeline]
dependency_graph:
  requires: ["04-01"]
  provides: ["@quent/components barrel export with all UI primitives and complex components"]
  affects: ["04-03"]
tech_stack:
  added: []
  patterns:
    - "isDark: boolean prop pattern (ThemeContext decoupling)"
    - "Controlled DAGChart API (selectedNodeIds/onSelectionChange)"
    - "useTimelineChartColors(isDark) function signature"
key_files:
  created:
    - ui/packages/@quent/components/src/index.ts
    - ui/packages/@quent/components/src/ui/{button,card,collapsible,data-text,dropdown-menu,hover-card,input,navigation-menu,popover,resizable,scroll-area,select-field,select,skeleton,tree-table,tree-view}.tsx
    - ui/packages/@quent/components/src/lib/{echarts,queryBundle.utils,resource.utils,timeline.utils}.ts
    - ui/packages/@quent/components/src/services/query-plan/{types,dagFieldProcessing,operationTypes,query-bundle-transformer}.ts
    - ui/packages/@quent/components/src/timeline/{Timeline,TimelineController,TimelineSkeleton,TimelineToolbar,TimelineTooltip,ResourceTimeline,useTimelineChartColors,types}.ts
    - ui/packages/@quent/components/src/dag/{DAGChart,DAGControls,DAGLegend,DAGSettingsPopover}.tsx
    - ui/packages/@quent/components/src/query-plan/{QueryPlanNode,OperatorStatisticsPopup}.tsx
    - ui/packages/@quent/components/src/resource-tree/{types,ResourceColumn,ResourceGroupRow,ResourceRow,InlineSelector,UsageColumn}.ts
    - ui/packages/@quent/components/src/operator-timeline/{OperatorGanttChart,types,utils}.ts
  modified: []
decisions:
  - "isDark boolean prop pattern chosen over ThemeContext re-export to maintain zero coupling to app shell"
  - "DAGChart gains controlled selectedNodeIds/onSelectionChange API for external consumers"
  - "Standalone tsc errors are all cascade effects of unresolved peer deps; main app tsconfig remains clean"
metrics:
  duration_minutes: 120
  completed_date: "2026-04-13"
  tasks_completed: 3
  files_created: 48
---

# Phase 04 Plan 02: Copy All Components into @quent/components Summary

Extracted all 16 UI primitives, internal lib utilities, and 5 complex component families (DAG, timeline, resource-tree, query-plan, operator-timeline) into `@quent/components`, decoupling from ThemeContext via `isDark: boolean` props and replacing Jotai atom access with `@quent/hooks` selectors. Barrel `index.ts` provides complete named exports.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Copy UI primitives and lib files | d9e7c7b8, 9b7e04d9 | 25 files (ui/, lib/, services/, resource-tree/types) |
| 2 | Copy complex components with ThemeContext decoupling | 3c64dbd3 | 22 files (timeline, dag, query-plan, resource-tree, operator-timeline) |
| 3 | Create barrel export index.ts | 89f9633f | index.ts |

## What Was Built

**Task 1 — UI primitives and lib files:**
- 16 UI primitives copied from `ui/src/components/ui/` with `@/lib/utils` → `@quent/utils` (cn), all Radix imports preserved as-is
- `lib/echarts.ts` — custom ECharts build with tree-shaking (no import changes needed)
- `lib/queryBundle.utils.ts` — `EntityRefKey`, `QueryEntities`, `Operator` from `@quent/utils`; `StatValue` from `../services/query-plan/types`
- `lib/resource.utils.ts` — `EntityTypeKey` from `@quent/utils`; `TreeTableItem` from `../resource-tree/types`
- `lib/timeline.utils.ts` — all `@/` and `~quent/types/*` imports rewritten to relative or `@quent/*`; imports `CHART_GROUP` from `../timeline/Timeline`
- `services/query-plan/*` — types, dagFieldProcessing, operationTypes, query-bundle-transformer all migrated
- `resource-tree/types.ts` — `EntityTypeValue` from `@quent/utils`

**Task 2 — Complex components:**
- `timeline/useTimelineChartColors.ts` — signature changed from `useTimelineChartColors()` (reading ThemeContext internally) to `useTimelineChartColors(isDark: boolean)`
- `timeline/Timeline.tsx`, `TimelineController.tsx`, `ResourceTimeline.tsx` — `isDark: boolean` prop added, passed through to `useTimelineChartColors`
- `dag/DAGChart.tsx` — `isDark: boolean` prop added; controlled API `selectedNodeIds?: string[]` + `onSelectionChange?: (nodeIds: string[]) => void` added; `useEdgeWidthConfig`, `useEdgeColoring`, `useEdgeColorPalette`, `useSelectedEdgeWidthField`, `useSelectedEdgeColorField` from `@quent/hooks`; `isDark` passed via edge `data` field
- `dag/DAGControls.tsx` — atom imports replaced with `useSelectedColorField`, `useSelectedEdgeWidthField`, `useSelectedEdgeColorField`, `useSelectedNodeLabelField` from `@quent/hooks`
- `dag/DAGLegend.tsx` — all atom imports replaced with hooks; `isDark` prop added
- `dag/DAGSettingsPopover.tsx` — atom imports replaced with `useNodeColorPalette`, `useEdgeColorPalette`; `isDark` prop added
- `resource-tree/UsageColumn.tsx` — `isDark` prop added, forwarded to `ResourceTimeline`
- `operator-timeline/OperatorGanttChart.tsx` — `useSetAtom(selectedNodeIdsAtom)` → `useSetSelectedNodeIds()`; `useAtomValue(selectedNodeIdsAtom)` → `useSelectedNodeIds()`; `useSetSelectedOperatorLabel`, `useSetSelectedPlanId`; `withOpacity` from `@quent/utils`; `isDark` prop added

**Task 3 — Barrel export:**
- `index.ts` exports all 48 modules with named re-exports, organized by subsystem

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Import chain: timeline.utils imports CHART_GROUP from Timeline**
- **Found during:** Task 1
- **Issue:** `lib/timeline.utils.ts` imports `CHART_GROUP` from `../timeline/Timeline`, which needed to exist before Task 1's lib files could compile
- **Fix:** Wrote `Timeline.tsx` (and its dependencies) as part of Task 1 to satisfy the import
- **Files modified:** `timeline/Timeline.tsx`, `timeline/useTimelineChartColors.ts`, `timeline/TimelineTooltip.tsx`
- **Commit:** d9e7c7b8 (included in Task 1 commit)

**2. [Rule 1 - Bug] VariableWidthEdgeProps extends EdgeProps — property access in destructure**
- **Found during:** Task 2 tsc verification
- **Issue:** TypeScript 2339 errors on `id`, `source`, `target`, etc. properties in `VariableWidthEdge` component. These are cascade failures from `@xyflow/react` not being resolvable in the standalone package tsconfig.
- **Fix:** Confirmed the same pattern works in the main app (which has all deps resolved). No code change needed — this is an expected isolation artifact.
- **Impact:** None — main app `tsc --noEmit` remains clean

**3. [Rule 1 - Fix] index.ts export locations corrected**
- **Found during:** Task 3 tsc verification
- **Issue:** `TIMELINE_MONO_FONT` was in `useTimelineChartColors.ts` not `timeline/types.ts`; `DEFAULT_TIMELINE_HEIGHT` was in `timeline/types.ts` not `TimelineSkeleton.tsx`; `TimelineMark`, `TimelineSeries`, `TimelineSeriesEntry` were in `timeline/types.ts` not `lib/timeline.utils.ts`; `transformResourceTree` is in `lib/timeline.utils.ts` not `query-bundle-transformer.ts`
- **Fix:** Corrected all export source paths in index.ts
- **Commit:** 89f9633f

## Known Stubs

None — all components are full implementations with real data flows, not stubs.

## Self-Check: PASSED

- All 48 source files exist under `ui/packages/@quent/components/src/`
- All 4 task commits exist in git log: d9e7c7b8, 9b7e04d9, 3c64dbd3, 89f9633f
- Main app `tsc --noEmit` exits clean (pre-existing vitest/globals error only)
