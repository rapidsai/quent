# Phase 3: Extract @quent/client and @quent/hooks - Context

**Gathered:** 2026-04-09
**Status:** Ready for planning

<domain>
## Phase Boundary

Extract two packages:

1. **`@quent/client`** — All API fetch functions and `queryOptions` factories from `ui/src/services/api.ts`. No Jotai dependency. No `apiFetch` in public API. `@quent/client` depends on `@quent/utils` (already established in Phase 2).

2. **`@quent/hooks`** — All Jotai atoms (from `ui/src/atoms/`) and every hook that directly reads/writes atoms. Atoms stay internal; only named hook functions are exported.

Hooks that mix both concerns (Jotai + API) follow the split rule: the **fetch function** lives in `@quent/client`; the **Jotai-aware wrapper hook** lives in `@quent/hooks`.

App imports migrate in this phase: `@/services/api` → `@quent/client`, `@/atoms/*` → `@quent/hooks`, `@/hooks/*` → appropriate package.

Out of scope: component extraction (Phase 4), any new API endpoints, new UI features.

</domain>

<decisions>
## Implementation Decisions

### Package split for hooks that mix API + Jotai (D-01)

- **D-01:** **Split at the concern boundary.** Fetch functions (e.g. `fetchBulkTimelines`, `fetchQueryBundle`) live in `@quent/client`. Jotai-aware wrapper hooks (e.g. `useBulkTimelines`, `useBulkTimelineFetch`) live in `@quent/hooks`. Each package owns only what it manages.
  - `@quent/client` exports: fetch functions, `queryOptions` factories, pure TanStack Query hooks with no Jotai (e.g. `useQueryBundle`, `useEngines`, `useQueryGroups`, `useQueries`), `DEFAULT_STALE_TIME`
  - `@quent/hooks` exports: all Jotai-backed hooks (`useBulkTimelines`, `useBulkTimelineFetch`, `useHighlightedItemIds`, and any other hook that reads/writes atoms); atoms remain internal
  - `@quent/hooks` declares `@quent/client` as a dependency (hooks call fetch functions); `@quent/client` has NO dependency on `@quent/hooks`

### apiFetch visibility (D-02)

- **D-02:** `apiFetch<T>` is **not exported** from `@quent/client`. It is an internal implementation detail. Consumers use the named fetch functions only. This keeps the public API surface clean and allows the fetch plumbing to evolve without breaking callers.

### Stub types in api.ts (D-03)

- **D-03:** **Drop** `ChartDataPoint`, `BarChartData`, `DashboardMetrics`, `DAGResponse`, `DAGNode`, `DAGEdge`, `NodeProfileResponse` — these are scaffolding stubs with no current consumers in the UI. They are not migrated to `@quent/client`. If real endpoints materialize, proper types will be added at that point.

### atomFamily migration (D-04)

- **D-04:** `atomFamily` from `jotai-family` is replaced with plain record-based atoms (per REQUIREMENTS HOOKS-01). `timelineDataAtom` and `isTimelineHoveredAtom` become atoms holding `Record<string, ...>` maps. The `timelineCacheKey` function is the lookup key — it stays in the package as an exported helper (consumers need it to read from the map).

### queryOptions factory pattern (D-05)

- **D-05:** Every fetch function in `@quent/client` has a matching `queryOptions` factory exported alongside it (per REQUIREMENTS CLIENT-02). Pattern follows the existing `queryBundleQueryOptions` in `useQueryBundle.ts` — same file as the hook, same module. Naming convention: `{domain}QueryOptions` (e.g. `enginesQueryOptions`, `queryBundleQueryOptions`).

### Claude's Discretion

- atomFamily replacement implementation: plain `atom<Record<string, T>>({})` with get/set helpers inside the hook is sufficient; no need for a more complex pattern
- Whether to keep `useExpandedIds`, `useDeferredReady`, `useQueryPlanVisualization` in the app or move to a package — these have no Jotai or API dependencies; they can stay in `ui/src/hooks/` for now (Phase 4 can move them to `@quent/components` if needed)
- Exact file structure within each package (one file per concern vs. grouped by domain)
- Whether to split `@quent/hooks`' index barrel by category (dag, timeline) or keep flat

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Requirements
- `.planning/REQUIREMENTS.md` §CLIENT-01 through CLIENT-05 — all @quent/client requirements
- `.planning/REQUIREMENTS.md` §HOOKS-01 through HOOKS-04 — all @quent/hooks requirements
- `.planning/ROADMAP.md` §Phase 3 — phase goal and success criteria

### Source files to extract from
- `ui/src/services/api.ts` — all fetch functions, DEFAULT_STALE_TIME, API_BASE_URL (NOT the stub types per D-03, NOT apiFetch per D-02)
- `ui/src/atoms/dag.ts` — selectedNodeIdsAtom, selectedOperatorLabelAtom, selectedPlanIdAtom, hoveredWorkerIdAtom
- `ui/src/atoms/timeline.ts` — all timeline atoms; note atomFamily usage that must be replaced per D-04
- `ui/src/hooks/useQueryBundle.ts` — queryBundleQueryOptions + useQueryBundle; this is the reference pattern for CLIENT-02/CLIENT-03
- `ui/src/hooks/useBulkTimelines.ts` — complex hook; fetch fn stays in client, hook body moves to @quent/hooks (D-01)
- `ui/src/hooks/useBulkTimelineFetch.ts` — same split as useBulkTimelines
- `ui/src/hooks/useHighlightedItemIds.ts` — uses hoveredWorkerIdAtom → goes to @quent/hooks

### Phase 1 decisions (carry forward)
- `.planning/phases/01-workspace-scaffold/01-CONTEXT.md` — D-04/D-05 (source-first resolution), D-10/D-11 (peerDependencies pattern), D-07/D-08/D-09 (tsup ESM-only)

### Package skeletons (already created in Phase 1)
- `ui/packages/@quent/client/` — exists with empty src/index.ts, package.json, tsconfig.json, tsup.config.ts
- `ui/packages/@quent/hooks/` — exists with empty src/index.ts, package.json, tsconfig.json, tsup.config.ts

### Existing pattern reference
- `ui/src/hooks/useQueryBundle.ts` — the queryOptions factory + hook pattern to replicate across all queries

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `queryBundleQueryOptions` pattern in `useQueryBundle.ts` — reference implementation for CLIENT-02 factories
- `timelineCacheKey` function in `atoms/timeline.ts` — must be exported from `@quent/hooks` alongside timeline hooks (consumers need it to address per-item data)
- Package skeletons already exist from Phase 1

### Established Patterns
- `@quent/client` → `@quent/utils` dependency already wired (api.ts already imports parseJsonWithBigInt from @quent/utils)
- peerDependencies: react, jotai, @tanstack/react-query per Phase 1 D-11
- Source-first resolution (`"main": "src/index.ts"`) per Phase 1 D-04/D-05

### Integration Points
- All files importing `@/services/api` — migrate to `@quent/client`
- All files importing `@/atoms/dag` or `@/atoms/timeline` — migrate to `@quent/hooks`
- All files importing `@/hooks/useQueryBundle`, `@/hooks/useBulkTimelines`, etc. — migrate to appropriate package
- Route files that use `queryBundleQueryOptions` in loaders — import from `@quent/client` after migration
- `@quent/hooks` package.json must declare `@quent/client` as a workspace dependency (`workspace:*`)

### Dependency direction (post-Phase 3)
- `@quent/utils` → no @quent/* deps (foundation)
- `@quent/client` → `@quent/utils` only (no Jotai, no React beyond peerDep)
- `@quent/hooks` → `@quent/client` + `@quent/utils` + jotai (peerDep)
- `@quent/components` → all of the above (Phase 4)

### Hooks NOT moving to packages (Claude's discretion)
- `useDeferredReady` — pure React, no Jotai/API; stays in app for now
- `useExpandedIds` — local state only; stays in app for now
- `useQueryPlanVisualization` — pure computation (useMemo); stays in app for now

</code_context>

<specifics>
## Specific Ideas

- The `atomFamily` in `jotai-family` causes a module-global leak because each `atomFamily` call creates atoms that persist even when the component unmounts. Replacing with `Record`-based atoms eliminates this. The `timelineCacheKey` function stays as the addressing mechanism — exported from `@quent/hooks` so consumers can read per-item data from the map atom.
- `useBulkTimelines` and `useBulkTimelineFetch` use `useStore()` from Jotai to write to atoms imperatively — this is intentional (performance optimization to avoid re-renders). The hook bodies move to `@quent/hooks`; they import fetch functions from `@quent/client`.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 03-extract-quent-client-and-quent-hooks*
*Context gathered: 2026-04-09*
