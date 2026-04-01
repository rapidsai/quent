# Roadmap: Quent UI Modularization

## Overview

Extract `ui/src/` into four scoped `@quent/*` workspace packages — utils, client, hooks, components — within the existing pnpm monorepo. Phases follow the strict dependency graph: scaffold first (nothing works without it), then the foundation package, then the two sibling pure-TypeScript layers, then the component extraction and full app migration that proves the graph works end-to-end. The existing app remains functional throughout.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [x] **Phase 1: Workspace Scaffold** - Wire the pnpm workspace, tsconfig, Vite config, and per-package skeletons; no code extracted yet (completed 2026-04-01)
- [ ] **Phase 2: Extract @quent/utils** - Extract the zero-dependency foundation package; unblocks all other packages
- [ ] **Phase 3: Extract @quent/client and @quent/hooks** - Extract the two sibling pure-TypeScript layers (API client and Jotai state hooks)
- [ ] **Phase 4: Extract @quent/components and Migrate App Shell** - Extract all UI components and migrate the app to consume only `@quent/*` imports

## Phase Details

### Phase 1: Workspace Scaffold
**Goal**: The pnpm workspace, tsconfig inheritance, Vite config guards, and per-package skeletons exist; the app still builds; duplicate-instance and Tailwind purge risks are eliminated before any code moves
**Depends on**: Nothing (first phase)
**Requirements**: INFRA-01, INFRA-02, INFRA-03, INFRA-04, INFRA-05, INFRA-06
**Success Criteria** (what must be TRUE):
  1. Running `pnpm install` from `ui/` resolves all four `@quent/*` packages via `workspace:*` with no errors
  2. `tsc --noEmit` passes from `ui/` root and from inside each package directory
  3. `pnpm dev` starts without errors and the existing app UI renders normally in the browser
  4. `pnpm why react` from `ui/` shows a single hoisted React instance (no duplicates)
  5. Each package has an empty `index.ts` and a `tsup.config.ts` skeleton
**Plans:** 2/2 plans complete

Plans:
- [x] 01-01-PLAN.md — Create pnpm workspace, tsconfig base, and four @quent/* package skeletons
- [x] 01-02-PLAN.md — Integrate packages into app config (workspace deps, Vite guards, Tailwind @source, Vitest workspace)

### Phase 2: Extract @quent/utils
**Goal**: `@quent/utils` is fully extracted and all app imports of `cn()`, Rust types, formatters, color utilities, and `parseJsonWithBigInt` resolve through the package
**Depends on**: Phase 1
**Requirements**: UTILS-01, UTILS-02, UTILS-03, UTILS-04, UTILS-05
**Success Criteria** (what must be TRUE):
  1. `import { cn } from '@quent/utils'` resolves correctly inside both `ui/src/` and any package
  2. All Rust-generated TypeScript types are accessible via `@quent/utils`; the `~quent/types` path alias is removed from `vite.config.ts` and `tsconfig.json`
  3. Color and formatter utilities callable from `@quent/utils` with JSDoc visible in editor hover
  4. `pnpm dev` still starts and the app renders correctly after imports are migrated to `@quent/utils`
**Plans**: TBD

### Phase 3: Extract @quent/client and @quent/hooks
**Goal**: All API fetch functions and `queryOptions` factories live in `@quent/client`; all Jotai atoms are hidden inside `@quent/hooks` with only named hooks exported; no raw atom access exists outside `@quent/hooks`
**Depends on**: Phase 2
**Requirements**: CLIENT-01, CLIENT-02, CLIENT-03, CLIENT-04, CLIENT-05, HOOKS-01, HOOKS-02, HOOKS-03, HOOKS-04
**Success Criteria** (what must be TRUE):
  1. `import { useQueryBundle, useEngines } from '@quent/client'` resolves and the hooks function correctly in the running app
  2. `import { useSelectedNodeId, useSetSelectedNodeId } from '@quent/hooks'` resolves; selecting a DAG node updates state as before
  3. No file in `ui/src/` imports directly from `ui/src/atoms/` or `ui/src/services/api.ts` after migration
  4. The Jotai `<Provider>` per-query scoping pattern works correctly; switching between queries resets state as before
  5. `tsc --noEmit` passes from inside `ui/packages/@quent/client/` and `ui/packages/@quent/hooks/`
**Plans**: TBD

### Phase 4: Extract @quent/components and Migrate App Shell
**Goal**: All UI components live in `@quent/components`; the app shell imports everything exclusively from `@quent/*` package names; production build passes and renders correctly
**Depends on**: Phase 3
**Requirements**: COMP-01, COMP-02, COMP-03, COMP-04, COMP-05, COMP-06, COMP-07, MIG-01, MIG-02, MIG-03
**Success Criteria** (what must be TRUE):
  1. `import { DAGChart, TimelineController, Button } from '@quent/components'` resolves; the DAG and timeline render correctly in the running app
  2. No `@/components/`, `@/atoms/`, `@/lib/`, or `@/services/` imports remain in `ui/src/` (grep returns zero results)
  3. `vite build` completes without errors and bundle size is within 10% of the pre-refactor baseline
  4. `vite preview` shows correct Tailwind styles in production mode (no purged classes)
  5. All pre-existing `vitest` tests pass (`pnpm test` green from `ui/`)
**Plans**: TBD
**UI hint**: yes

## Progress

**Execution Order:**
Phases execute in numeric order: 1 → 2 → 3 → 4

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Workspace Scaffold | 2/2 | Complete   | 2026-04-01 |
| 2. Extract @quent/utils | 0/? | Not started | - |
| 3. Extract @quent/client and @quent/hooks | 0/? | Not started | - |
| 4. Extract @quent/components and Migrate App Shell | 0/? | Not started | - |
