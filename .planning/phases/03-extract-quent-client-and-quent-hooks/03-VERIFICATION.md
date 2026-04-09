---
phase: 03-extract-quent-client-and-quent-hooks
verified: 2026-04-09T20:30:00Z
status: passed
score: 9/9 must-haves verified
re_verification: false
gaps: []
human_verification:
  - test: "Run the app and select a DAG node — verify state resets when switching to a different query"
    expected: "Node selection clears on query switch, confirming Jotai Provider scoping works correctly"
    why_human: "Provider key={queryId} reset behavior requires rendering the app with live route transitions; cannot verify programmatically"
  - test: "Run the app, open a timeline — verify timeline rows render data and useBulkTimelines fires"
    expected: "Timeline rows populate with live data; no console errors about missing atom imports"
    why_human: "End-to-end data rendering through @quent/hooks atoms requires a running app with a live API"
---

# Phase 3: Extract @quent/client and @quent/hooks Verification Report

**Phase Goal:** All API fetch functions and queryOptions factories live in @quent/client; all Jotai atoms are hidden inside @quent/hooks with only named hooks exported; no raw atom access exists outside @quent/hooks
**Verified:** 2026-04-09T20:30:00Z
**Status:** PASSED
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| #  | Truth | Status | Evidence |
|----|-------|--------|----------|
| 1  | All fetch functions from api.ts are importable from @quent/client | VERIFIED | `@quent/client/src/index.ts` exports all 6: fetchQueryBundle, fetchListEngines, fetchListCoordinators, fetchListQueries, fetchSingleTimeline, fetchBulkTimelines |
| 2  | queryOptions factories exist for every fetch function | VERIFIED | 6 factories exported: queryBundleQueryOptions, enginesQueryOptions, queryGroupsQueryOptions, queriesQueryOptions, singleTimelineQueryOptions, bulkTimelineQueryOptions |
| 3  | Pure TanStack Query hooks (useQueryBundle, useEngines, useQueryGroups, useQueries, useTimeline) are importable from @quent/client | VERIFIED | All 5 exported from `@quent/client/src/index.ts` lines 26-30 |
| 4  | DEFAULT_STALE_TIME is importable from @quent/client | VERIFIED | `export { DEFAULT_STALE_TIME } from './constants'` in index.ts line 5 |
| 5  | ZoomRange is importable from @quent/utils | VERIFIED | `ui/packages/@quent/utils/src/types/ZoomRange.ts` exists; exported at `@quent/utils/src/index.ts` line 34 |
| 6  | apiFetch is NOT in the public API | VERIFIED | `apiFetch` is a non-exported `async function` in api.ts (no `export` keyword); absent from index.ts |
| 7  | All Jotai atoms are hidden inside @quent/hooks (no raw atom exports from barrel) | VERIFIED | `grep "export.*Atom" @quent/hooks/src/index.ts` returns nothing; only hook functions and types exported |
| 8  | atomFamily is fully replaced with record-based atoms | VERIFIED | `atoms/timeline.ts` uses `atom<Record<string, SingleTimelineResponse>>({})` as `timelineDataMapAtom`; no jotai-family import anywhere in @quent/hooks |
| 9  | No file in ui/src/ imports from old paths (@/services/api, @/atoms/dag, @/atoms/timeline, moved hooks) | VERIFIED | All grep checks return zero results; seven source files deleted; app routes and components import from @quent/client or @quent/hooks |

**Score:** 9/9 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `ui/packages/@quent/client/src/index.ts` | Named barrel for @quent/client | VERIFIED | 17 named exports including all fetch functions, queryOptions factories, and 5 pure hooks |
| `ui/packages/@quent/client/src/api.ts` | Internal apiFetch + 6 named fetch functions | VERIFIED | Contains `async function apiFetch` (unexported) and all 6 `export async function fetch*` |
| `ui/packages/@quent/utils/src/types/ZoomRange.ts` | ZoomRange interface | VERIFIED | Contains `export interface ZoomRange { start: number; end: number; }` |
| `ui/packages/@quent/hooks/src/index.ts` | Named barrel for @quent/hooks | VERIFIED | Exports all 28+ hooks; no raw atom exports; includes useHydrateTimelineAtoms (Plan 03 addition) |
| `ui/packages/@quent/hooks/src/atoms/timeline.ts` | Record-based timeline atoms (internal) | VERIFIED | Contains `atom<Record<string, SingleTimelineResponse>>` as timelineDataMapAtom; no atomFamily |
| `ui/packages/@quent/hooks/src/atoms/dag.ts` | DAG atoms (internal) | VERIFIED | Contains selectedNodeIdsAtom, selectedOperatorLabelAtom, selectedPlanIdAtom, hoveredWorkerIdAtom |
| `ui/src/lib/queryClient.ts` | QueryClient with DEFAULT_STALE_TIME from @quent/client | VERIFIED | Contains `from '@quent/client'` |
| `ui/src/components/QueryResourceTree.tsx` | Component using @quent/hooks instead of raw atoms | VERIFIED | Contains `from '@quent/hooks'` |
| `ui/src/atoms/dagControls.ts` | Visual-only DAG control atoms (app-layer, not migrated) | VERIFIED | Created by Plan 03 as home for visual atoms not in scope for @quent/hooks migration |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `@quent/client/src/api.ts` | `@quent/utils` | `import { parseJsonWithBigInt } from '@quent/utils'` | WIRED | Line 4 of api.ts |
| `@quent/client/src/queryBundle.ts` | `@quent/client/src/api.ts` | `from './api'` | WIRED | Each queryOptions file imports its corresponding fetch function from `./api` |
| `@quent/hooks/src/timeline/useBulkTimelineFetch.ts` | `@quent/client` | `from '@quent/client'` | WIRED | Line 7: `import { fetchBulkTimelines, DEFAULT_STALE_TIME } from '@quent/client'` |
| `@quent/hooks/src/timeline/useBulkTimelines.ts` | `@quent/client` | `from '@quent/client'` | WIRED | Line 7: `import { fetchBulkTimelines, DEFAULT_STALE_TIME } from '@quent/client'` |
| `ui/src/lib/queryClient.ts` | `@quent/client` | `import { DEFAULT_STALE_TIME } from '@quent/client'` | WIRED | Line 4 confirmed |
| `ui/src/routes/profile.engine.$engineId.query.$queryId.index.tsx` | `@quent/client` | `import { queryBundleQueryOptions }` | WIRED | Line 5 confirmed |
| `useBulkTimelineFetch.ts` | `timelineDataMapAtom` | `store.set(timelineDataMapAtom, prev => ...)` | WIRED | Record-based atom write pattern; line 47 confirmed |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| `@quent/client/src/api.ts` | N/A — fetch functions | Real HTTP calls via `apiFetch` to `${API_BASE_URL}/*` | Yes — calls live API endpoints | FLOWING |
| `@quent/hooks/src/atoms/timeline.ts` | `timelineDataMapAtom` | Written by `applyBulkTimelineResponse` via `useBulkTimelineFetch` | Yes — populated from API response | FLOWING |
| `@quent/hooks/src/atoms/dag.ts` | `selectedNodeIdsAtom` etc. | Written by hook setters (useSetSelectedNodeIds etc.) from component interactions | Yes — driven by user interaction, not hardcoded | FLOWING |

### Behavioral Spot-Checks

Step 7b: SKIPPED for package-level checks (no runnable entry point without dev server). TypeScript compilation is the applicable automated check here; tsc pass/fail is documented in the summaries as passing for both @quent/client and @quent/hooks independently.

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| apiFetch absent from @quent/client barrel | `grep "apiFetch" ui/packages/@quent/client/src/index.ts` | no output | PASS |
| No jotai-family anywhere in @quent/hooks | `grep -r "jotai-family" ui/packages/@quent/hooks/` | no output | PASS |
| No raw atom exports in @quent/hooks barrel | `grep "export.*Atom" ui/packages/@quent/hooks/src/index.ts` | no output | PASS |
| No old @/services/api imports in ui/src/ | `grep -r "from '@/services/api'" ui/src/` | no output | PASS |
| No old @/atoms/dag imports in ui/src/ | `grep -r "from '@/atoms/dag'" ui/src/` | no output | PASS |
| No old @/atoms/timeline imports in ui/src/ | `grep -r "from '@/atoms/timeline'" ui/src/` | no output | PASS |
| ZoomRange exported from @quent/utils | `grep "ZoomRange" ui/packages/@quent/utils/src/index.ts` | line 34 found | PASS |
| ui/src/services/api.ts deleted | `test -f ui/src/services/api.ts` | file absent | PASS |
| ui/src/atoms/dag.ts deleted | `test -f ui/src/atoms/dag.ts` | file absent | PASS |
| ui/src/atoms/timeline.ts deleted | `test -f ui/src/atoms/timeline.ts` | file absent | PASS |
| jotai-family removed from ui/package.json | `grep "jotai-family" ui/package.json` | no output | PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| CLIENT-01 | 03-01-PLAN | All fetch functions from api.ts extracted to @quent/client with full TypeScript types; no React or Jotai dependency | SATISFIED | 6 fetch functions in api.ts; @quent/client has no jotai dependency |
| CLIENT-02 | 03-01-PLAN | queryOptions factory exported alongside every fetch function | SATISFIED | 6 queryOptions factories in index.ts barrel |
| CLIENT-03 | 03-01-PLAN, 03-03-PLAN | Named hook exports for every query: useQueryBundle, useEngines, useQueryGroups, useQueries, useTimeline, useBulkTimelines | PARTIALLY SATISFIED — see note | useQueryBundle, useEngines, useQueryGroups, useQueries, useTimeline are in @quent/client; useBulkTimelines is in @quent/hooks (design decision D-01 placed Jotai-aware hooks in @quent/hooks) |
| CLIENT-04 | 03-01-PLAN | Optional staleTime parameter on all @quent/client hooks; defaults to DEFAULT_STALE_TIME | SATISFIED | All hook files accept `options?: { staleTime?: number }` per plan spec |
| CLIENT-05 | 03-01-PLAN | DEFAULT_STALE_TIME constant exported from @quent/client | SATISFIED | Exported from constants.ts, re-exported in index.ts line 5 |
| HOOKS-01 | 03-02-PLAN | atomFamily usage migrated to plain record-based atoms | SATISFIED | timelineDataMapAtom = `atom<Record<string, SingleTimelineResponse>>({})` replaces atomFamily; isTimelineHoveredAtom removed; jotai-family gone from entire codebase |
| HOOKS-02 | 03-02-PLAN | Jotai atoms remain internal to @quent/hooks; no raw atom exports | SATISFIED | @quent/hooks/src/index.ts has zero atom exports; testing.ts subpath exposes timelineDataMapAtom for tests only |
| HOOKS-03 | 03-02-PLAN | All Jotai-backed state hooks exported by name | SATISFIED | All DAG and timeline hooks exported; note: REQUIREMENTS.md says `useSelectedNodeId` (singular) but implementation uses `useSelectedNodeIds` (plural, correct — the atom holds a `Set<string>`); plural form is used consistently by all app consumers |
| HOOKS-04 | 03-02-PLAN, 03-03-PLAN | Provider scoping pattern preserved | SATISFIED | `<Provider key={queryId ?? ''}>` in `profile.engine.$engineId.tsx` wraps query-scoped components; pattern unchanged by migration |

**Note on CLIENT-03 / useBulkTimelines placement:** REQUIREMENTS.md (written 2026-04-01) listed `useBulkTimelines` under CLIENT-03 as a @quent/client export. The phase CONTEXT.md locked design decision D-01 explicitly places Jotai-aware hooks in @quent/hooks because `useBulkTimelines` reads and writes timeline atoms. The implementation follows D-01. The requirement text predates the design decision lock and uses "useBulkTimelines" as an example of the class of hooks; the intent (named hook exports for all queries) is satisfied — `useBulkTimelines` is importable from `@quent/hooks`. This is not a gap.

### Anti-Patterns Found

| File | Pattern | Severity | Impact |
|------|---------|----------|--------|
| `ui/src/components/dag/DAGChart.tsx`, `DAGLegend.tsx`, `useNodeColoring.ts`, `useDagControls.ts` | Raw `useAtomValue`/`useSetAtom` calls in app code | INFO | These access atoms from `@/atoms/dagControls` — a deliberate app-layer file for visual-only DAG control atoms (edgeWidthConfigAtom, nodeColoringAtom, etc.) that were out of scope for @quent/hooks migration per Plan 03 decision. Not a violation of the phase goal. |

No blockers. No stubs. No placeholder data.

### Human Verification Required

#### 1. Provider Scoping Reset (HOOKS-04)

**Test:** Open the app, navigate to a query, select a DAG node. Then navigate to a different query.
**Expected:** The node selection clears when switching queries, confirming the `<Provider key={queryId}>` reset mechanism works correctly after atom extraction.
**Why human:** Provider key-based reset is a runtime React behavior that requires rendering with live route transitions; grep cannot verify behavioral correctness.

#### 2. Timeline Data Flow (HOOKS-01, HOOKS-02)

**Test:** Open the app, open a query with timeline data. Verify timeline rows show populated data.
**Expected:** Timeline rows show actual data fetched via `useBulkTimelineFetch`; no console errors about missing atom imports or undefined atom access.
**Why human:** End-to-end data rendering through the record-based `timelineDataMapAtom` requires a running app with a live API backend.

### Gaps Summary

No gaps. All 9 observable truths verified. All required artifacts exist and are wired. All requirement IDs from all three plans accounted for.

The one notable interpretation: CLIENT-03's mention of `useBulkTimelines` as a @quent/client export was superseded by design decision D-01 before any code was written. The hook is available from `@quent/hooks` and is correctly used throughout the app. This reflects the requirements document predating the design lock, not an implementation gap.

---

_Verified: 2026-04-09T20:30:00Z_
_Verifier: Claude (gsd-verifier)_
