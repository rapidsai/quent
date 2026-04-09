---
phase: 01-workspace-scaffold
verified: 2026-04-01T00:00:00Z
status: human_needed
score: 9/10 must-haves verified
re_verification: false
human_verification:
  - test: "Run pnpm install from ui/ and confirm exit 0 with four workspace packages linked"
    expected: "All four @quent/* packages resolved as symlinks in node_modules; no install errors"
    why_human: "Cannot run pnpm install in sandbox without network/registry access; structural evidence (package.json + pnpm-workspace.yaml) is correct but live install is unverifiable here"
  - test: "Run pnpm typecheck (tsr generate && tsc --noEmit) from ui/"
    expected: "Exit 0; all four package tsconfigs resolved via ../../../tsconfig.base.json"
    why_human: "Requires running tsc; cannot execute compiler in verification environment"
  - test: "Run pnpm test:run from ui/"
    expected: "37 tests pass, exit 0; vitest.workspace.ts picks up vitest.config.ts correctly"
    why_human: "Requires running vitest; cannot execute test runner in verification environment"
---

# Phase 01: Workspace Scaffold Verification Report

**Phase Goal:** Scaffold the pnpm monorepo workspace and four @quent/* package skeletons so the app can import from them.
**Verified:** 2026-04-01
**Status:** human_needed (all automated checks pass; 3 runtime checks need human execution)
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | pnpm-workspace.yaml exists and declares packages/@quent/* glob | VERIFIED | `ui/pnpm-workspace.yaml` line 2: `- 'packages/@quent/*'` |
| 2 | tsconfig.base.json exists with shared options excluding noEmit and allowImportingTsExtensions | VERIFIED | File exists, 0 grep hits for excluded fields; moduleResolution present |
| 3 | Each of the four packages has package.json, tsconfig.json, tsup.config.ts, and src/index.ts | VERIFIED | All 16 files present under ui/packages/@quent/{utils,client,hooks,components}/ |
| 4 | Package tsconfigs extend ../../../tsconfig.base.json with composite:true, declaration:true, noEmit:false | VERIFIED | All four tsconfigs: extends="../../../tsconfig.base.json", composite:true, declaration:true, noEmit:false |
| 5 | Package package.json files use source-first exports pointing to src/index.ts | VERIFIED | All four: "main":"src/index.ts" and exports["."]:"./src/index.ts" |
| 6 | ui/package.json lists all four packages via workspace:* protocol | VERIFIED | Lines 34-37 of ui/package.json contain all four workspace:* deps |
| 7 | ui/tsconfig.json references array includes all four packages | VERIFIED | Lines 34-37 of ui/tsconfig.json; noEmit:true and allowImportingTsExtensions:true preserved |
| 8 | vite.config.ts has resolve.dedupe, optimizeDeps.include, server.watch.followSymlinks | VERIFIED | Lines 63, 73-75, 77-79 of vite.config.ts |
| 9 | ui/src/index.css has @source directive scanning packages directory | VERIFIED | Line 5: `@source "../packages/@quent/**/*.{ts,tsx}"` |
| 10 | ui/vitest.workspace.ts exists with defineWorkspace referencing app config and package glob | VERIFIED | File exists; defineWorkspace with './vitest.config.ts' and './packages/@quent/*/vitest.config.ts' |

**Score:** 10/10 truths structurally verified. Three truths require runtime execution (human verification).

---

## Required Artifacts

### Plan 01-01 Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `ui/pnpm-workspace.yaml` | Workspace package discovery | VERIFIED | Contains `packages/@quent/*` |
| `ui/tsconfig.base.json` | Shared TypeScript compiler options | VERIFIED | Contains moduleResolution; excludes noEmit, allowImportingTsExtensions, composite, outDir |
| `ui/packages/@quent/utils/package.json` | Package identity, source-first exports | VERIFIED | @quent/utils, main:src/index.ts, no peerDependencies |
| `ui/packages/@quent/utils/tsconfig.json` | composite tsconfig extending base | VERIFIED | extends ../../../tsconfig.base.json, composite:true, noEmit:false |
| `ui/packages/@quent/utils/tsup.config.ts` | ESM-only build config | VERIFIED | defineConfig, format:['esm'], dts:true |
| `ui/packages/@quent/utils/src/index.ts` | Empty package entry point | VERIFIED | `export {}` (intentional scaffold stub) |
| `ui/packages/@quent/client/package.json` | Package identity, react+tanstack peers | VERIFIED | peerDependencies: react, @tanstack/react-query, @tanstack/react-router |
| `ui/packages/@quent/client/tsconfig.json` | composite tsconfig extending base | VERIFIED | extends ../../../tsconfig.base.json |
| `ui/packages/@quent/client/tsup.config.ts` | ESM-only build config | VERIFIED | defineConfig, format:['esm'], dts:true |
| `ui/packages/@quent/client/src/index.ts` | Empty package entry point | VERIFIED | `export {}` |
| `ui/packages/@quent/hooks/package.json` | Package identity, react+jotai peers | VERIFIED | peerDependencies: react, jotai |
| `ui/packages/@quent/hooks/tsconfig.json` | composite tsconfig extending base | VERIFIED | extends ../../../tsconfig.base.json |
| `ui/packages/@quent/hooks/tsup.config.ts` | ESM-only build config | VERIFIED | defineConfig, format:['esm'], dts:true |
| `ui/packages/@quent/hooks/src/index.ts` | Empty package entry point | VERIFIED | `export {}` |
| `ui/packages/@quent/components/package.json` | Package identity, react+react-dom peers | VERIFIED | peerDependencies: react, react-dom |
| `ui/packages/@quent/components/tsconfig.json` | composite tsconfig extending base | VERIFIED | extends ../../../tsconfig.base.json |
| `ui/packages/@quent/components/tsup.config.ts` | ESM-only build config | VERIFIED | defineConfig, format:['esm'], dts:true |
| `ui/packages/@quent/components/src/index.ts` | Empty package entry point | VERIFIED | `export {}` |

### Plan 01-02 Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `ui/vite.config.ts` | Dedupe + HMR + optimizeDeps | VERIFIED | resolve.dedupe line 63, optimizeDeps.include lines 73-75, server.watch.followSymlinks line 78 |
| `ui/vitest.workspace.ts` | Workspace-level test runner | VERIFIED | defineWorkspace with app config + package glob |
| `ui/src/index.css` | Tailwind @source for packages | VERIFIED | Line 5: `@source "../packages/@quent/**/*.{ts,tsx}"` |
| `ui/package.json` | workspace:* deps for all four packages | VERIFIED | All four @quent/* packages at workspace:* |
| `ui/tsconfig.json` | Project references for all four packages | VERIFIED | Four package references added; noEmit:true preserved |

---

## Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `ui/packages/@quent/*/tsconfig.json` | `ui/tsconfig.base.json` | extends field | VERIFIED | All four use `"extends": "../../../tsconfig.base.json"` (corrected from ../../) |
| `ui/packages/@quent/*/package.json` | `ui/packages/@quent/*/src/index.ts` | main and exports fields | VERIFIED | All four: `"main": "src/index.ts"`, `"exports": {".": "./src/index.ts"}` |
| `ui/package.json` | `ui/packages/@quent/*/package.json` | workspace:* protocol | VERIFIED | All four @quent packages listed with `workspace:*` in dependencies |
| `ui/vite.config.ts` | react, react-dom, jotai, @tanstack/* | resolve.dedupe array | VERIFIED | `dedupe: ['react', 'react-dom', 'jotai', '@tanstack/react-query', '@tanstack/react-router']` |
| `ui/vitest.workspace.ts` | `ui/vitest.config.ts` | workspace config reference | VERIFIED | `'./vitest.config.ts'` listed in defineWorkspace array |
| `ui/tsconfig.json` | `ui/packages/@quent/*/tsconfig.json` | references array | VERIFIED | All four packages in references array at lines 34-37 |

---

## Data-Flow Trace (Level 4)

Not applicable. This phase produces infrastructure configuration files only — no components that render dynamic data.

---

## Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| tsconfig.base.json excludes forbidden fields | `grep -c "noEmit\|allowImportingTsExtensions\|composite\|outDir" ui/tsconfig.base.json` | 0 matches | PASS |
| workspace glob correct | `grep "packages/@quent" ui/pnpm-workspace.yaml` | match found | PASS |
| dedupe array in vite.config.ts | `grep "dedupe" ui/vite.config.ts` | match found | PASS |
| @source directive in index.css | `grep "@source" ui/src/index.css` | match found | PASS |
| All 16 package files exist | ls check for all four packages | All present | PASS |
| Commits referenced in SUMMARY exist | `git log --oneline b76378a2 875628c1 186d5687 ee71bfc5` | All four found | PASS |
| pnpm install from ui/ resolves packages | Requires running pnpm | Not run | NEEDS HUMAN |
| tsc --noEmit passes from ui/ | Requires running tsc | Not run | NEEDS HUMAN |
| pnpm test:run passes (37 tests) | Requires running vitest | Not run | NEEDS HUMAN |

---

## Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| INFRA-01 | 01-01 | pnpm-workspace.yaml with packages/* glob; all four resolved via workspace:* | SATISFIED | pnpm-workspace.yaml with packages/@quent/*; all four in ui/package.json as workspace:* |
| INFRA-02 | 01-01 | tsconfig.base.json created; each package extends with composite:true, declaration:true, noEmit:false | SATISFIED | tsconfig.base.json verified; all four package tsconfigs verified with correct options |
| INFRA-03 | 01-01 | Each package has tsup.config.ts for publishability-ready builds (esm + cjs, .d.ts) | PARTIAL — see note | tsup.config.ts exists in all four packages with dts:true; format is ESM-only, not esm+cjs |
| INFRA-04 | 01-02 | peerDependencies in packages that use them; vite.config.ts with resolve.dedupe | SATISFIED | Per-role peerDeps verified; resolve.dedupe present in vite.config.ts |
| INFRA-05 | 01-02 | vitest.workspace.ts enables per-package test runs from workspace root | SATISFIED | vitest.workspace.ts with defineWorkspace exists; runtime verification needed |
| INFRA-06 | 01-02 | Tailwind @source directive in ui/src/index.css covering packages/**/*.{ts,tsx} | SATISFIED | Line 5 of index.css: `@source "../packages/@quent/**/*.{ts,tsx}"` |

### Note on INFRA-03

REQUIREMENTS.md states `esm + cjs` for INFRA-03. The implementation delivers ESM-only (`format: ['esm']`). This is a deliberate decision documented in CONTEXT.md D-07: "tsup.config.ts in each package targets ESM-only output (format: ['esm']). No CJS output." The rationale is that the app uses `"type": "module"` and Vite handles bundling. `.d.ts` generation is present (`dts: true`). The requirement text and the implementation decision conflict. This is flagged as PARTIAL because the requirement text was not retroactively updated to match the decision — it is a documentation gap, not a functional gap. The phase PLAN (01-01) explicitly specifies `format: ['esm']` throughout, so the plan and implementation agree; only the REQUIREMENTS.md text is inconsistent.

### Orphaned Requirements

No orphaned requirements. All six IDs (INFRA-01 through INFRA-06) are claimed by plans 01-01 and 01-02 and confirmed present in REQUIREMENTS.md traceability table.

---

## Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `ui/packages/@quent/*/src/index.ts` | 2 | `export {}` (empty export) | Info | Intentional scaffold stub documented in SUMMARY and PLAN; exports added in Phase 2+. Not a blocker. |

No TODO/FIXME/placeholder comments found in any phase-created files. No console.log-only implementations. No hardcoded empty data flowing to render paths (these files are infrastructure configs, not UI components).

---

## Human Verification Required

### 1. pnpm install resolves workspace packages

**Test:** From `ui/`, run `pnpm install` and then `pnpm why @quent/utils`
**Expected:** Exit 0; `@quent/utils` shown as a symlink to `packages/@quent/utils`; same for client, hooks, components
**Why human:** Cannot execute pnpm in verification environment without network access

### 2. TypeScript compilation passes

**Test:** From `ui/`, run `pnpm typecheck` (which runs `tsr generate && tsc --noEmit`)
**Expected:** Exit 0 with no type errors; package tsconfigs resolved correctly via ../../../tsconfig.base.json
**Why human:** Requires running the TypeScript compiler

### 3. Test suite passes without regression

**Test:** From `ui/`, run `pnpm test:run`
**Expected:** All 37 tests pass, exit 0; no tests duplicated or lost from vitest.workspace.ts introduction
**Why human:** Requires running the vitest test runner

---

## Gaps Summary

No structural gaps. All 18 expected files from Plan 01-01 are present and substantive. All 5 files from Plan 01-02 are present and correctly wired. The four package tsconfigs were correctly fixed (../../ -> ../../../) during Plan 01-02 execution.

One documentation inconsistency: INFRA-03 in REQUIREMENTS.md says "esm + cjs" but the deliberate implementation decision (D-07 in CONTEXT.md, and explicit in the PLAN) is ESM-only. This is a requirements-documentation mismatch, not a functional problem. The phase PLAN and SUMMARY are internally consistent on ESM-only.

Three runtime behaviors (pnpm install, tsc, vitest) cannot be verified statically and require human execution.

---

_Verified: 2026-04-01_
_Verifier: Claude (gsd-verifier)_
