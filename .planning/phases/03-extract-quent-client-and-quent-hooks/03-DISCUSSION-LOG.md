# Phase 3: Extract @quent/client and @quent/hooks - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-04-09
**Areas discussed:** Timeline hook placement, apiFetch visibility, Stub types in api.ts

---

## Area 1: Timeline Hook Placement

**Question:** useBulkTimelines and useBulkTimelineFetch use fetchBulkTimelines (API) AND write directly to Jotai atoms via store.set(). Where should they live?

**Options presented:**
1. `@quent/client` depends on `@quent/hooks` — timeline hooks stay in @quent/client per REQUIREMENTS CLIENT-03; @quent/client declares @quent/hooks as a dep
2. Timeline hooks go to `@quent/hooks` — @quent/client stays purely fetch-layer, Jotai-aware wrappers move to @quent/hooks
3. **Split them: fetch fn in client, hook in hooks** ← Selected

**Decision:** Split at the concern boundary. Each fetch function (fetchBulkTimelines) lives in @quent/client. The Jotai-aware wrapper hooks (useBulkTimelines, useBulkTimelineFetch) live in @quent/hooks. @quent/hooks depends on @quent/client; @quent/client has no dependency on @quent/hooks.

---

## Area 2: apiFetch Visibility

**Question:** Should apiFetch<T> be exported from @quent/client as public API?

**Options presented:**
1. **Keep internal** ← Selected — apiFetch is an implementation detail; consumers use named fetch functions
2. Export it — useful for custom endpoints

**Decision:** apiFetch<T> is not exported. Internal only.

---

## Area 3: Stub Types in api.ts

**Question:** api.ts has ChartDataPoint, BarChartData, DashboardMetrics, DAGResponse/Node/Edge, NodeProfileResponse — none currently used by UI code. What should happen?

**Options presented:**
1. **Drop them** ← Selected — scaffolding stubs, no current consumers, clean slate
2. Migrate to @quent/client — safe if unsure they're dead

**Decision:** Dropped. Not migrated to @quent/client.
