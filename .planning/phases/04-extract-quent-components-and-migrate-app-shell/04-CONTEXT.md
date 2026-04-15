# Phase 4: Extract @quent/components and Migrate App Shell - Context

**Gathered:** 2026-04-13
**Status:** Ready for planning

<domain>
## Phase Boundary

Extract all reusable UI components into `@quent/components`; migrate the app shell so all imports resolve through `@quent/*` packages. The result: `vite build` passes, `vitest` green, and no `@/components/`, `@/lib/`, or `@/services/` imports remain in `ui/src/`.

This phase does NOT add new UI features — structural refactor only.

</domain>

<decisions>
## Implementation Decisions

### App shell residuals — what stays in `ui/src/`

- **D-01:** Page-level compositions stay in `ui/src/` — `QueryPlan.tsx`, `QueryResourceTree.tsx`, `EngineSelectionPage.tsx`, route files, `NavBarNavigator.tsx`. Only reusable, composable components go into `@quent/components`: primitives (`ui/`), DAG components, timeline components, resource-tree components, operator-timeline components, query-plan node components.
- **D-02:** `ThemeContext` stays in the app shell (`ui/src/contexts/`). It is NOT moved to `@quent/components`.

### Theme handling in the package

- **D-03:** Components that need dark/light awareness accept an explicit `isDark?: boolean` prop (or equivalent `theme` prop) at their API boundary — they do NOT import from `ThemeContext` directly. This keeps `@quent/components` free of app-shell context coupling. The app passes the theme value down from its own context.
- **D-04:** Internal hook `useTheme` calls within components are replaced at the package boundary with the `isDark` prop. Components that use `useTimelineChartColors` or similar hooks pass the derived values as props or call the hook inside the component with the prop value.

### `@/lib/` and `@/services/query-plan/` destination

- **D-05:** `@/lib/` and `@/services/query-plan/` utilities move into `@quent/components` as **internal** (non-exported) modules. They are NOT re-exported from the package's `index.ts`. Files affected:
  - `ui/src/lib/timeline.utils.ts` → `ui/packages/@quent/components/src/lib/timeline.utils.ts`
  - `ui/src/lib/resource.utils.ts` → `ui/packages/@quent/components/src/lib/resource.utils.ts`
  - `ui/src/lib/queryBundle.utils.ts` → `ui/packages/@quent/components/src/lib/queryBundle.utils.ts`
  - `ui/src/lib/echarts.ts` → `ui/packages/@quent/components/src/lib/echarts.ts`
  - `ui/src/services/query-plan/dagFieldProcessing.ts` → `ui/packages/@quent/components/src/services/query-plan/dagFieldProcessing.ts`
  - `ui/src/services/query-plan/query-bundle-transformer.ts` → `ui/packages/@quent/components/src/services/query-plan/query-bundle-transformer.ts`
  - `ui/src/services/query-plan/operationTypes.ts` → `ui/packages/@quent/components/src/services/query-plan/operationTypes.ts`
  - `ui/src/services/query-plan/types.ts` → `ui/packages/@quent/components/src/services/query-plan/types.ts`
- **D-06:** After extraction, the app shell (routes, pages) imports these utilities from `@quent/components` ONLY if they are public exports. For utilities that stay internal, the app shell should not need them directly — they are consumed by the extracted components. If any page-level code currently calls these utils directly (e.g. `QueryResourceTree.tsx` calling `timeline.utils`), those calls stay in the app and the relevant functions are exported from the package.

### `@/types.ts` entity type aliases

- **D-07:** `EntityTypeValue`, `EntityRefKey`, `EntityTypeKey` from `ui/src/types.ts` are thin type aliases/enums built on top of Rust types already in `@quent/utils`. They move to `@quent/utils` — Claude's discretion on exact placement (either new file `ui/packages/@quent/utils/src/entityTypes.ts` or appended to the existing types barrel). After this, `ui/src/types.ts` becomes a thin re-export shim or is deleted.

### DAGChart controlled API (COMP-02)

- **D-08:** DAGChart gets a controlled-first API per REQUIREMENTS COMP-02: accepts `selectedNodeIds?: string[]` and `onSelectionChange?: (nodeIds: string[]) => void`. Falls back to `@quent/hooks` atoms when controlled props are absent. This pattern applies only to node selection — other DAG state (operator label, plan selection, hovered worker) continues to use atoms via `@quent/hooks` internals.

### Component scope in `@quent/components`

- **D-09:** Components that go into the package (v1):
  - All `ui/src/components/ui/` primitives (Button, Card, Collapsible, DataText, DropdownMenu, HoverCard, Input, NavigationMenu, Popover, Resizable, ScrollArea, SelectField, Select, Skeleton, TreeTable, TreeView)
  - DAG: `DAGChart`, `DAGControls`, `DAGLegend`, `DAGSettingsPopover`
  - Timeline: `Timeline`, `TimelineController`, `TimelineSkeleton`, `TimelineToolbar`, `TimelineTooltip`, `ResourceTimeline`, `useTimelineChartColors`
  - Query-plan: `QueryPlanNode`, `OperatorStatisticsPopup`
  - Resource-tree: `ResourceColumn`, `ResourceGroupRow`, `ResourceRow`, `InlineSelector`, `UsageColumn`
  - Operator-timeline: `OperatorGanttChart` and related (merged from main)
- **D-10:** Page-level compositions stay in app shell (see D-01): `QueryPlan.tsx`, `QueryResourceTree.tsx`, `EngineSelectionPage.tsx`, `NavBarNavigator.tsx`, `ThemeToggle.tsx`.

### Package infrastructure (carried from Phase 1)

- **D-11:** Source-first dev — `package.json` `"main": "src/index.ts"`. No build step needed in dev loop. tsup for publish builds only (ESM-only, `.d.ts`).
- **D-12:** `peerDependencies` for `@quent/components`: react, @tanstack/react-query, @tanstack/react-router (for any router-aware components), jotai. `@quent/utils` and `@quent/hooks` as direct `dependencies`.
- **D-13:** `resolve.dedupe` in `vite.config.ts` already covers react, jotai, @tanstack/* — no change needed.
- **D-14:** Tailwind `@source` directive in `ui/src/index.css` already covers `ui/packages/**/*.{ts,tsx}` (INFRA-06, Phase 1) — no change needed.

### Claude's Discretion

- Internal import style within the package: use relative paths (not package barrel) for intra-package imports to avoid circular dependencies.
- Order of extraction: primitives first (no deps), then query-plan utils, then DAG, then timeline, then resource-tree. Each extraction step should leave the app working.
- Whether `ui/src/types.ts` becomes a re-export shim or is deleted — delete is cleaner if all consumers can be updated.
- Whether to export `useTimelineChartColors` from the package or keep it internal — keep internal unless a consumer needs it.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Requirements
- `.planning/REQUIREMENTS.md` — COMP-01 through COMP-07, MIG-01 through MIG-03 (the full acceptance criteria for this phase)
- `.planning/ROADMAP.md` §Phase 4 — Success criteria and requirements mapping

### Prior phase decisions
- `.planning/phases/01-workspace-scaffold/01-CONTEXT.md` — D-04/D-05 (source-first resolution), D-07/D-08/D-09 (tsup ESM-only), D-10/D-11 (peerDependencies pattern)
- `.planning/phases/02-extract-quent-utils/02-CONTEXT.md` — D-04/D-05 (color utilities now in @quent/utils, canvas patterns removed)
- `.planning/phases/03-extract-quent-client-and-quent-hooks/03-CONTEXT.md` — D-01 (split at concern boundary), D-04 (atomFamily replaced with record-based atoms)

### Existing package implementations (reference for patterns)
- `ui/packages/@quent/utils/src/index.ts` — barrel export pattern to follow
- `ui/packages/@quent/hooks/src/index.ts` — hook export pattern
- `ui/packages/@quent/components/src/index.ts` — currently empty scaffold, this is the target

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `ui/src/components/ui/` — 16 primitive components, all use `cn()` from `@quent/utils`. Ready to move.
- `ui/packages/@quent/hooks` — all timeline and DAG atoms/hooks available via `@quent/hooks` imports (components already import from here in some cases post-merge)
- `ui/packages/@quent/utils` — `cn()`, color utilities, formatters, types all available

### Established Patterns
- Components import `cn` from `@quent/utils` (already migrated in earlier sessions)
- Components import hooks from `@quent/hooks` (DAGChart, timeline components)
- Components use `@/lib/timeline.utils` heavily — this is the primary coupling to resolve
- `~quent/types` alias now present in tsconfig + vite.config (added in merge); some operator-timeline files still use it

### Integration Points
- App routes (`ui/src/routes/`) import page compositions which import extracted components — after extraction, routes still import pages, pages import from `@quent/components`
- `ui/src/index.css` Tailwind `@source` already scans packages
- `vite.config.ts` `optimizeDeps.include` already lists `@quent/components` for pre-bundling

### Key remaining `@/` import clusters (from grep)
- `@/atoms/dagControls` — DAGSettingsPopover, DAGChart (dagControls atom not yet in @quent/hooks — check if it needs to be added)
- `@/hooks/useDagControls` — used in DAG components
- `@/lib/timeline.utils` — heavy usage across timeline and resource-tree components
- `@/services/query-plan/*` — DAG and query-plan components
- `@/components/ui/*` — inter-component imports that will resolve to relative paths once in the package
- `@/types` — EntityTypeKey, EntityTypeValue, EntityRefKey used in timeline and resource components

</code_context>

<specifics>
## Specific Ideas

- The `dagControls` atom (`nodeColorPaletteAtom`, `edgeColorPaletteAtom`) is still in `ui/src/atoms/dagControls.ts` and imported by DAG components — this was not migrated in Phase 3. It likely needs to move to `@quent/hooks` as part of this phase (alongside the DAG components it serves). Researcher should verify scope.
- `useDagControls` hook in `ui/src/hooks/useDagControls.ts` — same situation, needs to move to `@quent/hooks`.
- Operator-timeline (`ui/src/components/operator-timeline/`) uses `~quent/types` imports (e.g. `utils.ts` imports QueryBundle, EntityRef from `~quent/types`). These should be migrated to `@quent/utils` as part of extraction.

</specifics>

<deferred>
## Deferred Ideas

- Storybook / component catalog — V2 (DX-V2-01 in REQUIREMENTS.md)
- Per-package README with usage examples — V2 (DX-V2-02)
- `QueryPlanTree`, `ResourceTree` as standalone exported components — V2 (COMP-V2-01, COMP-V2-02)
- npm publishability (exports field, versioning) — V2 (PUB-V2-01, PUB-V2-02)

</deferred>

---

*Phase: 04-extract-quent-components-and-migrate-app-shell*
*Context gathered: 2026-04-13*
