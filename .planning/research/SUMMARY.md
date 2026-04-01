# Project Research Summary

**Project:** Quent UI — Internal pnpm workspace package extraction
**Domain:** React/TypeScript monorepo modularization into `@quent/*` workspace packages
**Researched:** 2026-04-01
**Confidence:** HIGH (codebase-grounded; MEDIUM on tsup version pinning)

## Executive Summary

This project extracts code from `ui/src/` into four scoped packages — `@quent/utils`, `@quent/client`, `@quent/hooks`, and `@quent/components` — within a pnpm workspace at `ui/packages/@quent/*`. The goal is strict agent-legibility: an AI agent (or new developer) should be able to read each package's `index.ts` and immediately understand the full public API without reading implementation files. The existing stack (React 19, TypeScript 5.9, Vite 7, Jotai, TanStack Query 5, Tailwind v4, Radix UI, XYFlow, ECharts) stays unchanged; the only new tooling required is a `pnpm-workspace.yaml` in `ui/` and `tsup` for each package's build.

The recommended approach is source-first: packages export `"exports": { ".": "./src/index.ts" }` pointing directly at TypeScript source. Vite resolves source files transparently through `workspace:*` symlinks with no build step required during development. `tsup` is added now so the package structure is publishability-ready, but it is only run in CI or on a future publish decision. The dependency order is strict and acyclic: `@quent/utils` has no dependencies; `@quent/client` and `@quent/hooks` depend only on `@quent/utils`; `@quent/components` depends only on `@quent/utils`; the app shell composes all four. Orchestration hooks that span multiple packages (e.g., `useBulkTimelines`) stay in `ui/src/`.

The primary risks are all instance-duplication bugs: duplicate React, duplicate Jotai, or duplicate TanStack Query instances caused by listing shared libraries as `dependencies` instead of `peerDependencies` in package manifests. These are preventable with a one-time `resolve.dedupe` addition to `vite.config.ts` and correct `peerDependencies` declarations. A secondary risk is Tailwind class purging in production builds — avoided by adding `@source` directives in `ui/src/index.css` pointing at each package's `src/` directory. Both risks must be addressed during the package scaffold phase, before any code moves.

---

## Key Findings

### Recommended Stack

The existing `ui/` stack requires no new runtime dependencies. All four packages reuse `react`, `jotai`, `@tanstack/react-query`, Radix UI, and other existing libraries via pnpm workspace hoisting — declared as `peerDependencies` in each package, never as `dependencies`. The only new tooling is `tsup` (esbuild-based, de-facto standard for TypeScript library builds) and supporting packages `publint` and `@arethetypeswrong/cli` for pre-publish validation. A `tsconfig.base.json` at `ui/` centralises shared compiler options; each package extends it with `composite: true`, `declaration: true`, and `outDir: ./dist`.

**Core technologies:**
- `pnpm workspaces` (9.15 — existing): zero-overhead symlinking via `workspace:*` protocol; already in use
- `TypeScript project references` (5.9 — existing): incremental cross-package type-checking with `composite: true`
- `tsup ^8.x`: esbuild-powered ESM+CJS+`.d.ts` output in one command; understands `exports` field; used by Jotai and TanStack Query themselves
- `publint ^0.2.x` + `@arethetypeswrong/cli ^0.17.x`: validate publish-readiness from day one without publishing prematurely

**Note:** tsup version should be verified against the registry before pinning — training data cutoff is August 2025.

### Expected Features

All four packages must be fully extracted and the existing `ui/src` app migrated to consume only from package names (no relative escapes) before v1 is considered complete. Every exported symbol must have JSDoc so agents can generate correct call sites from the `index.ts` alone.

**Must have (table stakes — v1):**
- `@quent/utils` extracted: `cn()`, Rust type re-exports, `parseJsonWithBigInt`, color utilities, formatters — foundation for everything else
- `@quent/client` extracted: all fetch functions, `queryOptions` factories, named `QUERY_KEYS` constants — enables route loader prefetching
- `@quent/hooks` extracted: all named hooks wrapping atoms; atoms stay internal, never exported — decouples consumers from Jotai
- `@quent/components` extracted: barrel export of all domain components and Radix UI primitives
- `index.ts` barrel per package with explicit named exports only (no wildcards)
- JSDoc on every exported function, hook, and component
- App shell migrated to consume only from package names

**Should have (v1.x — after extraction is stable):**
- Controlled-first props on `DAGChart` (`selectedNodeIds`, `onSelectionChange`) — decouples rendering from `@quent/hooks`
- `className` pass-through audit on all components
- `staleTime` override parameter on `@quent/client` hooks
- Skeleton component generalization from `TimelineSkeleton`

**Defer (v2+):**
- npm publish preparation (exports field, bundle build, package registry)
- Per-package semver and changelogs
- Storybook or component catalog

### Architecture Approach

The package graph is intentionally acyclic and layered: `@quent/utils` is the foundation (no React, no Jotai, no TanStack dep); `@quent/client` and `@quent/hooks` are siblings that both depend on `@quent/utils` but not on each other; `@quent/components` depends only on `@quent/utils`; the app shell composes all four. The critical design choice is that hooks managing Jotai state (`@quent/hooks`) and hooks managing server state (`@quent/client`) are kept separate — the app shell wires them together at the call site. Orchestration hooks that cross this boundary (currently `useBulkTimelines`) stay in `ui/src/` until a second consumer justifies promoting them to a package.

**Major components:**
1. `@quent/utils` — shared primitives: `cn()`, Rust-generated type re-exports, formatters, color utilities, `parseJsonWithBigInt`, constants; zero React/Jotai/TanStack dependency; extracted first
2. `@quent/client` — HTTP layer: `apiFetch()`, per-resource fetch functions, `queryOptions` factories, `QUERY_KEYS`; `@tanstack/react-query` as peerDep; pure (no Jotai)
3. `@quent/hooks` — Jotai state layer: named hooks wrapping atoms; atoms in `atoms/` subdir never exported from `index.ts`; Jotai as peerDep; returns typed value+setter tuples
4. `@quent/components` — React components: DAGChart, Timeline, QueryPlanTree, ResourceTree, Radix UI primitives; all styled via `cn()` and Tailwind classes only; no CSS files shipped; `@xyflow/react` and `echarts` as direct deps

### Critical Pitfalls

1. **Duplicate React/Jotai/TanStack instances** — list all three as `peerDependencies` in every package; add all three to `resolve.dedupe` in `vite.config.ts` immediately during scaffold phase
2. **Tailwind class purging in production builds** — add `@source '../../packages/*/src/**/*.{ts,tsx}'` directives to `ui/src/index.css`; address during `@quent/components` scaffold; verify with `vite build` + `vite preview` not just `vite dev`
3. **`~quent/types` alias breaks inside packages** — only `@quent/utils` uses the alias; add the alias to `@quent/utils/tsconfig.json` explicitly; all other packages import types via `@quent/utils`, never via the raw alias
4. **Vite HMR dead across package boundaries** — add `server.watch.followSymlinks: true` and `optimizeDeps.include: ['@quent/utils', '@quent/client', '@quent/hooks', '@quent/components']` to `vite.config.ts` during scaffold
5. **Module-level mutable state in `colors.ts`** — `colorAssignments` and `usedIndices` are module-global; move mutable state to a Jotai atom in `@quent/hooks` before extracting `colors.ts` to `@quent/utils`; `@quent/utils` must export pure functions only

---

## Implications for Roadmap

Based on the dependency graph, pitfall prevention requirements, and the phased extraction order, four phases are recommended.

### Phase 1: Package Scaffold and Workspace Setup

**Rationale:** All pitfall prevention work must happen before any code moves. The Vite config, pnpm-workspace.yaml, tsconfig.base.json, and package.json skeletons are the plumbing that every subsequent phase depends on. Getting this wrong causes silent failures (duplicate instances, CSS purge) that are hard to diagnose once code is in motion.

**Delivers:** Working monorepo structure with zero code extracted yet; Vite HMR across future package boundaries; duplicate-instance protection; tsconfig patterns; empty `index.ts` per package; CI typecheck passes

**Addresses:**
- pnpm-workspace.yaml at `ui/`
- `tsconfig.base.json` at `ui/`
- Per-package `tsconfig.json` (source-mode: `noEmit: true`, `moduleResolution: bundler`)
- Per-package `tsconfig.build.json` (publish-mode: `declaration: true`, `moduleResolution: node16`)
- Per-package `package.json` with `peerDependencies`, `private: true`, `exports` pointing to `src/index.ts`
- `@source` directives in `ui/src/index.css`

**Avoids:** Pitfalls 1 (duplicate React), 2 (Tailwind purge), 6 (TanStack isolation), 7 (Vite HMR), 8 (tsconfig noEmit conflict)

---

### Phase 2: Extract @quent/utils

**Rationale:** Every other package imports from `@quent/utils`. This package has no React or Jotai dependency, making it the easiest to extract and test in isolation. The `~quent/types` alias resolution must be established here before any other package can type-check.

**Delivers:** `@quent/utils` fully extracted and app imports migrated; `~quent/types` alias resolved through package; Rust-generated types available to all packages via `@quent/utils`

**Addresses:** `cn()`, `parseJsonWithBigInt`, `formatDuration`, `formatQuantity`, color utilities (`PALETTES`, `getColorForKey`), `DEFAULT_STALE_TIME`, all Rust type re-exports; JSDoc on all exports

**Avoids:** Pitfall 4 (`~quent/types` alias), Pitfall 5 (module-global color state — audit and refactor `colors.ts` before extraction)

---

### Phase 3: Extract @quent/client and @quent/hooks

**Rationale:** These two packages are siblings (neither depends on the other) and can be extracted in the same phase. Both are pure TypeScript with no JSX — lower migration risk than components. The `queryOptions` factory pattern must be fully applied to all resources in `@quent/client` as part of this phase to enable route loader prefetching. `useBulkTimelines` stays in `ui/src/` (it is an app-shell orchestration hook).

**Delivers:** `@quent/client` with all fetch functions and `queryOptions` factories; `@quent/hooks` with all named hooks and atoms hidden; app shell wires them together at call sites; no raw atom imports outside `@quent/hooks`

**Addresses:** All hooks in `ui/src/hooks/` except `useBulkTimelines`; all fetch functions in `ui/src/services/api.ts`; all atoms in `ui/src/atoms/dag.ts` and `ui/src/atoms/timeline.ts`; `atomFamily` leak remediation (replace with record atom)

**Avoids:** Pitfall 2 (Jotai atom identity), Pitfall 3 (TanStack Query cache isolation), Pitfall 6 (anti-pattern: `@quent/hooks` importing from `@quent/client`), Pitfall 9 (`atomFamily` memory leak)

---

### Phase 4: Extract @quent/components and Migrate App Shell

**Rationale:** Components have the most code volume and the highest migration risk (JSX, Tailwind classes, XYFlow, ECharts). This phase comes last because it depends on all three preceding packages being stable. The app shell migration (removing all `@/components/`, `@/hooks/`, `@/services/` imports) is the integration test proving the full package graph works end-to-end.

**Delivers:** `@quent/components` fully extracted; app shell consuming all code via `@quent/*` package names only; no `@/` alias escapes into packages; production build verified (`vite build` + `vite preview`)

**Addresses:** DAGChart, Timeline, TimelineController, QueryPlanTree, ResourceTree, NodeDetailView, all Radix UI primitives; prop type exports; JSDoc on all components

**Avoids:** Pitfall 1 (CSS in package — no CSS files shipped from component package), Pitfall 3 (Tailwind purge — verified in prod build), anti-pattern 5 (`@/` alias escapes)

---

### Phase Ordering Rationale

- Phases 1 and 2 are strict prerequisites: no code moves until the workspace is wired, and no other package extracts until `@quent/utils` types are available
- Phases 3 and 4 are sequenced by risk and dependency: pure TypeScript layers (client/hooks) before JSX-heavy components
- `useBulkTimelines` intentionally remains in `ui/src/` throughout all phases — it is a complex orchestration hook with one call site and would require accepting `fetchBulkTimelines` as a parameter or creating a circular dep to move it
- The "looks done but isn't" checklist from PITFALLS.md should gate phase completion: `pnpm why react` single path, `vite build`+`vite preview` styles correct, `tsc --noEmit` from inside each package directory, zero `@/atoms/` imports in app after Phase 3

### Research Flags

Phases with well-documented patterns (research-phase not needed):
- **Phase 1 (Scaffold):** Standard pnpm workspace + Vite config patterns; all decisions are well-established and verified against existing `ui/` config files
- **Phase 2 (`@quent/utils`):** Pure TypeScript utility extraction; no external API surface; HIGH confidence

Phases that may benefit from targeted research during planning:
- **Phase 3 (`@quent/client` and `@quent/hooks`):** The `atomFamily` → record atom migration may have nuanced Jotai v2 specifics; verify Jotai v2 scoping behavior with `<Provider key={...}>` before the hooks extraction task is written
- **Phase 4 (`@quent/components`):** XYFlow and ECharts have specific peer dependency and CSS import requirements; verify whether `@xyflow/react/dist/style.css` must be imported at the app level and document required peer stylesheets before task writing

---

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | MEDIUM | Core decisions verified against actual `ui/` files (pnpm 9, TS 5.9, Vite 7). tsup version ^8.x from training data — verify before pinning. No live registry access during research. |
| Features | HIGH | Feature list derived from direct codebase analysis of `ui/src/`; all exports, hooks, and components inventoried. Dependency graph grounded in actual imports. |
| Architecture | HIGH | All patterns grounded in existing code; dependency graph verified against actual import chains; anti-patterns verified against existing violations (e.g., `useBulkTimelines` cross-dep). |
| Pitfalls | HIGH | All 9 pitfalls traced to specific files in the codebase. Existing concerns documented in `.planning/codebase/CONCERNS.md` corroborate findings. Recovery strategies proven against known configurations. |

**Overall confidence:** HIGH

### Gaps to Address

- **tsup version:** Training data cutoff August 2025; verify current tsup major version before pinning in `package.json` scripts. Run `npm view tsup version` at task execution time.
- **XYFlow and ECharts peer CSS requirements:** The PITFALLS research notes these as direct deps of `@quent/components`, but the specific CSS import story for each was not fully traced. The components extraction phase plan should include a step to verify required peer stylesheets and document them in `@quent/components/README` or JSDoc.
- **Jotai v2 `atomFamily` alternatives:** The recommended replacement (record atom) is architecturally sound, but the exact Jotai v2 API for per-key atom reset should be verified at implementation time.
- **Tailwind v4 `@source` syntax:** The `@source` directive syntax is drawn from Tailwind v4 docs in training data. Verify the exact path syntax relative to `ui/src/index.css` before writing the components scaffold task.

---

## Sources

### Primary (HIGH confidence — codebase analysis)
- `ui/src/atoms/dag.ts`, `ui/src/atoms/timeline.ts` — atom patterns, `atomFamily` usage
- `ui/src/hooks/useQueryBundle.ts` — `queryOptions` factory pattern (already implemented correctly)
- `ui/src/hooks/useBulkTimelines.ts`, `useBulkTimelineFetch.ts` — orchestration complexity, circular dep risk
- `ui/src/services/api.ts`, `ui/src/services/colors.ts` — module-global mutable state location
- `ui/src/index.css`, `ui/vite.config.ts` — Tailwind v4 setup, Vite alias and proxy config
- `ui/tsconfig.json`, `ui/tsconfig.node.json`, `ui/package.json`, `ui/pnpm-lock.yaml` — existing versions and patterns
- `.planning/PROJECT.md` — agent-legibility goal, publishability constraint, `workspace:*` protocol
- `.planning/codebase/ARCHITECTURE.md`, `.planning/codebase/CONCERNS.md`, `.planning/codebase/CONVENTIONS.md`

### Secondary (MEDIUM confidence — training data, August 2025 cutoff)
- tsup ^8.x documentation — build patterns, `--dts`, `--external` flags
- pnpm workspace protocol docs — `workspace:*` vs `workspace:^` semantics
- Tailwind CSS v4 `@source` directive — explicit content source declaration
- Vite `optimizeDeps.include` and `resolve.dedupe` docs

### Tertiary (training data — verify at implementation time)
- Jotai v2 `atomFamily` scoping vs `<Provider key>` interaction
- `@arethetypeswrong/cli ^0.17.x` and `publint ^0.2.x` exact versions

---
*Research completed: 2026-04-01*
*Ready for roadmap: yes*
