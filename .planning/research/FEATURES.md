# Feature Research

**Domain:** React UI package library (internal monorepo — @quent/components, @quent/hooks, @quent/client, @quent/utils)
**Researched:** 2026-04-01
**Confidence:** HIGH (based on direct codebase analysis + established ecosystem knowledge)

---

## Feature Landscape

### Table Stakes (Users Expect These)

Features that agents and developers expect any package library to provide. Missing these means the
packages cannot be composed without reading implementation internals.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Clean barrel `index.ts` per package | Agent reads `index.ts` to discover the full API surface — no other file should need to be opened | LOW | Each package has one authoritative export file; everything the consumer needs is re-exported there |
| Explicit TypeScript types on every export | Consumers type-check without running code; agents generate correct call sites from type signatures alone | LOW | Already present in codebase; must be preserved and made part of public surface, not internal |
| Props interfaces exported alongside components | Callers extend or pass types without importing from internal paths | LOW | e.g. `DAGChartProps`, `TimelineControllerProps` — export the interface, not just the component |
| `queryOptions` factories exported from `@quent/client` | Allows route loaders to pre-populate TanStack Query cache with the same key used by hooks; without this consumers duplicate keys | MEDIUM | `queryBundleQueryOptions` pattern already exists in `useQueryBundle.ts` — lift to package export |
| Named hook exports (no default exports) from `@quent/hooks` | Agents discover hooks by name from the barrel; default exports are invisible to index scanning | LOW | Matches existing convention (`export function useQueryBundle`) |
| No raw Jotai atom exports from `@quent/hooks` | Consumers should not couple to atom identity; hooks hide the state primitive entirely | LOW | Atoms currently in `atoms/dag.ts` and `atoms/timeline.ts` must remain internal; only hook functions exported |
| `cn()` utility exported from `@quent/utils` | Every component that accepts `className` needs `cn`; if it is not in utils, consumers must add clsx+tailwind-merge themselves | LOW | Already exists in `lib/utils.ts` — trivial to re-export |
| Rust-generated TypeScript type re-exports from `@quent/utils` | All four packages share these types; importing from `~quent/types/*` directly is a path alias that breaks outside the app | MEDIUM | The `~quent/types/*` alias must be resolved to real package paths in `@quent/utils`; this is the foundational dependency for all other packages |
| Loading and error state handling per hook | TanStack Query exposes `isLoading`, `error`, `data` — each `@quent/client` hook must pass these through, not hide them | LOW | Pattern already exists; make it explicit in the hook return type |
| Stale-time defaults configurable at hook call site | Consumers with different freshness requirements override `DEFAULT_STALE_TIME` per call | LOW | Currently hardcoded at `5 * 60 * 1000`; expose as optional param with sane default |
| JSDoc on every exported function and component | Agents use doc comments to infer intent without reading bodies; missing docs = invisible semantics | MEDIUM | Partial today; every public export needs `@param`, `@returns`, and a purpose sentence |

### Differentiators (Agent-Driven Composition)

Features that are not universally expected but make these packages excellent for the specific goal
of enabling AI agents (and humans) to assemble new UIs without reading implementation internals.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| `className` pass-through on every visual component | Agents can apply layout, spacing, and theme overrides without forking components; follows shadcn convention | LOW | Add `className?: string` to every component's props interface; apply via `cn()` at root element |
| `asChild` / Radix `Slot` pattern on interactive primitives | Callers wrap a component in their own element (e.g. `<Button asChild><Link>`) without DOM nesting issues | LOW | Already in `button.tsx`; extend to other clickable wrappers if any are added |
| CVA `variants` prop exported alongside components | Agents can pass variant names (`variant="outline"`) without knowing CSS; the variant contract is part of the public API | LOW | `buttonVariants` is already exported; do the same for any new CVA components |
| Controlled-first component design | DAGChart, TimelineController accept selection/zoom state as props and emit change callbacks — no hidden internal state that conflicts with external state management | MEDIUM | DAGChart currently writes directly to Jotai atoms; for the package version, consider adding `onSelectionChange` callback prop so components work without `@quent/hooks` |
| Hook + `queryOptions` factory dual export | Route loaders can use `queryOptions` to prefetch; components use the hook — same query key, zero duplication | LOW | Already done for `useQueryBundle`; apply pattern consistently to all queries in `@quent/client` |
| `BigInt`-safe JSON parsing exposed as utility | Consumers of the Quent API deal with 64-bit integers from Rust; without `parseJsonWithBigInt`, they silently corrupt large IDs | LOW | Export from `@quent/utils`; it is domain-critical and not obvious to re-implement |
| Deterministic color utilities exported from `@quent/utils` | Visualization components need stable, accessible colors for operator types; `getColorForKey`, `assignColors`, and the Wong palette let agents build consistent charts without reinventing color logic | LOW | `services/colors.ts` contains a complete, tested system — lift wholesale into `@quent/utils` |
| Formatter utilities exported from `@quent/utils` | Duration, timestamp, and size formatting is shared across DAG, timeline, and tree views; without a shared formatter agents produce inconsistent displays | LOW | `services/formatters.ts` — re-export from `@quent/utils` |
| Skeleton/loading state components in `@quent/components` | Agents composing new views need empty-state and loading-state building blocks alongside data components; without them every new view re-invents loading UI | LOW | `TimelineSkeleton.tsx` already exists; generalize and export a set of skeleton primitives |
| Operation type color mapping centralized and exported | `colors.ts` now has a centralized operation type coloring system (per recent commit); agents using `DAGChart` nodes outside the chart need the same color lookup | LOW | Export `getOperationTypeColor()` or equivalent from `@quent/utils` alongside the palette utilities |

### Anti-Features (Commonly Requested, Often Problematic)

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| Raw atom exports from `@quent/hooks` | Consumers want fine-grained control over Jotai atoms | Breaks the abstraction boundary — swapping Jotai for Zustand or React Context later requires changing every consumer | Export hooks that return the same values; atoms are an implementation detail |
| Global singleton QueryClient inside `@quent/client` | Seems convenient to not pass a client around | Prevents multiple independent QueryClient instances; breaks tests; forces a specific initialization order | Export `queryOptions` factories and let the app own the QueryClient instance |
| Package-level CSS imports that affect global scope | Components that auto-import Tailwind base styles or `@xyflow/react/dist/style.css` at the package level | Pollutes the consumer's CSS scope; forces CSS load order; breaks tree-shaking | Document required peer stylesheets; let the consumer import them at the app level |
| Component-internal fetch calls (bypassing `@quent/client`) | Seems self-contained — component knows what data it needs | Breaks cache sharing; prevents prefetching; agents cannot reason about data dependencies separately from rendering | Components accept data as props; `@quent/client` hooks sit at the call site |
| Versioned package releases inside the monorepo | Keeps packages "publishable" | Creates changelog overhead, version drift, and `workspace:*` confusion before a publish decision is made | Single `workspace:*` lock throughout; version only on publish decision |
| Monolithic `@quent/everything` barrel | One import for everything seems convenient | Breaks tree-shaking; creates implicit coupling between components, hooks, and utils; any change anywhere invalidates caches for everything | Keep the 4-package split; each package is independently importable |
| Theming system with CSS-in-JS or runtime tokens | Flexible theming looks appealing | Adds bundle weight and runtime cost; Tailwind CSS v4 already handles theming via CSS variables; duplicating it creates two theming systems | Use Tailwind CSS v4 design tokens; expose `className` on all components for overrides |
| Abstract state management adapters | Support any state library (Zustand, Redux, etc.) | Adds indirection with no current consumer; the codebase is Jotai throughout | Hide atoms behind hooks; if a consumer needs a different state library they replace the hooks package |

---

## Feature Dependencies

```
@quent/utils (Rust TS bindings, cn, formatters, colors, parseJsonWithBigInt)
    └──required by──> @quent/client  (typed fetch functions, queryOptions factories)
    └──required by──> @quent/hooks   (typed atom values, hook return types)
    └──required by──> @quent/components (type-safe props, cn for className)

@quent/client (queryOptions factories + hooks)
    └──required by──> @quent/hooks   (useBulkTimelines calls fetchBulkTimelines)
    └──optional for──> @quent/components (components accept data as props; hooks are optional)

@quent/hooks (Jotai atoms wrapped in hooks)
    └──enhances──> @quent/components (DAGChart reads selectedNodeIds via hook)
    └──not required by──> @quent/components (controlled-first design decouples them)

queryOptions factory ──enables──> route loader prefetch + hook read-from-cache
    (both use identical query key; consumers get cache hits without extra work)

Controlled-first component design ──conflicts with──> internal Jotai writes
    (DAGChart currently calls useSetAtom directly; extracting the component requires
     either keeping @quent/hooks as a peer dep or adding onSelectionChange callback)
```

### Dependency Notes

- **`@quent/utils` is the foundation:** Every other package imports from it. It must be extracted first. The `~quent/types/*` path alias resolution is the critical prerequisite — until Rust-generated types are properly re-exported from `@quent/utils`, nothing else can be extracted cleanly.

- **`@quent/client` requires `@quent/utils`:** `api.ts` imports all Rust-generated types. The fetch functions are pure (no React, no Jotai) — this package has no React peer dependency, making it independently testable.

- **`@quent/hooks` couples `@quent/client` and Jotai:** `useBulkTimelines` calls `fetchBulkTimelines` from `@quent/client` and reads atoms from `atoms/timeline.ts`. The hook layer is the glue between server state and UI state.

- **`@quent/components` and `@quent/hooks` have a soft coupling:** `DAGChart` directly writes to `selectedNodeIdsAtom`. For package extraction this either means `@quent/hooks` is a peer dep of `@quent/components`, or `DAGChart` gains `onSelectionChange` + `selectedNodeIds` props and becomes fully controlled. The controlled-first approach is preferred (decouples rendering from state library).

- **`queryOptions` factory pattern:** `useQueryBundle` already demonstrates this — `queryBundleQueryOptions` is exported alongside `useQueryBundle`. Route loaders call the factory directly; the hook calls `useQuery(queryBundleQueryOptions(...))`. This pattern must be applied to all queries in `@quent/client`. The factory export is what enables prefetching without duplicating query keys.

---

## MVP Definition

### Launch With (v1 — Packages Usable by the Existing App)

- [ ] `@quent/utils` extracted with `cn`, Rust type re-exports, `parseJsonWithBigInt`, color utilities, formatters — **foundation for all other packages**
- [ ] `@quent/client` extracted with all fetch functions, `queryOptions` factories, `DEFAULT_STALE_TIME` — **app can call hooks without importing from `ui/src/services/api.ts`**
- [ ] `@quent/hooks` extracted with all named hooks wrapping atoms — **no raw atom imports outside this package**
- [ ] `@quent/components` extracted with barrel export of all domain components and UI primitives — **components importable without tracing `ui/src/components/`**
- [ ] Existing `ui/src` app migrated to consume from packages only — **proves packages work end-to-end**
- [ ] Each package has `index.ts` listing every public export — **agent can open one file and see the full API**
- [ ] Every exported function/component has JSDoc — **agents can generate correct call sites from docs alone**

### Add After Validation (v1.x)

- [ ] Controlled-first props on `DAGChart` (`selectedNodeIds`, `onSelectionChange`) — add once component package is stable and coupling is observable
- [ ] `className` pass-through audit and remediation — verify every component accepts and applies it, add where missing
- [ ] `staleTime` override param on all `@quent/client` hooks — low-effort addition once hook API shape is settled
- [ ] Skeleton component generalization — `TimelineSkeleton` abstracted into reusable loading primitives

### Future Consideration (v2+)

- [ ] npm publish preparation (package.json `exports` field, `types` entry, bundler config) — defer until publish decision; design is already publishability-ready
- [ ] Per-package changelogs and independent semver — adds overhead; only worthwhile when external consumers exist
- [ ] Storybook or equivalent component catalog — useful for external consumers; premature for internal-only use

---

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| `@quent/utils` extraction (types, cn, formatters, colors) | HIGH | LOW | P1 |
| `@quent/client` extraction (fetch functions + queryOptions factories) | HIGH | LOW | P1 |
| `@quent/hooks` extraction (hooks over atoms) | HIGH | MEDIUM | P1 |
| `@quent/components` extraction (barrel + all components) | HIGH | MEDIUM | P1 |
| App migration to consume from packages | HIGH | MEDIUM | P1 |
| `index.ts` barrel exports per package | HIGH | LOW | P1 |
| JSDoc on all public exports | HIGH | MEDIUM | P1 |
| `className` pass-through on components | MEDIUM | LOW | P2 |
| Controlled-first props on DAGChart | MEDIUM | MEDIUM | P2 |
| `staleTime` override in client hooks | LOW | LOW | P2 |
| Skeleton component generalization | MEDIUM | LOW | P2 |
| npm publish prep (`exports` field, etc.) | LOW | MEDIUM | P3 |
| Per-package versioning | LOW | HIGH | P3 |

**Priority key:**
- P1: Must have for the packages to replace `ui/src` internal imports
- P2: Should have, add when extraction is stable
- P3: Nice to have, future consideration

---

## Competitor / Reference Pattern Analysis

These are not direct product competitors but reference implementations that inform design.

| Pattern | Reference | How They Do It | Our Approach |
|---------|-----------|----------------|--------------|
| Component API surface | shadcn/ui | Copies source into project; exports `ComponentProps` + `componentVariants` + component | Same: export props interface, CVA variants object, and component from barrel |
| Hooks package | TanStack Query itself | All hooks exported by name from single barrel; `queryOptions` factory pattern for loader compat | Same: named exports only, `queryOptions` factories alongside every `useQuery` hook |
| State hiding | React Query v5 | `QueryClient` is owned by app; library provides factories and hooks, not singletons | Same: no singleton client; `@quent/hooks` owns atoms internally, hooks own the surface |
| Headless / controlled | Radix UI primitives | Components are fully controlled or uncontrolled via standard React patterns; no hidden state | Adopt for visualization components; `DAGChart` gains controlled props |
| Type safety at boundaries | tRPC | Types flow from server to client without duplication; generated types are the contract | Same: `@quent/utils` re-exports Rust-generated types; every package types against them |

---

## Sources

- Direct analysis of `ui/src/` codebase (2026-04-01): atoms, hooks, services/api.ts, components
- `.planning/PROJECT.md`: package split goals, publishability requirement, agent legibility goal
- `.planning/codebase/ARCHITECTURE.md`: data flow, state management patterns
- `.planning/codebase/CONVENTIONS.md`: naming, export, JSDoc conventions
- shadcn/ui design principles (training data, HIGH confidence): copy-owned components, export variants + props
- TanStack Query v5 `queryOptions` factory pattern (training data, HIGH confidence): loader/hook cache sharing
- Radix UI controlled component patterns (training data, HIGH confidence): headless, slot-based composition

---
*Feature research for: @quent package library extraction (components, hooks, client, utils)*
*Researched: 2026-04-01*
