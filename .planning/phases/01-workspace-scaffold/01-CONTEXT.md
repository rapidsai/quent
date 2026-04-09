# Phase 1: Workspace Scaffold - Context

**Gathered:** 2026-04-01
**Status:** Ready for planning

<domain>
## Phase Boundary

Wire the pnpm workspace, tsconfig inheritance, Vite config guards (peerDeps + resolve.dedupe), Tailwind CSS source scanning, and per-package skeletons. No code extracted yet — this phase is purely structural plumbing. The existing app must continue to build and run normally when this phase completes.

</domain>

<decisions>
## Implementation Decisions

### Package directory layout
- **D-01:** Packages live at `ui/packages/@quent/*` — namespaced under the `@quent` scope. Each package directory matches its eventual npm name: `ui/packages/@quent/utils`, `ui/packages/@quent/client`, `ui/packages/@quent/hooks`, `ui/packages/@quent/components`.
- **D-02:** `ui/pnpm-workspace.yaml` uses glob `packages/@quent/*` to register all four packages.
- **D-03:** Each package's `package.json` `name` field is `@quent/<name>` (e.g. `"name": "@quent/utils"`).

### Dev workflow — source-only, no build step
- **D-04:** During development, Vite resolves `@quent/*` imports directly to TypeScript source via pnpm workspace symlinks. No `dist/` directory is required in dev.
- **D-05:** Each package `package.json` sets `"main": "src/index.ts"` and `"exports": { ".": "./src/index.ts" }` for source-first resolution in dev. These fields will be updated to point to `dist/` only when publishing.
- **D-06:** `tsup` is only needed for production builds, not for the dev loop. The `build` script in each package runs tsup; no `--watch` mode is set up.

### tsup output format — ESM-only
- **D-07:** `tsup.config.ts` in each package targets ESM-only output (`format: ['esm']`). No CJS output.
- **D-08:** TypeScript declaration files (`.d.ts`) are emitted alongside ESM output (`dts: true`).
- **D-09:** `tsup` entry point is `src/index.ts`; output directory is `dist/`.

### Vite config guards (pitfall prevention)
- **D-10:** `vite.config.ts` updated with `resolve.dedupe: ['react', 'react-dom', 'jotai', '@tanstack/react-query', '@tanstack/react-router']` to prevent duplicate module instances once packages are linked.
- **D-11:** Each package that uses React, Jotai, or TanStack dependencies declares them as `peerDependencies` (not `dependencies`) in its `package.json`.

### Tailwind CSS source scanning
- **D-12:** `ui/src/index.css` extended with `@source "../packages/@quent/**/*.{ts,tsx}"` so Tailwind v4 scans component source in packages during the app build.

### tsconfig inheritance
- **D-13:** `ui/tsconfig.base.json` created with shared compiler options (strict, target, lib, moduleResolution: bundler, jsx). Each package `tsconfig.json` extends `../../tsconfig.base.json` and adds `composite: true`, `declaration: true`, `noEmit: false`, `outDir: ./dist`.
- **D-14:** The app-level `ui/tsconfig.json` continues to use `noEmit: true` and `allowImportingTsExtensions: true` (incompatible with package emit — packages need their own tsconfig).

### Vitest
- **Claude's Discretion:** Vitest workspace configuration. User did not specify — use a single `ui/vitest.workspace.ts` that covers the app and all packages. Each package can have its own `vitest.config.ts` that is picked up by the workspace.

</decisions>

<specifics>
## Specific Ideas

- The `@quent` namespace scope is chosen to match eventual npm package names — no renaming needed at publish time.
- Source-first resolution (`"main": "src/index.ts"`) is the shadcn/Jotai/TanStack pattern for monorepo packages — fast HMR, no stale dist artifacts.

</specifics>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project requirements
- `.planning/REQUIREMENTS.md` §INFRA-01 through INFRA-06 — all scaffold requirements
- `.planning/ROADMAP.md` §Phase 1 — phase goal and success criteria

### Existing configuration (read before modifying)
- `ui/package.json` — existing scripts, engines, volta config
- `ui/vite.config.ts` — existing Vite config (no resolve.dedupe yet; add it here)
- `ui/tsconfig.json` — existing app tsconfig (uses noEmit: true, allowImportingTsExtensions: true — do NOT change these)
- `ui/tsconfig.node.json` — existing node tsconfig with composite: true (use as pattern for package tsconfigs)
- `ui/src/index.css` — add @source directive here for Tailwind package scanning
- `ui/vitest.config.ts` — existing test config (do not break it; workspace config should include it)

### Research
- `.planning/research/STACK.md` — tsup config recommendations, tsconfig strategy, peerDeps pattern
- `.planning/research/PITFALLS.md` — duplicate instance risks, Tailwind purge fix, tsconfig conflicts

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Patterns
- `ui/tsconfig.node.json` uses `composite: true` — copy this pattern for package tsconfigs
- `ui/package.json` enforces pnpm via preinstall script — package.json files should follow same engines/volta structure

### Integration Points
- `ui/vite.config.ts` — add `resolve.dedupe` here
- `ui/src/index.css` — add `@source` directive here
- `ui/` root — add `pnpm-workspace.yaml` here

</code_context>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 01-workspace-scaffold*
*Context gathered: 2026-04-01*
