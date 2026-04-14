# Requirements: Quent UI Modularization

**Defined:** 2026-04-01
**Core Value:** Components, state, and API access are each independently importable with zero coupling to the app shell — an agent can read the package exports and assemble a functional UI without reading implementation details.

## v1 Requirements

### Package Infrastructure

- [x] **INFRA-01**: `ui/pnpm-workspace.yaml` created with `ui/packages/*` glob; all four packages resolved via `workspace:*` protocol
- [x] **INFRA-02**: `ui/tsconfig.base.json` created with shared compiler options; each package `tsconfig.json` extends it with `composite: true`, `declaration: true`, `noEmit: false`
- [x] **INFRA-03**: Each package has a `tsup.config.ts` (or equivalent) for publishability-ready builds (`esm` + `cjs`, `.d.ts` generation)
- [x] **INFRA-04**: `react`, `jotai`, `@tanstack/react-query`, and `@tanstack/react-router` are declared as `peerDependencies` in all packages that use them; `vite.config.ts` updated with `resolve.dedupe` for these modules
- [x] **INFRA-05**: Vitest workspace config (`ui/vitest.workspace.ts`) enables per-package test runs from the workspace root
- [x] **INFRA-06**: Tailwind CSS content scanning extended via `@source` directive in `ui/src/index.css` to cover `ui/packages/**/*.{ts,tsx}`

### @quent/utils

- [x] **UTILS-01**: `cn()` utility extracted from `ui/src/lib/utils.ts` and exported from `@quent/utils`
- [x] **UTILS-02**: All Rust-generated TypeScript types (currently at `~quent/types/*`) re-exported from `@quent/utils`; `~quent/types` path alias removed from `vite.config.ts` and `tsconfig.json` (replaced by `@quent/utils` imports)
- [x] **UTILS-03**: `parseJsonWithBigInt` utility exported from `@quent/utils`
- [x] **UTILS-04**: Color utilities extracted from `ui/src/services/colors.ts` and exported: `getOperationTypeColor`, `assignColors`, Wong palette constants
- [x] **UTILS-05**: Formatter utilities extracted from `ui/src/services/formatters.ts` and exported: duration, timestamp, and size formatters

### @quent/client

- [x] **CLIENT-01**: All fetch functions from `ui/src/services/api.ts` extracted to `@quent/client` with full TypeScript types; no React or Jotai dependency in this package
- [x] **CLIENT-02**: `queryOptions` factory exported alongside every fetch function (e.g. `queryBundleQueryOptions`, `enginesQueryOptions`) for route loader / TanStack Router prefetch compatibility
- [x] **CLIENT-03**: Named hook exports for every query: `useQueryBundle`, `useEngines`, `useQueryGroups`, `useQueries`, `useTimeline`, `useBulkTimelines`
- [x] **CLIENT-04**: Optional `staleTime` parameter on all `@quent/client` hooks; defaults to `DEFAULT_STALE_TIME` (5 minutes)
- [x] **CLIENT-05**: `DEFAULT_STALE_TIME` constant exported from `@quent/client`

### @quent/hooks

- [x] **HOOKS-01**: `atomFamily` usage in `ui/src/atoms/` migrated to plain record-based atoms (fixes module-global leak) before atoms are moved to the package
- [x] **HOOKS-02**: Jotai atoms remain internal to `@quent/hooks`; no raw atom exports; only hook functions exported
- [x] **HOOKS-03**: All Jotai-backed state hooks exported by name: `useSelectedNodeId`, `useSetSelectedNodeId`, `useSelectedPlanId`, `useSetSelectedPlanId`, `useHoveredWorkerId`, `useSetHoveredWorkerId`, and any timeline selection hooks
- [x] **HOOKS-04**: `<Provider>` scoping pattern preserved — the Jotai Provider used per-query in route files continues to work correctly after atom extraction

### @quent/components

- [ ] **COMP-01**: All Radix UI + CVA UI primitives extracted from `ui/src/components/ui/`: Button, Accordion, Dropdown, Popover, Select, and all others present — with props interfaces and CVA variant objects exported alongside each component
- [x] **COMP-02**: `DAGChart` extracted with controlled-first API: accepts `selectedNodeIds?: string[]` and `onSelectionChange?: (nodeIds: string[]) => void` props; falls back to `@quent/hooks` atoms when controlled props are not provided
- [ ] **COMP-03**: `TimelineController` and associated timeline components extracted from `ui/src/components/timeline/`
- [ ] **COMP-04**: Every exported component has JSDoc comment with purpose, `@param` for non-obvious props, and `@returns` description
- [x] **COMP-05**: Every exported component's props interface is exported alongside the component (e.g. `DAGChartProps`)
- [x] **COMP-06**: `className?: string` accepted and applied via `cn()` at the root element of every visual component
- [ ] **COMP-07**: `index.ts` barrel export lists all public exports by name (no `export *`); this is the complete API surface for the package

### App Migration

- [x] **MIG-01**: All `@/components/*`, `@/atoms/*`, `@/lib/*`, `@/services/*` imports in `ui/src/` updated to import from the appropriate `@quent/*` package
- [x] **MIG-02**: `vite build` completes without errors; bundle output is comparable to pre-refactor baseline
- [x] **MIG-03**: All existing `vitest` tests pass after migration; no test regressions

## v2 Requirements

### Additional Components

- **COMP-V2-01**: `QueryPlanTree` extracted and exported from `@quent/components`
- **COMP-V2-02**: `ResourceTree` extracted and exported from `@quent/components`
- **COMP-V2-03**: Node detail view components extracted and exported from `@quent/components`
- **COMP-V2-04**: Loading skeleton components generalized and exported (e.g. `DAGSkeleton`, `TimelineSkeleton`)

### Developer Experience

- **DX-V2-01**: Storybook or equivalent component catalog for `@quent/components`
- **DX-V2-02**: Per-package README with usage examples

### Publishability

- **PUB-V2-01**: `package.json` `exports` field configured for `@quent/*` packages with proper `types`, `import`, `require` conditions
- **PUB-V2-02**: Per-package versioning and changelogs enabled

## Out of Scope

| Feature | Reason |
|---------|--------|
| Raw Jotai atom exports from `@quent/hooks` | Breaks abstraction boundary; atoms are implementation detail |
| Singleton QueryClient inside `@quent/client` | Prevents multiple QueryClient instances; breaks tests |
| Package-level CSS imports (global-scope) | Pollutes consumer CSS; breaks tree-shaking |
| Monolithic `@quent/everything` barrel | Breaks tree-shaking; creates implicit coupling |
| Per-package versioning (this milestone) | Adds overhead before external consumers exist; design already publishability-ready |
| npm publish to registry (this milestone) | Design for it, but don't execute |
| Backend / Rust crate changes | Out of scope entirely |
| New UI features | Structural refactor only |
| CSS-in-JS or runtime theming system | Tailwind v4 already handles theming via CSS variables |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| INFRA-01 | Phase 1 | Complete |
| INFRA-02 | Phase 1 | Complete |
| INFRA-03 | Phase 1 | Complete |
| INFRA-04 | Phase 1 | Complete |
| INFRA-05 | Phase 1 | Complete |
| INFRA-06 | Phase 1 | Complete |
| UTILS-01 | Phase 2 | Complete |
| UTILS-02 | Phase 2 | Complete |
| UTILS-03 | Phase 2 | Complete |
| UTILS-04 | Phase 2 | Complete |
| UTILS-05 | Phase 2 | Complete |
| CLIENT-01 | Phase 3 | Complete |
| CLIENT-02 | Phase 3 | Complete |
| CLIENT-03 | Phase 3 | Complete |
| CLIENT-04 | Phase 3 | Complete |
| CLIENT-05 | Phase 3 | Complete |
| HOOKS-01 | Phase 3 | Complete |
| HOOKS-02 | Phase 3 | Complete |
| HOOKS-03 | Phase 3 | Complete |
| HOOKS-04 | Phase 3 | Complete |
| COMP-01 | Phase 4 | Pending |
| COMP-02 | Phase 4 | Complete |
| COMP-03 | Phase 4 | Pending |
| COMP-04 | Phase 4 | Pending |
| COMP-05 | Phase 4 | Complete |
| COMP-06 | Phase 4 | Complete |
| COMP-07 | Phase 4 | Pending |
| MIG-01 | Phase 4 | Complete |
| MIG-02 | Phase 4 | Complete |
| MIG-03 | Phase 4 | Complete |

**Coverage:**
- v1 requirements: 30 total
- Mapped to phases: 30
- Unmapped: 0 ✓

---
*Requirements defined: 2026-04-01*
*Last updated: 2026-04-01 after roadmap creation (MIG-01/02/03 moved from Phase 5 → Phase 4; app migration combined with component extraction)*
