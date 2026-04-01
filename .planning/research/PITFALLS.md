# Pitfalls Research

**Domain:** React monorepo package extraction — pnpm workspace, Vite, React 19, Jotai v2, TanStack Query v5, Tailwind CSS v4
**Researched:** 2026-04-01
**Confidence:** HIGH (codebase-grounded; all pitfalls verified against actual files in `ui/src/`)

---

## Critical Pitfalls

### Pitfall 1: Duplicate React Instance (Invalid Hook Calls)

**What goes wrong:**
Components imported from `@quent/components` throw "Invalid hook call" or "Cannot read properties of null (reading 'useRef')" at runtime. React's hooks system relies on a single module-level instance. When the app bundle and a package bundle each resolve their own copy of `react`, hooks break silently or catastrophically.

**Why it happens:**
pnpm uses symlinks, but if `react` is listed as a regular `dependency` (not `peerDependency`) in a package's `package.json`, pnpm resolves a second copy inside the package's own `node_modules`. Both the app and the package then import from different physical paths, producing two React instances.

**How to avoid:**
In every `@quent/*` package `package.json`:
- List `react` and `react-dom` only under `peerDependencies`, never `dependencies`
- Set the peer range to match the app: `"react": "^19.0.0"`
- Keep them in `devDependencies` for local test/build only

```json
{
  "peerDependencies": {
    "react": "^19.0.0",
    "react-dom": "^19.0.0"
  },
  "devDependencies": {
    "react": "^19.2.4",
    "react-dom": "^19.2.4"
  }
}
```

In `ui/vite.config.ts`, add a `resolve.dedupe` entry so Vite always uses one copy even if resolution goes wrong:

```ts
resolve: {
  dedupe: ['react', 'react-dom', 'jotai', '@tanstack/react-query'],
  alias: { /* existing */ }
}
```

**Warning signs:**
- "Invalid hook call" in browser console after adding a package import
- React DevTools shows two React roots or two store trees
- `npm ls react` (or `pnpm why react`) lists the package under more than one path

**Phase to address:** Package scaffold phase (before any code moves). Setting `peerDependencies` correctly from the start prevents cascading failures in every subsequent extraction step.

---

### Pitfall 2: Jotai Atom Identity — Multiple Stores from Module Duplication

**What goes wrong:**
Atoms exported from `@quent/hooks` are created at module load time (`atom(...)` is called once per module evaluation). If jotai is resolved twice (same root cause as React duplication), atom identity is broken: the app's Jotai `Provider` does not hold the atoms the package is reading, so `useAtom(selectedNodeIdsAtom)` always returns the initial value regardless of what the app writes.

This is harder to diagnose than the React duplicate because there is no thrown error — components just silently read stale values.

**Why it happens:**
Same mechanism as Pitfall 1 but for `jotai`. Additionally: if atoms are defined in `@quent/hooks` but the app also imports the raw atom file during transition (before extraction is complete), two atom instances with the same semantic meaning exist in the global default store, causing split state.

The existing code already uses `<Provider key={queryId}>` in `profile.engine.$engineId.tsx` to scope atoms per query. This intentional scoping must be preserved — if atoms move into a package, the Provider must wrap the same subtree, using the same imported `Provider` from the same jotai instance.

**How to avoid:**
1. Add `jotai` to `peerDependencies` (not `dependencies`) in every `@quent/*` package that reads or writes atoms.
2. Add `jotai` to `resolve.dedupe` in `vite.config.ts`.
3. During extraction: never have two import paths for the same atom (old `@/atoms/dag` and new `@quent/hooks`) active simultaneously. Use a re-export bridge: keep the old path as a thin re-export of the new package until the migration is complete, then delete it.

```ts
// ui/src/atoms/dag.ts (bridge, deleted after migration)
export * from '@quent/hooks/dag';
```

4. The `<Provider key={queryId}>` pattern in the route must import `Provider` from the same jotai resolution. Since the app controls that import, this is fine — but do not re-export `Provider` from `@quent/hooks`.

**Warning signs:**
- `useAtom(selectedNodeIdsAtom)` returns initial value while another component that writes it reports writing correctly
- Atom updates in DevTools show two separate atom graphs
- `pnpm why jotai` lists jotai under `@quent/hooks` node_modules path

**Phase to address:** `@quent/hooks` extraction phase. Verify with a targeted test: write to an atom from the app, read it from a component that imports via the new package path — they must agree.

---

### Pitfall 3: Tailwind CSS v4 — Package Classes Purged (Not Scanned)

**What goes wrong:**
Components in `@quent/components` use Tailwind utility classes. In production build, those classes are absent from the generated CSS because Tailwind's content scanner only knows about `ui/src/**`. The app renders components with no styling. This does not fail locally if the dev server's JIT scanner happens to pick up the package through an import chain, making it a "works locally, broken in prod" failure.

**Why it happens:**
Tailwind CSS v4 auto-detects content from the entry point and its imports, but only follows imports that Vite processes. Packages installed as workspace dependencies are linked via `node_modules`. Tailwind's scanner does not walk into `node_modules` by default. The `@quent/components` source files never get scanned.

The current setup uses `@tailwindcss/vite` plugin with `@import 'tailwindcss'` in `index.css` — Tailwind v4 automatic content detection. This is correct for a single-package setup but insufficient for cross-package class usage.

**How to avoid:**
**Option A (recommended for internal packages):** Do not ship Tailwind class strings in packages. Instead, ship components that accept `className` props and document the required CSS variables. The app's `index.css` keeps all styling. This is the shadcn model — components are unstyled tokens that the app themes. This also solves the publishability problem: published packages cannot depend on the consumer having Tailwind installed.

**Option B (if Option A is not feasible):** Add an explicit content source in the app's Tailwind config pointing at the package source directories. With Tailwind v4's `@source` directive:

```css
/* ui/src/index.css */
@import 'tailwindcss';
@source '../../packages/components/src/**/*.{ts,tsx}';
@source '../../packages/hooks/src/**/*.{ts,tsx}';
```

This requires the packages to ship source files (not compiled), which is correct for `workspace:*` internal packages but incompatible with npm publishing without an additional build step.

**Option C (for publishable packages):** Pre-generate a CSS file per package and have consumers import it alongside the JS. This is the standard pattern for component libraries (e.g., `import '@quent/components/dist/style.css'`). Requires a build step with `postcss` + Tailwind in each package.

For this project (internal-first, eventual publish), use **Option A** as the default and document **Option C** as the path for when npm publishing happens.

**Warning signs:**
- Styled components look correct in `vite dev` but lose styling after `vite build`
- `grep -r "bg-" packages/components/src` finds class strings that are not in the app's own source

**Phase to address:** `@quent/components` scaffold phase. Establish the CSS strategy before moving a single component.

---

### Pitfall 4: `~quent/types` Alias Breaks Inside Packages

**What goes wrong:**
The `~quent/types` path alias is defined in `ui/vite.config.ts` and `ui/tsconfig.json`. When code that uses this alias is moved into `ui/packages/utils/src/`, that code is no longer processed by the app's Vite config during standalone package builds. TypeScript in the package also cannot resolve the alias without the alias being re-declared in the package's own `tsconfig.json`.

In dev (with `workspace:*` and no separate package build step), this may appear to work because Vite processes everything through the app's config. But package-level `tsc --noEmit` checks and any future package build will fail.

**Why it happens:**
Path aliases are Vite/TypeScript build tool artifacts, not Node module resolution. They do not cross package boundaries automatically. Packages have their own compilation context.

**How to avoid:**
The types from `~quent/types` (`crates/server/ts-bindings/` and `examples/simulator/server/ts-bindings/`) belong in `@quent/utils`. The resolution strategy:

1. Move the type-binding source path resolution into `@quent/utils/package.json` using the `exports` field:
```json
{
  "exports": {
    "./types/*": "./src/types/*"
  }
}
```
2. Copy (or symlink during dev) the generated `.ts` files from `../../../examples/simulator/server/ts-bindings/` into `packages/utils/src/types/` as part of a code-gen step, or re-export them.
3. Update consuming packages to import from `@quent/utils/types/...` instead of `~quent/types/...`.
4. Remove the `~quent/types` alias from `vite.config.ts` once all consumers are migrated.

For the package's own `tsconfig.json`, use a relative path to the bindings:
```json
{
  "compilerOptions": {
    "paths": {
      "~quent/types/*": ["../../../examples/simulator/server/ts-bindings/*"]
    }
  }
}
```

**Warning signs:**
- TypeScript errors in package source files: "Cannot find module '~quent/types/...'"
- Build passes in the app but `tsc --noEmit` inside the package directory fails
- CI fails on `typecheck` despite dev working fine

**Phase to address:** `@quent/utils` extraction phase (first package built, since all others depend on it).

---

### Pitfall 5: Global Mutable Module State Escapes Package Boundary

**What goes wrong:**
`ui/src/services/colors.ts` uses module-level mutable variables (`colorAssignments`, `usedIndices`). When this is moved to `@quent/utils`, the module is evaluated once per JavaScript module graph entry. In a standard single-app setup this is one evaluation. But if the package is imported by multiple independent entry points (e.g., in tests, or if two apps ever consume it), the color state resets between modules or diverges.

More immediately: the module-global state means colors are non-deterministic across re-renders and React StrictMode double-invocations, causing color flicker in development. This existing bug becomes more visible after extraction because tests will call the module from a fresh context each time.

**Why it happens:**
Module-level `let` or `const` holding mutable objects are effectively global singletons within a module graph. React's programming model assumes side-effect-free renders; module state violates this.

**How to avoid:**
Before extracting `colors.ts` to `@quent/utils`, move the mutable state into a Jotai atom (or React ref) so it lives in the React tree, not the module. The `@quent/utils` package should export pure functions only — no module-level mutation.

```ts
// @quent/utils — pure, no module state
export function assignColor(id: string, assignments: Map<string, string>): string { ... }

// @quent/hooks — stateful, uses Jotai
export const colorAssignmentsAtom = atom(new Map<string, string>());
```

**Warning signs:**
- Color assignments change between page reloads with the same data
- React StrictMode causes double color assignment on first render
- Tests that import colors independently get different results from the app

**Phase to address:** `@quent/utils` extraction phase. Audit every file being moved for module-level mutable state before moving it.

---

### Pitfall 6: TanStack Query Cache — Accidental Cache Sharing or Isolation

**What goes wrong:**
Two scenarios in tension:

**Scenario A (too isolated):** `@quent/client` creates its own `QueryClient` instance internally and exports it. The app also has a `QueryClient`. Components from `@quent/client` use the package's internal client; the app uses its own. The same API data is fetched twice, caches do not share, devtools shows inconsistency.

**Scenario B (too shared):** `@quent/client` exports query key factories and `useQuery` hooks that assume they will run under the app's `QueryClientProvider`. If the package is tested in isolation or used in a Storybook without a `QueryClientProvider`, all hooks throw "No QueryClient set, use QueryClientProvider to set one."

**Why it happens:**
TanStack Query's `QueryClient` is provided via React context. Packages that call `useQuery` must run inside a `QueryClientProvider`. The correct pattern is: the app owns the `QueryClientProvider`; packages export hooks that call `useQuery` and must be documented as requiring the provider.

**How to avoid:**
- Never create a `QueryClient` inside `@quent/client`. Export query hooks only.
- Add `@tanstack/react-query` to `peerDependencies` in `@quent/client` and `@quent/hooks`.
- Add it to `resolve.dedupe` in `vite.config.ts`.
- Document in `@quent/client/README` (or `index.ts` JSDoc): "Requires `<QueryClientProvider>` in the tree."
- For testing `@quent/client` hooks in isolation, use a test wrapper that provides a fresh `QueryClient` — follow the existing pattern in `ui/src/test/test-utils.tsx`.

**Warning signs:**
- "No QueryClient set" error in tests for package hooks
- Two separate QueryClient instances visible in React DevTools
- `pnpm why @tanstack/react-query` lists it under package node_modules

**Phase to address:** `@quent/client` extraction phase.

---

### Pitfall 7: Vite `optimizeDeps` Does Not Pre-Bundle Workspace Packages by Default

**What goes wrong:**
After linking `@quent/*` packages via `workspace:*`, Vite may not pre-bundle them during `vite dev`. This causes two problems:
1. **HMR breaks** — changes in `packages/components/src/` do not trigger hot reload in the app because Vite's watcher does not follow symlinks into `node_modules` by default.
2. **Slow first load** — each package file is served as individual ES module requests without pre-bundling, causing hundreds of network requests on first page load in dev.

**Why it happens:**
By default Vite excludes anything in `node_modules` from the dep optimizer (which pre-bundles CJS/ESM into optimized ES modules). Workspace packages linked via symlinks appear in `node_modules`, so they get excluded. However, since these packages are source-only (no `dist/` build), Vite also can't process them as standard npm packages.

**How to avoid:**
In `ui/vite.config.ts`, add the workspace packages explicitly:

```ts
optimizeDeps: {
  include: [
    '@quent/components',
    '@quent/hooks',
    '@quent/client',
    '@quent/utils',
  ],
},
```

And enable symlink following for the watcher:

```ts
server: {
  watch: {
    // Follow symlinks so changes in packages/ trigger HMR
    followSymlinks: true,
  },
  // ... existing proxy config
},
```

For HMR across boundaries, ensure packages have no `main`/`module` field pointing to a `dist/` directory until a dist build is added — Vite should resolve to the `src/` entry point via the `exports` field:

```json
{
  "exports": {
    ".": "./src/index.ts"
  }
}
```

**Warning signs:**
- Editing a component in `packages/components/src/` does not trigger HMR in the browser
- Dev server first load is very slow (hundreds of individual module requests)
- Vite console shows "new dependencies found" every restart

**Phase to address:** Package scaffold phase. Configure before any code moves.

---

### Pitfall 8: TypeScript Project References — `noEmit` Conflicts with Package Builds

**What goes wrong:**
The app's `tsconfig.json` uses `"noEmit": true` and `"moduleResolution": "bundler"` — correct for a Vite app. If the same `tsconfig.json` is reused or referenced from packages, packages cannot emit `.d.ts` files for consumers (required for publishability). `moduleResolution: "bundler"` is also not appropriate for packages that may be consumed by non-Vite toolchains.

**Why it happens:**
App tsconfig options are optimized for Vite-bundled apps. Package tsconfig options need to target compilation output for type generation.

**How to avoid:**
Each `@quent/*` package needs its own `tsconfig.json` that:
- Sets `"moduleResolution": "node16"` or `"nodenext"` (correct for packages with `exports` map)
- Sets `"declaration": true`, `"declarationDir": "./dist/types"` (for publishability)
- Sets `"outDir": "./dist"` and removes `"noEmit": true`
- Does NOT use `"allowImportingTsExtensions": true` (that requires `noEmit: true`)

During the `workspace:*` phase, packages do not need to build — Vite resolves source directly. But the tsconfig must still be correct so `tsc --noEmit` type-checks correctly. Use a separate `tsconfig.build.json` per package for when publish-ready compilation is needed:

```json
// packages/components/tsconfig.json (for checking and IDE)
{
  "extends": "../../tsconfig.base.json",
  "compilerOptions": {
    "noEmit": true,
    "moduleResolution": "bundler"
  }
}

// packages/components/tsconfig.build.json (for npm publish)
{
  "extends": "../../tsconfig.base.json",
  "compilerOptions": {
    "declaration": true,
    "outDir": "./dist",
    "moduleResolution": "node16"
  }
}
```

**Warning signs:**
- `tsc --build` in a package produces no `.d.ts` files
- Consumers of a published package see "has no exported member" despite the source having exports
- IDE shows type errors in packages that `vite dev` ignores

**Phase to address:** Package scaffold phase. Establish the tsconfig pattern before any code moves.

---

### Pitfall 9: `atomFamily` Leak When Atoms Are Scoped Per-Query

**What goes wrong:**
`timeline.ts` uses `atomFamily` from `jotai-family` to create per-resource atoms. When these atoms are moved to `@quent/hooks`, the `atomFamily` map is global to the module. The existing `<Provider key={queryId}>` pattern in the route creates a new Jotai store on each navigation (the `key` forces remount). However, `atomFamily` is not store-scoped — it is module-scoped. Changing queries does NOT clear the `atomFamily` cache; old atom instances accumulate for the lifetime of the page.

This is a memory leak that already exists in the current code but becomes more visible and harder to fix after extraction if the atomFamily is buried inside a package.

**Why it happens:**
`atomFamily` in `jotai-family` v1 stores instances in a `WeakMap` or `Map` keyed by parameter. The map lives at module scope, not store scope. The Jotai `Provider` with a `key` creates a new store (so atom values reset), but the atomFamily map retains all the atom instances.

**How to avoid:**
When moving `timelineDataAtom` and `isTimelineHoveredAtom` to `@quent/hooks`:

1. Export a `clearTimelineAtomFamily(params)` function alongside the atom family so callers can explicitly clean up.
2. Or switch from `atomFamily` to storing the data in a single record atom keyed by string: `atom<Record<string, SingleTimelineResponse>>({})`. The entire record resets when the Provider remounts on query change.

Option 2 is simpler and eliminates the leak entirely:

```ts
// Instead of atomFamily
export const timelineDataAtom = atom<Record<string, SingleTimelineResponse | undefined>>({});
```

**Warning signs:**
- Memory usage grows with each query navigation and never releases
- `console.log(atomFamily.getParams())` (if available) shows stale resource IDs from previous queries

**Phase to address:** `@quent/hooks` extraction phase. Address the leak before moving the atom family.

---

## Technical Debt Patterns

Shortcuts that seem reasonable but create long-term problems.

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Re-exporting from old paths (`@/atoms/dag` re-exports `@quent/hooks/dag`) | Zero breakage during migration | Old imports persist forever; deletion never happens | Only if there is a committed deletion date per phase |
| Listing dependencies instead of peerDependencies in packages | Simpler `package.json` | Duplicate instances, broken hooks, cache isolation | Never for react, jotai, @tanstack/react-query |
| Single `tsconfig.json` shared between app and packages | Less config to maintain | `noEmit`/`allowImportingTsExtensions` conflicts with package builds | Never — packages need distinct tsconfig |
| No package `exports` field (rely on directory resolution) | Simpler package.json | Private internals are importable; publishability is broken | Only in MVP if exports map is added before publish |
| Keeping Tailwind classes in package component source | Easiest component migration | Classes get purged in prod; publishability requires CSS pipeline | Never in production builds |
| Skipping `resolve.dedupe` in Vite config | One less config line | Silent duplicate instance bugs that appear only after adding a new consumer | Never for react/jotai/tanstack |

## Integration Gotchas

Common mistakes when connecting the package layer to the build system.

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| pnpm workspace | Using `file:` protocol instead of `workspace:*` | `workspace:*` resolves to the workspace package correctly; `file:` does not update on version bumps and has different symlink behavior |
| Vite + workspace packages | Relying on automatic optimizeDeps detection | Explicitly list all `@quent/*` packages in `optimizeDeps.include` |
| TypeScript paths + packages | Copying `@/*` alias into package tsconfig | Package tsconfig must NOT inherit app path aliases; only package-relative paths |
| `~quent/types` alias | Moving files that use it without updating the alias | Migrate to `@quent/utils` imports first, remove alias last |
| Tailwind v4 `@source` | Using glob patterns relative to CSS file | Paths in `@source` are relative to the CSS file location — verify they resolve from `ui/src/index.css` |
| pnpm hoisting | Assuming packages get hoisted to root `node_modules` | pnpm uses a virtual store; never rely on implicit hoisting — always declare deps explicitly |

## Performance Traps

Patterns that work at small scale but cause problems.

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| Barrel file with all package exports | Fast initial development | Vite tree-shakes but TypeScript checker processes entire barrel on every save; slow HMR at 50+ exports | At ~30+ re-exported symbols |
| No `sideEffects: false` in package.json | Works in dev | Tree-shaking disabled for entire package in production; bundle size bloat | Production builds |
| `atomFamily` with unbounded growth | Correct behavior per query | Memory leak accumulates across query navigations | After ~20 query navigations without page reload |
| Deep component import chains through barrel | Convenient imports | Single changed component in a package triggers full barrel re-evaluation | At ~15+ components in one package |

## "Looks Done But Isn't" Checklist

Things that appear complete but are missing critical pieces.

- [ ] **Package extraction complete:** Verify `pnpm why react` shows only one React instance — not that it "seems to work"
- [ ] **Tailwind classes visible in prod:** Run `vite build` and `vite preview`, not just `vite dev` — purging only happens in prod builds
- [ ] **Jotai atom scoping preserved:** Navigate between two different queries and confirm atom state resets correctly — not just that one query works
- [ ] **TypeScript clean without app config:** Run `tsc --noEmit` from inside each package directory, not just from `ui/` root
- [ ] **HMR works across boundary:** Edit a file in `packages/components/src/`, confirm browser updates without full reload
- [ ] **No raw atom imports in app src:** After `@quent/hooks` extraction, grep `ui/src` for `import.*from.*atoms/` — should be zero (or bridge re-exports only)
- [ ] **`~quent/types` alias removed:** After `@quent/utils` extraction, grep `vite.config.ts` and `tsconfig.json` for `~quent/types` — should be gone
- [ ] **`sideEffects: false` set:** Every package's `package.json` has `"sideEffects": false` (or specific CSS file list)

## Recovery Strategies

When pitfalls occur despite prevention, how to recover.

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| Duplicate React instance | LOW | Add `resolve.dedupe` + change dep to peerDep in package; rebuild |
| Tailwind purging in prod | LOW | Add `@source` directive in `index.css` pointing at package src paths; rebuild |
| Jotai atom identity broken | MEDIUM | Add `jotai` to `resolve.dedupe`; verify with React DevTools; grep for double imports |
| `~quent/types` alias broken in package | LOW | Add path alias to package tsconfig or migrate to `@quent/utils` imports |
| HMR not updating across boundary | LOW | Add `server.watch.followSymlinks: true` + `optimizeDeps.include` in vite config |
| `atomFamily` memory leak | MEDIUM | Migrate from `atomFamily` to record atom; requires touching all call sites |
| Module-global color state non-determinism | MEDIUM | Move state to Jotai atom before extraction; audit all module-level `let`/`const` holding objects |
| tsconfig conflict between app and package | LOW | Add separate `tsconfig.json` per package; do not reference app tsconfig |

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| Duplicate React instance | Package scaffold (before any extraction) | `pnpm why react` shows single path; `resolve.dedupe` in vite config |
| Jotai atom identity | Package scaffold + `@quent/hooks` phase | Write atom in app, read via package import — values match |
| Tailwind CSS purging | `@quent/components` scaffold | `vite build` + `vite preview` — styles visible in prod |
| `~quent/types` alias | `@quent/utils` phase (first package) | `tsc --noEmit` from inside `packages/utils/` passes |
| TanStack Query cache isolation | `@quent/client` phase | No `QueryClient` created in package; `pnpm why @tanstack/react-query` single path |
| Vite HMR across boundaries | Package scaffold | Edit package source → browser updates without full reload |
| tsconfig `noEmit` conflict | Package scaffold | `tsc` from package directory produces no errors |
| `atomFamily` leak | `@quent/hooks` phase | Memory profiler shows GC after query navigation |
| Module-global mutable state | `@quent/utils` phase | Audit `colors.ts` before extraction; no module-level `let` |

## Sources

- Codebase analysis: `ui/vite.config.ts`, `ui/src/atoms/dag.ts`, `ui/src/atoms/timeline.ts`, `ui/src/routes/profile.engine.$engineId.tsx`, `ui/src/contexts/ThemeContext.tsx`, `ui/src/services/colors.ts`
- Known issues: `.planning/codebase/CONCERNS.md` — Global Color Assignment State, Jotai Atom Lifecycle Management
- Project constraints: `.planning/PROJECT.md` — publishability-ready design, `workspace:*` protocol, pnpm workspace
- React docs: Invalid hook call causes (multiple React copies) — official React hooks troubleshooting
- Jotai v2 docs: `Provider`, `createStore`, `atomFamily` scoping behavior
- Tailwind CSS v4 docs: `@source` directive for explicit content sources, `@tailwindcss/vite` plugin behavior
- Vite docs: `optimizeDeps.include` for workspace packages, `resolve.dedupe`, symlink handling

---
*Pitfalls research for: React 19 + Jotai + TanStack Query monorepo package extraction*
*Researched: 2026-04-01*
