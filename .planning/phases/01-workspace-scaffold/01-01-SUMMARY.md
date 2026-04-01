---
phase: 01-workspace-scaffold
plan: 01
subsystem: infra
tags: [pnpm, typescript, tsup, monorepo, workspace, tsconfig]

# Dependency graph
requires: []
provides:
  - pnpm workspace declaration registering packages/@quent/* glob
  - Shared tsconfig.base.json with bundler-mode compiler options
  - Four @quent/* package skeletons (utils, client, hooks, components) each with package.json, tsconfig.json, tsup.config.ts, src/index.ts
  - Source-first exports wiring (main: src/index.ts) for dev loop without build step
  - ESM-only tsup build config with dts emission per package
affects: [02-vite-tailwind-guards, 03-hooks-extraction, 04-components-extraction, 05-client-extraction]

# Tech tracking
tech-stack:
  added: [tsup@^8.0.0 (dev, per-package)]
  patterns: [source-first exports, tsconfig inheritance via extends, ESM-only package output, per-package peerDependencies, composite tsconfig for packages]

key-files:
  created:
    - ui/pnpm-workspace.yaml
    - ui/tsconfig.base.json
    - ui/packages/@quent/utils/package.json
    - ui/packages/@quent/utils/tsconfig.json
    - ui/packages/@quent/utils/tsup.config.ts
    - ui/packages/@quent/utils/src/index.ts
    - ui/packages/@quent/client/package.json
    - ui/packages/@quent/client/tsconfig.json
    - ui/packages/@quent/client/tsup.config.ts
    - ui/packages/@quent/client/src/index.ts
    - ui/packages/@quent/hooks/package.json
    - ui/packages/@quent/hooks/tsconfig.json
    - ui/packages/@quent/hooks/tsup.config.ts
    - ui/packages/@quent/hooks/src/index.ts
    - ui/packages/@quent/components/package.json
    - ui/packages/@quent/components/tsconfig.json
    - ui/packages/@quent/components/tsup.config.ts
    - ui/packages/@quent/components/src/index.ts
  modified:
    - ui/pnpm-lock.yaml

key-decisions:
  - "tsconfig.base.json excludes noEmit, allowImportingTsExtensions, composite, outDir — those are app-level or per-package concerns"
  - "Source-first exports (main: src/index.ts) enable Vite dev loop without any build step"
  - "tsup format: esm-only, dts: true — no CJS output, declarations alongside ESM"
  - "peerDependencies per package role: utils=none, client=react+tanstack, hooks=react+jotai, components=react+react-dom"

patterns-established:
  - "Package tsconfig extends ../../tsconfig.base.json and adds composite/declaration/noEmit:false"
  - "Source-first package.json exports for workspace development, update to dist/ only at publish time"
  - "tsup.config.ts identical across packages: entry src/index.ts, ESM, dts, sourcemap"

requirements-completed: [INFRA-01, INFRA-02, INFRA-03]

# Metrics
duration: 15min
completed: 2026-04-01
---

# Phase 1 Plan 01: Workspace Scaffold — Package Skeletons Summary

**pnpm workspace with tsconfig inheritance and four ESM-only @quent/* package skeletons (utils, client, hooks, components) with source-first exports and per-role peerDependencies**

## Performance

- **Duration:** 15 min
- **Started:** 2026-04-01T17:25:07Z
- **Completed:** 2026-04-01T17:40:27Z
- **Tasks:** 2
- **Files modified:** 19 (18 created + pnpm-lock.yaml updated)

## Accomplishments

- pnpm workspace declared at `ui/pnpm-workspace.yaml` registering `packages/@quent/*` glob
- `ui/tsconfig.base.json` with shared compiler options (strict, bundler moduleResolution, react-jsx) without emit/composite fields
- Four `@quent/*` package skeletons created with identical tsup config (ESM+dts) and tsconfig inheritance
- Per-package peerDependencies correctly scoped: utils=none, client=react+tanstack, hooks=react+jotai, components=react+react-dom
- `pnpm install` resolved workspace packages successfully

## Task Commits

Each task was committed atomically:

1. **Task 1: Create pnpm workspace file and tsconfig base** - `b76378a2` (chore)
2. **Task 2: Create four @quent/* package skeletons** - `875628c1` (feat)

## Files Created/Modified

- `ui/pnpm-workspace.yaml` — Workspace package discovery glob `packages/@quent/*`
- `ui/tsconfig.base.json` — Shared compiler options: ES2020, bundler moduleResolution, strict, react-jsx
- `ui/packages/@quent/utils/{package.json,tsconfig.json,tsup.config.ts,src/index.ts}` — Pure utility package, no peers
- `ui/packages/@quent/client/{package.json,tsconfig.json,tsup.config.ts,src/index.ts}` — API client, peers: react+tanstack
- `ui/packages/@quent/hooks/{package.json,tsconfig.json,tsup.config.ts,src/index.ts}` — Jotai hooks, peers: react+jotai
- `ui/packages/@quent/components/{package.json,tsconfig.json,tsup.config.ts,src/index.ts}` — UI library, peers: react+react-dom
- `ui/pnpm-lock.yaml` — Updated after workspace install

## Decisions Made

- `tsconfig.base.json` deliberately omits `noEmit`, `allowImportingTsExtensions`, `composite`, and `outDir` — the app tsconfig owns `noEmit`/`allowImportingTsExtensions` (D-14), packages own `composite`/`outDir`
- Source-first exports (`"main": "src/index.ts"`) keep dev loop build-free (D-04/D-05); flip to `dist/` only at npm publish time
- ESM-only tsup output (D-07) — no CJS because the app is `"type": "module"` and Vite handles bundling

## Deviations from Plan

None — plan executed exactly as written.

## Issues Encountered

None.

## Known Stubs

All four `src/index.ts` files contain only `export {}` — intentional scaffold stubs. Exports are added in Phase 2+ when code is extracted into the packages. This does not prevent the plan's goal (structural scaffold) from being achieved.

## Next Phase Readiness

- Workspace structure and tsconfig inheritance established — Plan 01-02 (Vite/Tailwind guards) can proceed
- All four packages resolvable via pnpm workspace symlinks
- No blockers for subsequent plans

---
*Phase: 01-workspace-scaffold*
*Completed: 2026-04-01*
