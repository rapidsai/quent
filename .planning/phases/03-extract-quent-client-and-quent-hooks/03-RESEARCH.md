# Phase 03: Extract @quent/client and @quent/hooks - Research

**Researched:** 2026-04-09
**Domain:** TypeScript monorepo package extraction — TanStack Query + Jotai
**Confidence:** HIGH

## Summary

This phase extracts two packages from the app's existing source files. All source files were read directly — no API speculation needed. The boundaries are well-defined in CONTEXT.md decisions D-01 through D-05 and the code is straightforward to move.

The primary complexity is not the extraction itself but three pre-conditions that must be handled before or during the move: (1) `ZoomRange` is defined inside a component file (`TimelineController.tsx`) but imported by both `useBulkTimelines.ts`, `useBulkTimelineFetch.ts`, and `atoms/timeline.ts` — this type must be relocated so the extracted packages have no component-layer import dependency; (2) `atomFamily` from `jotai-family` must be replaced with record-based atoms before the atom files move to `@quent/hooks`; (3) `@quent/hooks` package.json is missing the `@quent/client` workspace dependency and the `@tanstack/react-query` peer dependency.

**Primary recommendation:** Work package by package in dependency order — relocate `ZoomRange` first, then build `@quent/client`, then build `@quent/hooks`, then update all app import sites.

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** Split at the concern boundary. Fetch functions (e.g. `fetchBulkTimelines`, `fetchQueryBundle`) live in `@quent/client`. Jotai-aware wrapper hooks (e.g. `useBulkTimelines`, `useBulkTimelineFetch`) live in `@quent/hooks`. Each package owns only what it manages.
  - `@quent/client` exports: fetch functions, `queryOptions` factories, pure TanStack Query hooks with no Jotai (e.g. `useQueryBundle`, `useEngines`, `useQueryGroups`, `useQueries`), `DEFAULT_STALE_TIME`
  - `@quent/hooks` exports: all Jotai-backed hooks (`useBulkTimelines`, `useBulkTimelineFetch`, `useHighlightedItemIds`, and any other hook that reads/writes atoms); atoms remain internal
  - `@quent/hooks` declares `@quent/client` as a dependency (hooks call fetch functions); `@quent/client` has NO dependency on `@quent/hooks`

- **D-02:** `apiFetch<T>` is **not exported** from `@quent/client`. It is an internal implementation detail. Consumers use the named fetch functions only.

- **D-03:** **Drop** `ChartDataPoint`, `BarChartData`, `DashboardMetrics`, `DAGResponse`, `DAGNode`, `DAGEdge`, `NodeProfileResponse` — these are scaffolding stubs with no current consumers in the UI. They are not migrated to `@quent/client`.

- **D-04:** `atomFamily` from `jotai-family` is replaced with plain record-based atoms (per REQUIREMENTS HOOKS-01). `timelineDataAtom` and `isTimelineHoveredAtom` become atoms holding `Record<string, ...>` maps. The `timelineCacheKey` function is the lookup key — it stays in the package as an exported helper.

- **D-05:** Every fetch function in `@quent/client` has a matching `queryOptions` factory exported alongside it. Naming convention: `{domain}QueryOptions` (e.g. `enginesQueryOptions`, `queryBundleQueryOptions`).

### Claude's Discretion

- atomFamily replacement implementation: plain `atom<Record<string, T>>({})` with get/set helpers inside the hook is sufficient; no need for a more complex pattern
- Whether to keep `useExpandedIds`, `useDeferredReady`, `useQueryPlanVisualization` in the app or move to a package — these have no Jotai or API dependencies; they can stay in `ui/src/hooks/` for now
- Exact file structure within each package (one file per concern vs. grouped by domain)
- Whether to split `@quent/hooks`' index barrel by category (dag, timeline) or keep flat

### Deferred Ideas (OUT OF SCOPE)

None — discussion stayed within phase scope.
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| CLIENT-01 | All fetch functions from `ui/src/services/api.ts` extracted to `@quent/client` with full TypeScript types; no React or Jotai dependency | Source file read: 6 named fetch fns identified; apiFetch stays internal (D-02); stub types dropped (D-03) |
| CLIENT-02 | `queryOptions` factory exported alongside every fetch function | Pattern exists in `useQueryBundle.ts`; must replicate for all 5 fetch functions |
| CLIENT-03 | Named hook exports for every query: `useQueryBundle`, `useEngines`, `useQueryGroups`, `useQueries`, `useTimeline`, `useBulkTimelines` | `useQueryBundle` is the reference; others are new; `useBulkTimelines` is the Jotai-free fetch-only version |
| CLIENT-04 | Optional `staleTime` parameter on all `@quent/client` hooks; defaults to `DEFAULT_STALE_TIME` | `useQueryBundle` already uses staleTime from queryOptions; pattern extends naturally |
| CLIENT-05 | `DEFAULT_STALE_TIME` constant exported from `@quent/client` | Currently in `api.ts` as `export const DEFAULT_STALE_TIME = 5 * 60 * 1000`; move directly |
| HOOKS-01 | `atomFamily` usage migrated to plain record-based atoms before atoms move to package | Two atomFamily calls found: `timelineDataAtom` and `isTimelineHoveredAtom` in `atoms/timeline.ts` |
| HOOKS-02 | Jotai atoms remain internal to `@quent/hooks`; only hook functions exported | All atoms become internal; `timelineCacheKey` exported as helper |
| HOOKS-03 | All Jotai-backed state hooks exported by name: `useSelectedNodeId`, `useSetSelectedNodeId`, `useSelectedPlanId`, `useSetSelectedPlanId`, `useHoveredWorkerId`, `useSetHoveredWorkerId`, and any timeline selection hooks | Currently no hook wrappers exist for dag atoms — they are used as raw atoms in components. Wrapping hooks must be written fresh. |
| HOOKS-04 | `<Provider>` scoping pattern preserved — the Jotai Provider used per-query in route files continues to work correctly after atom extraction | Provider wraps atoms, not the package. No change needed to Provider usage if atoms maintain the same shape. |
</phase_requirements>

---

## Standard Stack

### Core (Already Established in Phases 1-2)

| Library | Version | Purpose | Status |
|---------|---------|---------|--------|
| jotai | ^2.0.0 | Atom state management (peerDep in @quent/hooks) | Already wired in Phase 1 |
| @tanstack/react-query | ^5.0.0 | Server state, queryOptions factories (peerDep) | Already wired in Phase 1 |
| @tanstack/react-router | ^1.0.0 | Route loaders use queryOptions (peerDep) | Already wired in Phase 1 |
| @quent/utils | workspace:* | Types + utilities | Dependency in @quent/client (already wired in api.ts) |

### What Must Be Added to Package JSONs

`@quent/hooks/package.json` is currently missing two declarations:

1. `@quent/client` as a workspace dependency: `"dependencies": { "@quent/client": "workspace:*" }`
2. `@tanstack/react-query` as a peer dependency (hooks call `useQuery`, `useQueryClient`)

Current `@quent/hooks/package.json` peers: `react ^19.0.0`, `jotai ^2.0.0` — `@tanstack/react-query` is missing.

`@quent/client/package.json` peers are correct (`react`, `@tanstack/react-query`, `@tanstack/react-router`). No `@quent/utils` dependency is declared — but `api.ts` already imports from `@quent/utils`. Must add: `"dependencies": { "@quent/utils": "workspace:*" }`.

---

## Architecture Patterns

### Recommended Package Structure

```
ui/packages/@quent/client/src/
├── index.ts              # named barrel: all public exports
├── api.ts                # apiFetch (internal), API_BASE_URL, all fetch functions
├── constants.ts          # DEFAULT_STALE_TIME
├── queryBundle.ts        # queryBundleQueryOptions + useQueryBundle
├── engines.ts            # enginesQueryOptions + useEngines
├── queryGroups.ts        # queryGroupsQueryOptions + useQueryGroups
├── queries.ts            # queriesQueryOptions + useQueries
├── timeline.ts           # singleTimelineQueryOptions + useTimeline
└── bulkTimelines.ts      # bulkTimelinesQueryOptions (no Jotai hook — goes to hooks pkg)

ui/packages/@quent/hooks/src/
├── index.ts              # named barrel: all public hook exports + timelineCacheKey
├── atoms/
│   ├── dag.ts            # dag atoms (internal)
│   └── timeline.ts       # timeline atoms with record-based replacement (internal)
├── dag/
│   ├── useSelectedNodeIds.ts
│   ├── useSelectedPlanId.ts
│   ├── useHoveredWorkerId.ts
│   └── useSelectedOperatorLabel.ts
└── timeline/
    ├── useTimelineAtoms.ts    # re-exports timelineCacheKey, atom helpers
    ├── useBulkTimelines.ts    # moved from app
    └── useBulkTimelineFetch.ts # moved from app + useHighlightedItemIds
```

Note: Exact file grouping is Claude's discretion per D-01. The above is a recommendation.

### Pattern 1: queryOptions Factory + Hook (Reference Pattern)

Established in `useQueryBundle.ts`. All five fetch functions follow this same pattern.

```typescript
// Source: ui/src/hooks/useQueryBundle.ts (reference — copy this pattern)
import { DEFAULT_STALE_TIME, fetchQueryBundle } from './api';
import { queryOptions, useQuery } from '@tanstack/react-query';

interface QueryBundleParams { engineId: string; queryId: string; }

export const queryBundleQueryOptions = ({ engineId, queryId }: QueryBundleParams) =>
  queryOptions({
    queryKey: ['queryBundle', engineId, queryId],
    queryFn: async () => fetchQueryBundle(engineId, queryId),
    staleTime: DEFAULT_STALE_TIME,
    retry: 2,
  });

export const useQueryBundle = (
  params: QueryBundleParams,
  options?: { staleTime?: number }
) => {
  return useQuery({
    ...queryBundleQueryOptions(params),
    staleTime: options?.staleTime ?? DEFAULT_STALE_TIME,
  });
};
```

CLIENT-04 (optional staleTime on hooks) requires the hook to accept an options bag. The queryOptions factory always uses `DEFAULT_STALE_TIME`; the hook spreads and overrides.

### Pattern 2: Record-Based Atom Replacing atomFamily (HOOKS-01)

The two `atomFamily` calls in `atoms/timeline.ts` must be replaced before the file can move.

**Before (jotai-family):**
```typescript
export const timelineDataAtom = atomFamily(() =>
  atom<SingleTimelineResponse | undefined>(undefined)
);
// Usage: store.set(timelineDataAtom(key), value)
// Usage: useAtomValue(timelineDataAtom(key))
```

**After (record-based, internal to @quent/hooks):**
```typescript
// INTERNAL — not exported
const _timelineDataAtom = atom<Record<string, SingleTimelineResponse>>({});
const _isTimelineHoveredAtom = atom<string | null>(null); // already exists as hoveredTimelineIdAtom

// Getter hook pattern (replaces useAtomValue(timelineDataAtom(key)))
// Callers who need per-key reads: expose selector helpers or accept key param in hook
```

The calling code in `useBulkTimelineFetch.ts` uses `store.set(timelineDataAtom(key), value)` and `store.get(timelineDataAtom(key))`. After migration these become `store.set(_timelineDataAtom, prev => ({ ...prev, [key]: value }))` and `store.get(_timelineDataAtom)[key]`.

In `ResourceTimeline.tsx` the atom is used as: `useAtomValue(timelineDataAtom(key))`. After migration this becomes a hook call like `useTimelineData(key)` exported from `@quent/hooks`. The raw atom stays internal.

For `isTimelineHoveredAtom`: this is a derived atom `atom(get => get(hoveredTimelineIdAtom) === itemId)`. The record replacement means: the `hoveredTimelineIdAtom` stays as-is (already a plain atom); the derived check moves into `useIsTimelineHovered(itemId)` hook that calls `useAtomValue(hoveredTimelineIdAtom) === itemId`.

### Pattern 3: Dag Atom Wrapping (New Hooks, HOOKS-03)

No hook wrappers currently exist for dag atoms. They are consumed as raw atoms via `useAtomValue`/`useSetAtom` throughout the app. The wrapping hooks must be written from scratch.

```typescript
// @quent/hooks/src/dag/useSelectedNodeIds.ts
import { useAtomValue, useSetAtom } from 'jotai';
import { selectedNodeIdsAtom } from '../atoms/dag';

export const useSelectedNodeIds = () => useAtomValue(selectedNodeIdsAtom);
export const useSetSelectedNodeIds = () => useSetAtom(selectedNodeIdsAtom);
```

HOOKS-03 lists: `useSelectedNodeId`, `useSetSelectedNodeId`, `useSelectedPlanId`, `useSetSelectedPlanId`, `useHoveredWorkerId`, `useSetHoveredWorkerId`. Note: `selectedNodeIdsAtom` holds a `Set<string>` (plural), not a single ID. The requirements list singular `useSelectedNodeId` — confirm naming matches the existing atom shape.

### Pattern 4: Jotai Provider Scoping (HOOKS-04)

The Jotai `<Provider>` wraps the component subtree in route files. The Provider is created in the app, not in `@quent/hooks`. Atoms are instantiated inside the Provider's scope. After extraction, this works identically because the atoms are still the same Jotai atom objects — just imported from `@quent/hooks` instead of `@/atoms/*`. No change to Provider usage required (confirmed: Provider scoping is a runtime concern, not an import-path concern).

---

## Critical Pre-Condition: ZoomRange Type Relocation

**Problem:** `ZoomRange` is defined in `ui/src/components/timeline/TimelineController.tsx` (a component file). Three files that need to move to packages import it:

- `ui/src/atoms/timeline.ts` — imports `ZoomRange` (for `zoomRangeAtom` and `debouncedZoomRangeAtom` types)
- `ui/src/hooks/useBulkTimelines.ts` — imports `ZoomRange`
- `ui/src/hooks/useBulkTimelineFetch.ts` — imports `ZoomRange`

When these move to `@quent/hooks`, importing from `@/components/timeline/TimelineController` would create a package dependency on the app's component layer. This is circular and wrong.

**Resolution:** Move the `ZoomRange` interface out of `TimelineController.tsx` into `@quent/utils` (or into `@quent/hooks` as a shared type). Since `ZoomRange` is a simple `{ start: number; end: number }` with no library dependencies, it belongs in `@quent/utils` alongside other shared types.

**Action:** In Wave 1 (pre-conditions), extract `ZoomRange` from `TimelineController.tsx`, export it from `@quent/utils`, and update all import sites before moving the hook files.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| queryOptions factories | Custom caching object | `queryOptions()` from `@tanstack/react-query` | Type-safe, integrates with loader prefetch, cache key deduplication |
| Record-based atom updates | Mutable atom mutation | Immutable update: `set(atom, prev => ({ ...prev, [key]: val }))` | Jotai atoms require new object reference to trigger re-renders |
| Atom family simulation | Custom WeakMap registry | Plain `atom<Record<string, T>>({})` | Simpler, no module-level leak, already decided D-04 |
| Workspace package linking | Copying source | `workspace:*` pnpm protocol | Source-first resolution, no build step in dev |

---

## Common Pitfalls

### Pitfall 1: Circular Dependency via ZoomRange
**What goes wrong:** `@quent/hooks` imports `ZoomRange` from `@/components/timeline/TimelineController` — the package imports from the app's component layer.
**Why it happens:** `ZoomRange` was defined in a component file as a convenience.
**How to avoid:** Move `ZoomRange` to `@quent/utils` before moving any hook files. Update all three import sites simultaneously.
**Warning signs:** TypeScript error "Cannot find module '@/components/timeline/TimelineController'" when typechecking the package.

### Pitfall 2: Missing peerDependency Declarations
**What goes wrong:** `@quent/hooks` calls `useQuery`/`useQueryClient` but doesn't declare `@tanstack/react-query` as a peerDependency.
**Why it happens:** The package.json skeleton only lists `react` and `jotai`.
**How to avoid:** Add `"@tanstack/react-query": "^5.0.0"` to `@quent/hooks` peerDependencies. Add `"@quent/utils": "workspace:*"` to `@quent/client` dependencies.
**Warning signs:** Vite duplicate module warning at runtime; TypeScript "cannot find module" at typecheck.

### Pitfall 3: atomFamily Call Sites After Migration
**What goes wrong:** `applyBulkTimelineResponse` calls `store.set(timelineDataAtom(key), ...)` — the function signature changes when `timelineDataAtom` becomes a Record atom.
**Why it happens:** `applyBulkTimelineResponse` is exported and used in `QueryResourceTree.test.tsx`. If the atom shape changes but the function isn't updated, tests break.
**How to avoid:** `applyBulkTimelineResponse` must be updated in the same commit that changes `timelineDataAtom`. The test file imports `applyBulkTimelineResponse` from `@/hooks/useBulkTimelineFetch` — after migration the import path changes to `@quent/hooks`.
**Warning signs:** Type error on `store.set(timelineDataAtom(key), ...)` — argument is no longer a writable atom.

### Pitfall 4: Test File Import Paths
**What goes wrong:** `QueryResourceTree.test.tsx` imports from `@/hooks/useBulkTimelineFetch`, `@/atoms/timeline`, and `@/services/api`. These all change to `@quent/hooks` / `@quent/client`.
**Why it happens:** Tests mock the app-layer paths; after migration those paths no longer exist.
**How to avoid:** The test file must be updated as part of the import migration wave, not as an afterthought.
**Warning signs:** Vitest "Cannot find module '@/hooks/useBulkTimelineFetch'" error.

### Pitfall 5: queryClient.ts Uses DEFAULT_STALE_TIME from App
**What goes wrong:** `ui/src/lib/queryClient.ts` imports `DEFAULT_STALE_TIME` from `@/services/api`. After `api.ts` no longer re-exports it (or the file is emptied), this import breaks.
**Why it happens:** `api.ts` currently exports both fetch functions and the constant.
**How to avoid:** Update `queryClient.ts` import to `@quent/client` as part of the import migration wave.

### Pitfall 6: Immutable Record Updates in Jotai
**What goes wrong:** Updating a Record atom with mutation (`prev[key] = value`) does not trigger Jotai subscribers.
**Why it happens:** Jotai atom equality check uses reference equality.
**How to avoid:** Always return a new object: `set(atom, prev => ({ ...prev, [key]: value }))`.

### Pitfall 7: useStore() Imperative Writes Across Provider Boundaries
**What goes wrong:** `useBulkTimelines` and `useBulkTimelineFetch` both call `useStore()` to write atoms imperatively. If a different Jotai store instance is used by the hook vs. the component reading the atom, writes are invisible to readers.
**Why it happens:** `useStore()` returns the store from the nearest Provider in the React tree. The pattern is correct as long as everything is inside the same Provider.
**How to avoid:** No change needed — the Provider scoping is unchanged (HOOKS-04). Document this as an assumption in the plan.

---

## Code Examples

### queryOptions Factory: New Patterns Needed for All Fetch Functions

The reference pattern from `useQueryBundle.ts` needs to be replicated for 5 functions. Below are the required factories that don't yet exist:

```typescript
// enginesQueryOptions — for fetchListEngines
export const enginesQueryOptions = (options?: { staleTime?: number }) =>
  queryOptions({
    queryKey: ['list_engines'],
    queryFn: fetchListEngines,
    staleTime: options?.staleTime ?? DEFAULT_STALE_TIME,
  });

// queryGroupsQueryOptions — for fetchListCoordinators
export const queryGroupsQueryOptions = (
  engineId: string,
  options?: { staleTime?: number }
) =>
  queryOptions({
    queryKey: ['list_coordinators', engineId],
    queryFn: () => fetchListCoordinators(engineId),
    staleTime: options?.staleTime ?? DEFAULT_STALE_TIME,
    enabled: !!engineId,
  });

// queriesQueryOptions — for fetchListQueries
export const queriesQueryOptions = (
  engineId: string,
  coordinatorId: string,
  options?: { staleTime?: number }
) =>
  queryOptions({
    queryKey: ['list_queries', engineId, coordinatorId],
    queryFn: () => fetchListQueries(engineId, coordinatorId),
    staleTime: options?.staleTime ?? DEFAULT_STALE_TIME,
    enabled: !!engineId && !!coordinatorId,
  });

// singleTimelineQueryOptions — for fetchSingleTimeline
// (complex params; exact shape to be determined during implementation)
export const singleTimelineQueryOptions = (...) =>
  queryOptions({ queryKey: ['singleTimeline', ...], ... });
```

### Record-Based Atom Replacement

```typescript
// INTERNAL to @quent/hooks — not exported
const _timelineDataMap = atom<Record<string, SingleTimelineResponse>>({});
const _isTimelineHoveredAtom = atom<string | null>(null);  // same as hoveredTimelineIdAtom

// EXPORTED helper
export { timelineCacheKey } from './atoms/timeline';

// EXPORTED hook replacing useAtomValue(timelineDataAtom(key))
export function useTimelineData(key: string): SingleTimelineResponse | undefined {
  const map = useAtomValue(_timelineDataMap);
  return map[key];
}

// EXPORTED hook replacing isTimelineHoveredAtom(itemId)
export function useIsTimelineHovered(itemId: string): boolean {
  const hoveredId = useAtomValue(_isTimelineHoveredAtom);
  return hoveredId === itemId;
}
```

### applyBulkTimelineResponse After Record Migration

```typescript
// Updated signature — store is still useStore() result
export function applyBulkTimelineResponse(
  response: BulkTimelinesResponse,
  idToMeta: Map<string, BulkTimelineIdMeta>,
  store: ReturnType<typeof import('jotai').useStore>
): void {
  const updates: Record<string, SingleTimelineResponse> = {};
  for (const [id, entry] of Object.entries(response.entries)) {
    if (entry?.status !== 'ok') continue;
    const meta = idToMeta.get(id);
    if (!meta) continue;
    const key = timelineCacheKey(meta);
    updates[key] = { data: entry.data, config: entry.config };
  }
  if (Object.keys(updates).length > 0) {
    store.set(_timelineDataMap, prev => ({ ...prev, ...updates }));
  }
}
```

---

## Import Migration Map

All app files that import from the modules being extracted, and what they migrate to:

### @/services/api → @quent/client

| File | What it imports |
|------|-----------------|
| `ui/src/lib/queryClient.ts` | `DEFAULT_STALE_TIME` |
| `ui/src/hooks/useQueryBundle.ts` | `DEFAULT_STALE_TIME`, `fetchQueryBundle` |
| `ui/src/hooks/useBulkTimelines.ts` | `fetchBulkTimelines`, `DEFAULT_STALE_TIME` |
| `ui/src/hooks/useBulkTimelineFetch.ts` | `fetchBulkTimelines`, `DEFAULT_STALE_TIME` |
| `ui/src/components/timeline/ResourceTimeline.tsx` | `DEFAULT_STALE_TIME`, `fetchSingleTimeline` |
| `ui/src/components/QueryResourceTree.tsx` | (check actual imports) |
| `ui/src/components/QueryResourceTree.test.tsx` | `@/services/api` (mock) |
| `ui/src/components/NavBarNavigator.tsx` | `fetchListEngines`, `fetchListCoordinators`, `fetchListQueries` |
| `ui/src/pages/EngineSelectionPage.tsx` | `fetchListEngines`, `fetchListCoordinators`, `fetchListQueries` |

### @/atoms/dag → @quent/hooks (raw atom → named hook)

| File | Raw atom used → Migrates to hook |
|------|----------------------------------|
| `ui/src/hooks/useBulkTimelines.ts` | `selectedNodeIdsAtom` → `useSelectedNodeIds()` |
| `ui/src/hooks/useHighlightedItemIds.ts` | `hoveredWorkerIdAtom` → `useHoveredWorkerId()` |
| `ui/src/components/timeline/ResourceTimeline.tsx` | `selectedNodeIdsAtom`, `selectedOperatorLabelAtom` → hooks |
| `ui/src/components/dag/DAGChart.tsx` | (check — likely uses dag atoms) |
| `ui/src/components/query-plan/QueryPlanNode.tsx` | (check — likely uses dag atoms) |
| `ui/src/components/QueryPlan.tsx` | (check) |
| `ui/src/components/QueryResourceTree.tsx` | (check) |
| `ui/src/components/QueryResourceTree.test.tsx` | (mock) |

### @/atoms/timeline → @quent/hooks (raw atom → named hook or exported helper)

| File | Raw atom → Migrates to |
|------|------------------------|
| `ui/src/hooks/useBulkTimelines.ts` | multiple timeline atoms → hook internals |
| `ui/src/hooks/useBulkTimelineFetch.ts` | `timelineCacheKey`, `timelineDataAtom` → internals |
| `ui/src/components/timeline/TimelineController.tsx` | `zoomRangeAtom` → hook |
| `ui/src/components/timeline/Timeline.tsx` | (check) |
| `ui/src/components/timeline/TimelineToolbar.tsx` | (check) |
| `ui/src/components/timeline/ResourceTimeline.tsx` | multiple atoms → hooks |
| `ui/src/components/resource-tree/UsageColumn.tsx` | (check) |
| `ui/src/components/query-plan/QueryPlanNode.tsx` | (check) |
| `ui/src/components/dag/DAGChart.tsx` | (check) |
| `ui/src/components/QueryResourceTree.tsx` | (check) |
| `ui/src/components/QueryResourceTree.test.tsx` | `timelineCacheKey`, `timelineDataAtom` → `@quent/hooks` |

### @/hooks/(useQueryBundle|useBulkTimelines|useBulkTimelineFetch|useHighlightedItemIds) → package

| File | Imports → New location |
|------|------------------------|
| `ui/src/routes/profile.engine.$engineId.query.$queryId.index.tsx` | `queryBundleQueryOptions` → `@quent/client` |
| `ui/src/routes/profile.engine.$engineId.query.$queryId.node.$nodeId.tsx` | (same) |
| `ui/src/components/NavBarNavigator.tsx` | `queryBundleQueryOptions` → `@quent/client` |
| `ui/src/components/QueryResourceTree.tsx` | `useBulkTimelines`, `useHighlightedItemIds` → `@quent/hooks` |
| `ui/src/components/QueryResourceTree.test.tsx` | `applyBulkTimelineResponse` → `@quent/hooks` |
| `ui/src/components/QueryPlan.tsx` | (check which hooks) |

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Vitest (workspace mode) |
| Config file | `ui/vitest.config.ts` (app), `ui/vitest.workspace.ts` (workspace) |
| Quick run command | `pnpm --filter @quent/client typecheck && pnpm --filter @quent/hooks typecheck` |
| Full suite command | `cd ui && pnpm vitest run` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| CLIENT-01 | Fetch functions exported, no React/Jotai import | typecheck | `pnpm --filter @quent/client typecheck` | ❌ Wave 0 |
| CLIENT-02 | queryOptions factories exist and are usable in loaders | type + smoke | typecheck + existing route test | ✅ (route files) |
| CLIENT-03 | Named hooks exported and callable | typecheck | `pnpm --filter @quent/client typecheck` | ❌ Wave 0 |
| CLIENT-04 | staleTime param on hooks | typecheck | same | ❌ Wave 0 |
| CLIENT-05 | DEFAULT_STALE_TIME exported | typecheck | same | ❌ Wave 0 |
| HOOKS-01 | atomFamily replaced; no jotai-family import | grep + typecheck | `grep -r "jotai-family" ui/packages` returns empty | ❌ Wave 0 |
| HOOKS-02 | No atom exports from @quent/hooks | typecheck + grep | `grep -r "export.*Atom" ui/packages/@quent/hooks/src/index.ts` returns empty | ❌ Wave 0 |
| HOOKS-03 | Named hooks exported | typecheck | `pnpm --filter @quent/hooks typecheck` | ❌ Wave 0 |
| HOOKS-04 | Provider scoping works post-migration | integration | `cd ui && pnpm vitest run` (existing QueryResourceTree test) | ✅ existing test |

### Sampling Rate

- **Per task commit:** `pnpm --filter @quent/client typecheck` or `pnpm --filter @quent/hooks typecheck` depending on which package was touched
- **Per wave merge:** `cd ui && pnpm vitest run` — full suite including `QueryResourceTree.test.tsx`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps

- [ ] `ui/packages/@quent/client/vitest.config.ts` — if package-level tests are wanted (not required; typecheck is sufficient for CLIENT requirements)
- [ ] `ui/packages/@quent/hooks/vitest.config.ts` — same
- Framework install: already installed (pnpm workspace, vitest present)

---

## Environment Availability

Step 2.6: All dependencies are pure workspace packages — no external services, databases, or CLIs beyond pnpm/TypeScript/Vitest already in use.

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| pnpm | workspace management | ✓ | enforced by preinstall | — |
| TypeScript | typechecking | ✓ | ^5.9.3 | — |
| jotai | @quent/hooks peerDep | ✓ | ^2.x | — |
| @tanstack/react-query | @quent/client peerDep | ✓ | ^5.x | — |
| jotai-family | REMOVED per HOOKS-01 | ✓ (but being dropped) | current | no fallback needed — being replaced |

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `atomFamily` from `jotai-family` | `atom<Record<string, T>>({})` | This phase (D-04) | Eliminates module-level memory leak; call sites change |
| `import.meta.env.VITE_API_BASE_URL` in package | Keep in @quent/client | — | Vite replaces env vars at bundle time; works in package source |

---

## Open Questions

1. **`useTimeline` hook in CLIENT-03 — does this mean `useSingleTimeline` wrapping `fetchSingleTimeline`?**
   - What we know: `fetchSingleTimeline` exists; `useBulkTimelines` is listed in CLIENT-03 but uses Jotai (goes to @quent/hooks per D-01 — the pure non-Jotai version for CLIENT-03 must be a new `useTimeline` hook that calls `useQuery` with no atom involvement)
   - What's unclear: whether `useBulkTimelines` in CLIENT-03 is the Jotai version (wrong, should be @quent/hooks) or a Jotai-free bulk query hook
   - Recommendation: CLIENT-03's `useBulkTimelines` refers to a pure `useQuery`-based hook without atom writes — a thin wrapper. The Jotai-enhanced version with imperative store writes is separate and lives in `@quent/hooks`.

2. **Dag atom consumers in component files**
   - What we know: 13 files import from `@/atoms/*` (components + hooks). Phase 3 only migrates atoms to `@quent/hooks`; component migration is Phase 4.
   - What's unclear: Whether component files in Phase 3 should already consume `@quent/hooks` named hooks, or keep using raw atoms from `@/atoms/*` (which would continue to exist as thin re-exports or be deleted)
   - Recommendation: Phase 3 scope per CONTEXT.md is "App imports migrate in this phase: `@/atoms/*` → `@quent/hooks`". This means ALL atom import sites (including component files) migrate to `@quent/hooks` named hooks in Phase 3, not Phase 4. Plan should include component file import updates.

3. **`QueryResponse` interface in api.ts**
   - What we know: `export interface QueryResponse { id: string; }` — listed alongside stub types but simpler. No consumers found.
   - Recommendation: Drop along with the other stub types per D-03.

---

## Sources

### Primary (HIGH confidence)
- Direct source file reads: `ui/src/services/api.ts`, `ui/src/atoms/dag.ts`, `ui/src/atoms/timeline.ts`, `ui/src/hooks/useQueryBundle.ts`, `ui/src/hooks/useBulkTimelines.ts`, `ui/src/hooks/useBulkTimelineFetch.ts`, `ui/src/hooks/useHighlightedItemIds.ts`
- Package skeleton reads: `ui/packages/@quent/client/package.json`, `ui/packages/@quent/hooks/package.json`, both `tsconfig.json` and `tsup.config.ts`
- Context docs: `03-CONTEXT.md`, `01-CONTEXT.md`, `REQUIREMENTS.md`, `STATE.md`
- Import site audit: grep across all `ui/src/` files for `@/services/api`, `@/atoms/`, and moving hooks

### Secondary (MEDIUM confidence)
- `QueryResourceTree.test.tsx` read — identifies test import paths that must migrate
- `NavBarNavigator.tsx` and `EngineSelectionPage.tsx` read — confirms fetch function usage patterns
- `TimelineController.tsx` read — confirms `ZoomRange` is defined there (type relocation required)

### Tertiary (LOW confidence)
- Component files that import from `@/atoms/*` were identified via grep but not all read individually; exact hook wrappers needed per file may vary from what's listed above

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all packages/versions read directly from package.json files
- Architecture: HIGH — all source files read; patterns are direct translations
- Pitfalls: HIGH — identified from actual code (ZoomRange type location, missing peerDeps, atomFamily call sites in tests)
- Import migration map: MEDIUM — grep found all files; component file exact atom usage not exhaustively read

**Research date:** 2026-04-09
**Valid until:** 2026-05-09 (stable codebase, no fast-moving dependencies)
