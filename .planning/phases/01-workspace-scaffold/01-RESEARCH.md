# Phase 1: Workspace Scaffold - Research

**Researched:** 2026-04-01
**Domain:** pnpm workspace setup, TypeScript project references, Vite config guards, Tailwind v4 source scanning, Vitest workspace mode
**Confidence:** HIGH (all findings grounded in actual project files + prior project-level research)

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** Packages live at `ui/packages/@quent/*` — namespaced under the `@quent` scope. Each package directory matches its eventual npm name: `ui/packages/@quent/utils`, `ui/packages/@quent/client`, `ui/packages/@quent/hooks`, `ui/packages/@quent/components`.
- **D-02:** `ui/pnpm-workspace.yaml` uses glob `packages/@quent/*` to register all four packages.
- **D-03:** Each package's `package.json` `name` field is `@quent/<name>` (e.g. `"name": "@quent/utils"`).
- **D-04:** During development, Vite resolves `@quent/*` imports directly to TypeScript source via pnpm workspace symlinks. No `dist/` directory is required in dev.
- **D-05:** Each package `package.json` sets `"main": "src/index.ts"` and `"exports": { ".": "./src/index.ts" }` for source-first resolution in dev. These fields will be updated to point to `dist/` only when publishing.
- **D-06:** `tsup` is only needed for production builds, not for the dev loop. The `build` script in each package runs tsup; no `--watch` mode is set up.
- **D-07:** `tsup.config.ts` in each package targets ESM-only output (`format: ['esm']`). No CJS output.
- **D-08:** TypeScript declaration files (`.d.ts`) are emitted alongside ESM output (`dts: true`).
- **D-09:** `tsup` entry point is `src/index.ts`; output directory is `dist/`.
- **D-10:** `vite.config.ts` updated with `resolve.dedupe: ['react', 'react-dom', 'jotai', '@tanstack/react-query', '@tanstack/react-router']` to prevent duplicate module instances once packages are linked.
- **D-11:** Each package that uses React, Jotai, or TanStack dependencies declares them as `peerDependencies` (not `dependencies`) in its `package.json`.
- **D-12:** `ui/src/index.css` extended with `@source "../packages/@quent/**/*.{ts,tsx}"` so Tailwind v4 scans component source in packages during the app build.
- **D-13:** `ui/tsconfig.base.json` created with shared compiler options (strict, target, lib, moduleResolution: bundler, jsx). Each package `tsconfig.json` extends `../../tsconfig.base.json` and adds `composite: true`, `declaration: true`, `noEmit: false`, `outDir: ./dist`.
- **D-14:** The app-level `ui/tsconfig.json` continues to use `noEmit: true` and `allowImportingTsExtensions: true` (incompatible with package emit — packages need their own tsconfig).

### Claude's Discretion

- **Vitest workspace configuration.** User did not specify — use a single `ui/vitest.workspace.ts` that covers the app and all packages. Each package can have its own `vitest.config.ts` that is picked up by the workspace.

### Deferred Ideas (OUT OF SCOPE)

None — discussion stayed within phase scope.
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| INFRA-01 | `ui/pnpm-workspace.yaml` created with `packages/@quent/*` glob; all four packages resolved via `workspace:*` protocol | D-01, D-02, D-03; pnpm workspace glob syntax confirmed |
| INFRA-02 | `ui/tsconfig.base.json` created with shared compiler options; each package `tsconfig.json` extends it with `composite: true`, `declaration: true`, `noEmit: false` | D-13, D-14; existing `tsconfig.node.json` provides the composite pattern; tsconfig.base content defined |
| INFRA-03 | Each package has a `tsup.config.ts` (or equivalent) for publishability-ready builds (esm + `.d.ts` generation) | D-06, D-07, D-08, D-09; tsup config skeleton pattern defined; NOTE: REQUIREMENTS.md says "esm + cjs" but D-07 locks to ESM-only — ESM-only is the locked decision |
| INFRA-04 | `react`, `jotai`, `@tanstack/react-query`, and `@tanstack/react-router` declared as `peerDependencies`; `vite.config.ts` updated with `resolve.dedupe` | D-10, D-11; exact dedupe list specified; existing vite.config.ts reviewed and dedupe is absent today |
| INFRA-05 | Vitest workspace config (`ui/vitest.workspace.ts`) enables per-package test runs from the workspace root | Claude's Discretion; Vitest 4.x workspace mode syntax documented; existing `vitest.config.ts` must remain as-is |
| INFRA-06 | Tailwind CSS content scanning extended via `@source` directive in `ui/src/index.css` to cover `ui/packages/**/*.{ts,tsx}` | D-12; existing `index.css` uses `@import 'tailwindcss'` at line 4 — `@source` directive to be added after import |
</phase_requirements>

---

## Summary

Phase 1 is purely structural — no code moves, no logic changes. It creates the scaffolding that all subsequent extraction phases depend on: a pnpm workspace file, four empty `@quent/*` package skeletons, a shared `tsconfig.base.json`, per-package tsconfigs and tsup configs, Vite config guards against duplicate module instances, a Tailwind `@source` directive for future component scanning, and a Vitest workspace config.

All decisions are locked from the CONTEXT.md discussion. The existing codebase is well-understood from prior project-level research (STACK.md, PITFALLS.md). The main execution risk is the tsup version — npm registry was unreachable during this research session, so the version is sourced from training data only. This must be verified at task execution time.

The existing app must continue to function without regressions throughout this phase. Every change is additive — no existing files are deleted. The only modifications to existing files are: `vite.config.ts` (add `resolve.dedupe` and `optimizeDeps`/`server.watch`), `src/index.css` (add `@source` line), `tsconfig.json` (add `references` entries), and `package.json` (add `@quent/*` as `workspace:*` dependencies).

**Primary recommendation:** Follow the exact locked decisions verbatim. The tsconfig split between app (`noEmit: true`, `allowImportingTsExtensions: true`) and packages (`noEmit: false`, `composite: true`) is the most failure-prone area — do not deviate.

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| pnpm workspaces | 9.15.0 (existing) | Link `@quent/*` packages via symlinks | Already in use; `workspace:*` requires no new tooling |
| TypeScript project references | 5.9.3 (existing) | Cross-package type checking with `composite: true` | Pattern already exists in `tsconfig.node.json`; enables `tsc --build` incremental checks |
| tsup | ^8.x (training data — verify at execution time) | Build skeleton for publishability (`format: ['esm']`, `dts: true`) | esbuild-based; de-facto standard for TS library builds (used by Jotai, TanStack, etc.); no Vite-specific pipeline needed for pure TS packages |
| Vitest workspace mode | 4.0.18 (existing) | Single command runs tests across all packages | Already installed; `vitest.workspace.ts` is native Vitest feature |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `publint` | ^0.2.x | Lint `package.json` exports for correctness | Wave 0 or Wave 1 — add to package scripts so future phases catch publish-readiness issues early |
| `@arethetypeswrong/cli` | ^0.17.x | Validate `exports` map before publishing | Not needed now (packages are private); install when publishing milestone begins |

No new runtime dependencies. All packages reuse `react`, `jotai`, `@tanstack/react-query` etc. from the root via pnpm hoisting.

**Version verification note:** `tsup` version could not be confirmed via npm registry during this research session (network unavailable in sandbox). STACK.md flags this as MEDIUM confidence. The executing agent MUST run `npm view tsup version` or `pnpm dlx tsup --version` before installing to confirm the current stable version.

**Installation (at task execution time):**
```bash
# In each package directory
pnpm add -D tsup typescript

# Optional from ui/ root
pnpm add -D -w publint
```

---

## Architecture Patterns

### Recommended Project Structure

```
ui/
├── pnpm-workspace.yaml          # NEW: packages/@quent/*
├── tsconfig.json                # MODIFIED: add references[] entries
├── tsconfig.base.json           # NEW: shared compiler options
├── tsconfig.node.json           # UNCHANGED
├── vite.config.ts               # MODIFIED: resolve.dedupe, optimizeDeps, server.watch
├── vitest.config.ts             # UNCHANGED
├── vitest.workspace.ts          # NEW: workspace-level test runner
├── src/
│   └── index.css                # MODIFIED: add @source directive
└── packages/
    └── @quent/
        ├── utils/
        │   ├── package.json     # name: @quent/utils, exports: ./src/index.ts
        │   ├── tsconfig.json    # extends ../../tsconfig.base.json
        │   ├── tsup.config.ts   # ESM-only skeleton
        │   └── src/
        │       └── index.ts     # empty
        ├── client/              # same structure
        ├── hooks/               # same structure
        └── components/          # same structure
```

### Pattern 1: pnpm Workspace Declaration

**What:** `ui/pnpm-workspace.yaml` declares which directories are workspace packages.

**When to use:** Required once; enables `workspace:*` protocol in `package.json` dependencies.

```yaml
# ui/pnpm-workspace.yaml
packages:
  - 'packages/@quent/*'
```

Note: CONTEXT.md D-02 specifies glob `packages/@quent/*` (not `packages/*`). This is more specific than the prior STACK.md example, which used `packages/*`. Use the D-02 value.

### Pattern 2: Source-First Package Resolution (Dev Mode)

**What:** Each package exports its TypeScript source directly in dev, avoiding a build step in the dev loop.

**When to use:** This is the locked approach for this project (D-04, D-05). Matches the shadcn/TanStack pattern for monorepo packages.

```json
// ui/packages/@quent/utils/package.json
{
  "name": "@quent/utils",
  "version": "0.0.0",
  "private": true,
  "type": "module",
  "main": "src/index.ts",
  "exports": {
    ".": "./src/index.ts"
  },
  "scripts": {
    "build": "tsup",
    "typecheck": "tsc --noEmit"
  }
}
```

Note: This differs from the STACK.md example that pointed `exports` at `dist/`. D-05 explicitly locks `exports` to `./src/index.ts` during dev. Planner must use the D-05 pattern, not the STACK.md example.

### Pattern 3: tsconfig Split — App vs. Package

**What:** App tsconfig keeps `noEmit: true` + `allowImportingTsExtensions: true`; packages use separate tsconfigs with `composite: true`, `noEmit: false`, `declaration: true`.

**Why:** These options are mutually exclusive. `allowImportingTsExtensions: true` requires `noEmit: true`. Package builds need to emit `.d.ts` files. CONTEXT.md D-14 explicitly locks this separation.

**`ui/tsconfig.base.json` (new):**
```json
{
  "compilerOptions": {
    "target": "ES2020",
    "lib": ["ES2020", "DOM", "DOM.Iterable"],
    "module": "ESNext",
    "moduleResolution": "bundler",
    "jsx": "react-jsx",
    "strict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noFallthroughCasesInSwitch": true,
    "isolatedModules": true,
    "skipLibCheck": true,
    "resolveJsonModule": true
  }
}
```

Note: Do NOT include `noEmit`, `allowImportingTsExtensions`, `composite`, `declaration`, or `outDir` in the base — each consumer sets these differently.

**`ui/packages/@quent/<name>/tsconfig.json` (per-package):**
```json
{
  "extends": "../../tsconfig.base.json",
  "compilerOptions": {
    "composite": true,
    "declaration": true,
    "declarationMap": true,
    "noEmit": false,
    "outDir": "./dist",
    "rootDir": "./src"
  },
  "include": ["src"]
}
```

**`ui/tsconfig.json` additions (references):**
```json
{
  "references": [
    { "path": "./tsconfig.node.json" },
    { "path": "./packages/@quent/utils" },
    { "path": "./packages/@quent/client" },
    { "path": "./packages/@quent/hooks" },
    { "path": "./packages/@quent/components" }
  ]
}
```

The existing `references` array in `ui/tsconfig.json` currently only contains `{ "path": "./tsconfig.node.json" }`. The package references are appended.

### Pattern 4: tsup Config Skeleton (ESM-only, D-07)

```typescript
// ui/packages/@quent/<name>/tsup.config.ts
import { defineConfig } from 'tsup';

export default defineConfig({
  entry: ['src/index.ts'],
  format: ['esm'],
  dts: true,
  outDir: 'dist',
  clean: true,
});
```

Note: D-07 locks to ESM-only. REQUIREMENTS.md INFRA-03 mentions "esm + cjs" but D-07 (locked decision from CONTEXT.md) overrides this. ESM-only is the correct pattern for this phase.

### Pattern 5: Vite Config Guards (Duplicate Instance Prevention)

The existing `vite.config.ts` has no `resolve.dedupe`. This must be added to prevent duplicate module instances when `@quent/*` packages are linked.

```typescript
// Additions to ui/vite.config.ts resolve section
resolve: {
  dedupe: ['react', 'react-dom', 'jotai', '@tanstack/react-query', '@tanstack/react-router'],
  alias: {
    '@': path.resolve(__dirname, './src'),
    '~quent/types': path.resolve(__dirname, '../examples/simulator/server/ts-bindings'),
    elkjs: 'elkjs/lib/elk.bundled.js',
  },
},
```

Additional Vite additions for workspace HMR (Pitfall 7 prevention):
```typescript
optimizeDeps: {
  include: ['@quent/components', '@quent/hooks', '@quent/client', '@quent/utils'],
},
server: {
  watch: {
    followSymlinks: true,
  },
  // ... existing proxy config unchanged
},
```

### Pattern 6: Tailwind v4 Source Scanning

The existing `ui/src/index.css` line 4 is `@import 'tailwindcss';`. The `@source` directive is added immediately after:

```css
@import 'tailwindcss';
@source "../packages/@quent/**/*.{ts,tsx}";
@plugin "tailwindcss-animate";
```

Note: The path `../packages/@quent/**/*.{ts,tsx}` is relative to `ui/src/index.css`. From `ui/src/`, `../packages/` resolves to `ui/packages/`. This is correct.

### Pattern 7: Vitest Workspace Config

The existing `ui/vitest.config.ts` covers `src/**/*.{test,spec}.{ts,tsx}` only. A new `ui/vitest.workspace.ts` wraps it and adds per-package configs:

```typescript
// ui/vitest.workspace.ts
import { defineWorkspace } from 'vitest/config';

export default defineWorkspace([
  // App tests — existing config unchanged
  './vitest.config.ts',
  // Per-package configs (picked up when created in later phases)
  './packages/@quent/utils/vitest.config.ts',
  './packages/@quent/client/vitest.config.ts',
  './packages/@quent/hooks/vitest.config.ts',
  './packages/@quent/components/vitest.config.ts',
]);
```

Important: The workspace config references per-package vitest configs that don't exist yet in Phase 1. The file glob form (`'./packages/@quent/*/vitest.config.ts'`) is safer — Vitest ignores missing files in a glob. Use the glob form to avoid failures when packages have no vitest.config.ts yet:

```typescript
export default defineWorkspace([
  './vitest.config.ts',
  './packages/@quent/*/vitest.config.ts',
]);
```

Verify at execution time: Vitest 4.x workspace mode behavior with missing globs. The project already has Vitest 4.0.18 installed.

### Pattern 8: App package.json — Add Workspace Dependencies

The root `ui/package.json` needs `@quent/*` packages declared so `pnpm install` resolves them:

```json
{
  "dependencies": {
    "@quent/utils": "workspace:*",
    "@quent/client": "workspace:*",
    "@quent/hooks": "workspace:*",
    "@quent/components": "workspace:*"
  }
}
```

These are added to the existing `dependencies` block. Since no code imports them yet (Phase 1 is scaffold only), these declarations establish the resolution without breaking anything.

### Anti-Patterns to Avoid

- **Listing `react` as `dependencies` in packages:** Causes duplicate React instance → invalid hook calls. Must be `peerDependencies` only (D-11).
- **Using `exports` → `./dist/index.js` in dev packages:** D-05 explicitly locks `exports` to `./src/index.ts` for dev. Using `dist/` breaks dev until a build runs.
- **Inheriting app tsconfig options in packages:** The app's `allowImportingTsExtensions: true` requires `noEmit: true`. Package tsconfigs must NOT extend `ui/tsconfig.json` — they extend `ui/tsconfig.base.json` only.
- **Skipping `optimizeDeps.include`:** Without explicit inclusion, Vite may not pre-bundle workspace packages, breaking HMR (Pitfall 7).
- **Using `packages/*` glob in pnpm-workspace.yaml:** D-02 specifies `packages/@quent/*`. The narrower glob is intentional.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Workspace package linking | Manual `file:` references or `npm link` | pnpm `workspace:*` protocol | `workspace:*` rewrites to correct version on publish; `file:` does not follow workspace semantics; already using pnpm 9 |
| TypeScript declaration generation | Manual `.d.ts` authoring | `tsup` with `dts: true` | tsup invokes tsc for declaration emit with correct settings; hand-authoring .d.ts is error-prone |
| Duplicate module detection | Manual import auditing | `pnpm why <package>` + `resolve.dedupe` | Vite's deduplication is reliable; manual detection misses indirect dependencies |
| Build orchestration across packages | Custom shell scripts | pnpm `--filter` flag (e.g., `pnpm -r build`) | pnpm understands workspace topology; shell scripts do not handle dependency ordering |

**Key insight:** This phase is about wiring up existing tooling correctly, not building new tools. The risk is misconfiguration, not missing libraries.

---

## Common Pitfalls

### Pitfall 1: Duplicate Module Instance (React, Jotai, TanStack)

**What goes wrong:** A `@quent/*` package accidentally lists `react` (or `jotai`, `@tanstack/react-query`) as a regular `dependency`. pnpm installs a second copy inside the package's `node_modules`. React hooks throw "Invalid hook call"; Jotai atoms return stale values silently.

**Why it happens:** Forgetting that `workspace:*` packages still get their own `node_modules` if dependencies are not declared as peers.

**How to avoid:** Every package that uses React, Jotai, or TanStack declares them as `peerDependencies` + `devDependencies` only. Also add `resolve.dedupe` to `vite.config.ts` (D-10, D-11). Verify with `pnpm why react` — must show a single path.

**Warning signs:** "Invalid hook call" in browser after linking a package; `pnpm why react` shows multiple install paths.

### Pitfall 2: tsconfig `noEmit` Conflict Between App and Packages

**What goes wrong:** Package tsconfig accidentally includes or inherits `noEmit: true` or `allowImportingTsExtensions: true` from the app config. Package cannot emit `.d.ts` files; `tsc --noEmit` inside the package passes but `tsc` (for build) fails.

**Why it happens:** Extending the wrong base — extending `ui/tsconfig.json` instead of `ui/tsconfig.base.json`.

**How to avoid:** Package tsconfigs extend `../../tsconfig.base.json` (D-13). The base file deliberately omits `noEmit` and `allowImportingTsExtensions`. Packages set `noEmit: false`, `composite: true`, `declaration: true` themselves (D-13).

**Warning signs:** `tsc --noEmit` passes from `ui/` but fails from inside a package directory; IDE shows type errors that dev server ignores.

### Pitfall 3: Tailwind `@source` Path Resolves Incorrectly

**What goes wrong:** The `@source` directive path is evaluated relative to the CSS file (`ui/src/index.css`). An incorrect path (e.g., `../../packages/` instead of `../packages/`) means Tailwind scans nothing — classes from packages are purged in production builds. This is silent in dev.

**Why it happens:** Tailwind v4 `@source` paths are relative to the CSS file, not the project root. Easy to get wrong.

**How to avoid:** The correct path from `ui/src/index.css` to `ui/packages/@quent/` is `../packages/@quent/`. Verify the directive is:
```
@source "../packages/@quent/**/*.{ts,tsx}";
```
Test: After adding the directive, confirm `vite build` completes and a known Tailwind class from a future package component appears in the CSS output.

**Warning signs:** Tailwind classes present in dev, absent after `vite build`.

### Pitfall 4: Vite Cannot Watch Across Symlinks (HMR Broken)

**What goes wrong:** After linking packages, editing a file in `packages/@quent/*/src/` does not trigger HMR in the browser. Vite's file watcher does not follow symlinks into `node_modules` by default.

**Why it happens:** pnpm uses symlinks for workspace packages. Vite's watcher excludes `node_modules` by default.

**How to avoid:** Add `server.watch.followSymlinks: true` and `optimizeDeps.include` (Pattern 5 above). Test: edit the empty `index.ts` in a package, confirm no Vite watcher errors.

**Warning signs:** File changes in packages/ don't trigger page reload during dev.

### Pitfall 5: Vitest Workspace Config Breaks Existing Tests

**What goes wrong:** Adding `vitest.workspace.ts` changes how Vitest discovers tests. If not careful, the existing `vitest.config.ts` setup (MSW server, jsdom environment, path alias `@/`) is lost for app tests.

**Why it happens:** When `vitest.workspace.ts` exists, Vitest uses it as the source of truth for all test projects. It must explicitly include the existing config.

**How to avoid:** The workspace config must reference `./vitest.config.ts` as the first entry so app tests preserve their current config. Use glob `./packages/@quent/*/vitest.config.ts` for package configs (which don't exist yet in Phase 1 — this is expected). Run `pnpm test:run` after creating the workspace file to confirm app tests still pass.

**Warning signs:** App tests disappear from test output; MSW-related test failures.

### Pitfall 6: pnpm Version Mismatch

**What goes wrong:** The project `package.json` declares `"packageManager": "pnpm@9.15.0"` and enforces pnpm via `preinstall` script. The environment has pnpm 10.20.0 installed. Running `pnpm install` with a newer pnpm than declared may warn or behave differently with lockfile format.

**Root cause observed:** Environment has pnpm 10.20.0; project declares pnpm 9.15.0. The lockfile format `9.0` is pnpm 9 format. pnpm 10 can read pnpm 9 lockfiles but may upgrade them on write.

**How to avoid:** Use `volta run pnpm@9.15.0` or `corepack enable && corepack use pnpm@9.15.0` if pnpm 10 behaves unexpectedly. The planner should note this discrepancy and test `pnpm install` after workspace setup.

---

## Code Examples

Verified patterns from project files and prior project-level research:

### Empty Package `index.ts` (INFRA-01 success criterion)
```typescript
// ui/packages/@quent/utils/src/index.ts
// Scaffold — exports added in Phase 2
export {};
```

### Per-Package `package.json` (source-first, D-05)
```json
{
  "name": "@quent/utils",
  "version": "0.0.0",
  "private": true,
  "type": "module",
  "main": "src/index.ts",
  "exports": {
    ".": "./src/index.ts"
  },
  "sideEffects": false,
  "scripts": {
    "build": "tsup",
    "typecheck": "tsc --noEmit"
  },
  "peerDependencies": {
    "react": "^19.0.0",
    "react-dom": "^19.0.0"
  },
  "devDependencies": {
    "tsup": "^8.x",
    "typescript": "^5.9.3"
  }
}
```

Note: `@quent/utils` and `@quent/client` may not need `react`/`react-dom` as peers depending on what they export. `@quent/client` needs `react` and `@tanstack/react-query`; `@quent/utils` may need neither. For Phase 1 scaffold, include only the peers that logically belong:
- `@quent/utils`: no peers (pure utilities, no framework deps)
- `@quent/client`: `react`, `@tanstack/react-query`, `@tanstack/react-router`
- `@quent/hooks`: `react`, `jotai`
- `@quent/components`: `react`, `react-dom`

### `tsup.config.ts` Skeleton (D-07 ESM-only)
```typescript
// ui/packages/@quent/utils/tsup.config.ts
import { defineConfig } from 'tsup';

export default defineConfig({
  entry: ['src/index.ts'],
  format: ['esm'],
  dts: true,
  outDir: 'dist',
  clean: true,
  sourcemap: true,
});
```

### `pnpm-workspace.yaml` (D-02)
```yaml
packages:
  - 'packages/@quent/*'
```

### Tailwind `@source` directive placement (D-12)
```css
/* ui/src/index.css — existing line 4 then add line 5 */
@import 'tailwindcss';
@source "../packages/@quent/**/*.{ts,tsx}";
@plugin "tailwindcss-animate";
```

---

## State of the Art

| Old Approach | Current Approach | Notes |
|--------------|------------------|-------|
| `exports` pointing to `dist/` in dev | `exports` pointing to `src/index.ts` in dev | D-05 locks source-first; no build required in dev loop |
| Separate Tailwind `content` array in `tailwind.config.js` | `@source` directive in CSS with Tailwind v4 | Tailwind v4 uses `@source` in CSS; no `tailwind.config.js` exists in this project |
| CJS + ESM dual output from tsup | ESM-only (D-07) | Vite/modern Node both handle ESM; CJS output deferred until publishing if ever needed |
| Vitest per-project config only | `vitest.workspace.ts` at root | Enables `pnpm test` from `ui/` to run all packages |

---

## Open Questions

1. **Vitest workspace glob behavior with missing package configs**
   - What we know: `vitest.workspace.ts` with a glob like `'./packages/@quent/*/vitest.config.ts'` should not fail if no matching files exist yet
   - What's unclear: Whether Vitest 4.x throws or warns when a glob matches nothing
   - Recommendation: Test by running `pnpm test:run` after creating `vitest.workspace.ts` with the glob. If it errors, list packages explicitly and add configs as stubs in Phase 1.

2. **tsup current version**
   - What we know: Training data says `^8.x`; npm registry was unreachable during research
   - What's unclear: Whether tsup has released a major version since August 2025 (tsup v9+ may exist)
   - Recommendation: At task execution time, run `npm view tsup dist-tags.latest` before installing. If a v9+ exists, verify the `defineConfig` API is compatible before using the config skeleton above.

3. **pnpm 10 vs pnpm 9 lockfile compatibility**
   - What we know: Environment has pnpm 10.20.0; project targets pnpm 9.15.0; lockfile is `9.0` format
   - What's unclear: Whether `pnpm install` with v10 will silently upgrade the lockfile format, breaking CI that uses pnpm 9
   - Recommendation: After running `pnpm install` in the executing environment, check if `pnpm-lock.yaml` version header changed. If it changed from `9.0`, revert or ensure CI is also on pnpm 10.

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| pnpm | Workspace install, all tasks | Yes | 10.20.0 (project targets 9.15.0) | See Open Question 3 |
| Node.js | All JS tasks | Yes | v24.11.0 | — |
| TypeScript (tsc) | tsconfig verification (`tsc --noEmit`) | Yes (via pnpm devDeps) | 5.9.3 | — |
| tsup | Package build skeletons | Not installed yet | Install via `pnpm add -D tsup` | — |
| Vite | Dev server and build verification | Yes (existing devDep) | 7.3.1 | — |
| Vitest | Test runner | Yes (existing devDep) | 4.0.18 | — |

**Missing dependencies with no fallback:** None — all required tools are available or trivially installable.

**Missing dependencies with fallback:** tsup is not yet installed; installed as part of Phase 1 package skeleton setup.

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Vitest 4.0.18 |
| Config file | `ui/vitest.config.ts` (existing); `ui/vitest.workspace.ts` (to be created in INFRA-05) |
| Quick run command | `pnpm test:run` (from `ui/`) |
| Full suite command | `pnpm test:run` (from `ui/`) |
| Type check command | `pnpm typecheck` (runs `tsc --noEmit` from `ui/`) |

### Phase Requirements to Test Map

| Req ID | Behavior | Test Type | Automated Command | Notes |
|--------|----------|-----------|-------------------|-------|
| INFRA-01 | `pnpm install` resolves `@quent/*` via `workspace:*` | smoke | `pnpm install && pnpm why @quent/utils` | Manual verification step; no unit test needed |
| INFRA-02 | `tsc --noEmit` passes from `ui/` and inside each package | type-check | `pnpm typecheck` from `ui/`; `tsc --noEmit` from each package dir | Automated; existing `typecheck` script |
| INFRA-03 | Each package has `tsup.config.ts`; `tsup` builds without error | smoke | `pnpm --filter @quent/utils build` (per package) | Verifies tsup config is valid |
| INFRA-04 | `pnpm dev` starts; `pnpm why react` shows single instance | smoke | `pnpm why react` from `ui/`; manual dev server start | Semi-automated; requires reading output |
| INFRA-05 | `pnpm test:run` via workspace config runs app tests without regression | automated | `pnpm test:run` from `ui/` | Must run after creating `vitest.workspace.ts` |
| INFRA-06 | Tailwind `@source` directive present in `index.css` | automated | `grep "@source" ui/src/index.css` | Also: `pnpm build` completes without CSS errors |

### Success Criteria Mapping (from phase definition)

1. `pnpm install` from `ui/` resolves all four `@quent/*` packages via `workspace:*` → INFRA-01 smoke test
2. `tsc --noEmit` passes from `ui/` root and from inside each package directory → INFRA-02 typecheck
3. `pnpm dev` starts without errors, app renders → INFRA-04 smoke test
4. `pnpm why react` shows single hoisted React instance → INFRA-04 verification
5. Each package has empty `index.ts` and `tsup.config.ts` skeleton → INFRA-03 + INFRA-01 structural check

### Sampling Rate

- **Per task commit:** `pnpm typecheck` (ensures tsconfig changes don't break type checking)
- **Per wave merge:** `pnpm test:run` (ensures Vitest workspace config doesn't break existing tests)
- **Phase gate:** All 5 success criteria green before `/gsd:verify-work`

### Wave 0 Gaps

- [ ] `ui/vitest.workspace.ts` — to be created as part of INFRA-05; no existing test covers this; verify by running `pnpm test:run` after creation
- [ ] Per-package `vitest.config.ts` stubs — Phase 1 may need minimal stubs if workspace glob requires at least one match

---

## Project Constraints (from CLAUDE.md)

CLAUDE.md does not exist in this project. No additional project-specific directives to enforce.

---

## Sources

### Primary (HIGH confidence)
- `ui/package.json` — confirmed exact versions: React 19.2.4, TypeScript 5.9.3, Vite 7.3.1, Vitest 4.0.18, pnpm 9.15.0 target, lockfile 9.0
- `ui/vite.config.ts` — confirmed no `resolve.dedupe` exists today; confirmed `resolve.alias` entries that must be preserved
- `ui/tsconfig.json` — confirmed `noEmit: true`, `allowImportingTsExtensions: true`, existing `references: [{path: ./tsconfig.node.json}]`
- `ui/tsconfig.node.json` — confirmed `composite: true` pattern to replicate in package tsconfigs
- `ui/vitest.config.ts` — confirmed `include: ['src/**/*.{test,spec}.{ts,tsx}']`; must be preserved unchanged
- `ui/src/index.css` — confirmed `@import 'tailwindcss'` at line 4; `@source` to be inserted at line 5
- `.planning/phases/01-workspace-scaffold/01-CONTEXT.md` — all locked decisions (D-01 through D-14)
- `.planning/research/STACK.md` — tsup patterns, tsconfig strategy, workspace patterns (MEDIUM: version unverified)
- `.planning/research/PITFALLS.md` — duplicate instance risks, HMR pitfall, Tailwind purge fix, tsconfig conflicts (HIGH: grounded in codebase)

### Secondary (MEDIUM confidence)
- Training data (August 2025 cutoff) — tsup `^8.x` API, `defineConfig`, `dts: true` option — verify tsup version before installing
- Training data — Vitest 4.x `defineWorkspace` API for `vitest.workspace.ts`

### Tertiary (LOW confidence)
- Training data — pnpm 10 lockfile format compatibility with pnpm 9 lockfiles — verify at execution time

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all versions confirmed from actual `package.json` and lockfile; only tsup version is MEDIUM (unverifiable without npm access)
- Architecture: HIGH — patterns derived directly from locked CONTEXT.md decisions and existing project file contents
- Pitfalls: HIGH — grounded in prior codebase-level PITFALLS.md research; all verified against actual project files

**Research date:** 2026-04-01
**Valid until:** 2026-05-01 (stable tooling; tsup version should be re-verified if more than 2 weeks pass before execution)
