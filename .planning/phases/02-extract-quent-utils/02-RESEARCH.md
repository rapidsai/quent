# Phase 02: Extract @quent/utils - Research

**Researched:** 2026-04-01
**Domain:** TypeScript monorepo package extraction — barrel re-exports, import migration, pnpm workspace
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** Rust-generated types live in `examples/simulator/server/ts-bindings/` (52 files). `@quent/utils` re-exports them via a barrel using relative path imports. The `examples/simulator/server/ts-bindings/` directory remains the source of truth.
- **D-02:** The `~quent/types` path alias is removed from `vite.config.ts` and `tsconfig.json`. App code migrates from `import type { Foo } from '~quent/types/Foo'` to `import type { Foo } from '@quent/utils'`.
- **D-03:** Relative path from the package to `examples/simulator/server/ts-bindings/` crosses package boundaries. Acceptable for this milestone. Publishability is Phase V2/publish concern.
- **D-04:** All of `ui/src/services/colors.ts` is extracted to `@quent/utils`, including browser-specific canvas pattern functions.
- **D-05:** Canvas pattern functions annotated with `@remarks Browser-only — requires \`document.createElement('canvas')\`. Do not call in SSR or Node.js environments.`
- **D-06:** Module-level mutable state (`colorAssignments`, `usedIndices`, `activePalette`) is preserved as-is. Phase 1 `resolve.dedupe` ensures a single module instance at runtime.
- **D-07:** `parseJsonWithBigInt` is extracted from `ui/src/services/api.ts` and exported from `@quent/utils`.
- **D-08:** After extraction, `ui/src/services/api.ts` imports `parseJsonWithBigInt` from `@quent/utils`. Establishes `@quent/client` → `@quent/utils` dependency direction for Phase 3.
- **D-09:** All of `ui/src/services/formatters.ts` is extracted to `@quent/utils`. Formatter imports of Rust types (`PrefixSystem`, `QuantitySpec`, `CapacityKind`) are resolved through the `@quent/utils` types barrel (relative intra-package imports — no circular dependency).
- **D-10:** `cn()` extracted from `ui/src/lib/utils.ts`. `clsx` and `tailwind-merge` become direct `dependencies` (not peerDependencies) of `@quent/utils`.
- **D-11:** All app imports of the extracted modules are migrated:
  - `@/lib/utils` → `@quent/utils` (for `cn`)
  - `~quent/types/*` → `@quent/utils` (for Rust types)
  - `@/services/formatters` → `@quent/utils`
  - `@/services/colors` → `@quent/utils`
  - `parseJsonWithBigInt` from `@/services/api` → `@quent/utils` (within `api.ts` itself)
- **D-12:** After migration, `ui/src/lib/utils.ts`, `ui/src/services/colors.ts`, and `ui/src/services/formatters.ts` are deleted or reduced to thin re-exports (Claude's discretion — prefer deletion if no other app code is in those files).
- **D-13:** `@quent/utils/package.json` dependencies: `clsx`, `tailwind-merge` as direct deps. No React, Jotai, or TanStack deps.
- **D-14:** `@quent/utils` has no package-level peerDependencies — it is a zero-framework-dep utility package.

### Claude's Discretion

- Whether to delete `ui/src/lib/utils.ts`, `ui/src/services/colors.ts`, `ui/src/services/formatters.ts` or convert them to thin re-exports. Either is acceptable — prefer deletion if no other app code is in those files.
- The exact structure of `src/index.ts` barrel (grouped by category or flat list) — whatever is clearest.
- Whether to break the extraction into multiple plan files or handle in one plan.

### Deferred Ideas (OUT OF SCOPE)

- Publishability fix: when publishing, the Rust type re-exports from a relative path outside the package boundary will need to be bundled or the types copied into the package. Not needed now.
- Wong palette constants were mentioned in REQUIREMENTS.md as a named export. They are already present in `colors.ts` as `PALETTES.wong` — ensure this is exported by name from the index barrel.

</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| UTILS-01 | `cn()` extracted from `ui/src/lib/utils.ts` and exported from `@quent/utils` | Source file is 9 lines — direct copy; `clsx` and `tailwind-merge` must be added to `@quent/utils` `dependencies`. |
| UTILS-02 | All 52 Rust-generated TypeScript types re-exported from `@quent/utils`; `~quent/types` path alias removed from `vite.config.ts` and `tsconfig.json` | Types live in `examples/simulator/server/ts-bindings/`; some cross-reference each other with relative `./` imports (they must remain intact). A barrel in `@quent/utils/src/types/index.ts` re-exports all 52 via relative paths from the package. Removing the alias from `vite.config.ts` line 68 and `tsconfig.json` lines 28-29 and 31. |
| UTILS-03 | `parseJsonWithBigInt` exported from `@quent/utils` | Function is 23 lines (lines 31–53 of `api.ts`). No dependencies. A covering vitest test suite already exists at `ui/src/services/api.test.ts` — test file must be moved/re-pointed alongside the function. |
| UTILS-04 | Color utilities extracted with `getOperationTypeColor`, `assignColors`, Wong palette constants | **Important finding:** `getOperationTypeColor` does NOT exist in `colors.ts`. It is mentioned in REQUIREMENTS.md but is absent from the source. See Pitfall section below. All other exports from `colors.ts` are present and extractable. |
| UTILS-05 | Formatter utilities extracted: duration, timestamp, and size formatters | `formatters.ts` imports `PrefixSystem`, `QuantitySpec`, `CapacityKind` via `~quent/types/*`. After extraction, these imports become intra-package relative imports (e.g., `../types/PrefixSystem` or `./types/PrefixSystem`). No circular dependency introduced. |

</phase_requirements>

---

## Summary

Phase 2 is a pure extraction and import migration phase. The `@quent/utils` package skeleton was created in Phase 1 and already has an empty `src/index.ts`, correct `package.json` (source-first exports), `tsconfig.json`, and `tsup.config.ts`. No new infrastructure is required.

Five modules need to move: `cn()` (trivial), the 52 Rust type bindings (barrel re-export via relative paths), `parseJsonWithBigInt` (23 lines, no deps), color utilities (self-contained, one canvas concern), and formatter utilities (depends on three Rust types — intra-package after extraction).

The primary complexity is the import migration sweep across 40+ app files that currently use `~quent/types/*`, `@/lib/utils`, `@/services/colors`, or `@/services/formatters`. This is mechanical but must be thorough. The existing test for `parseJsonWithBigInt` must be updated to import from the new location (or remain in the app pointing at the extracted function indirectly).

One open finding: `getOperationTypeColor` is listed in REQUIREMENTS.md UTILS-04 but does not exist in `colors.ts`. The planner must resolve this gap — either the function needs to be created, or the requirement text was aspirational. The Wong palette and `assignColors` are present in `colors.ts` and extractable as-is.

**Primary recommendation:** Execute extraction in three tasks — (1) populate the package source files and update `package.json` dependencies, (2) update the vitest config to resolve `~quent/types` in test env and update the `api.test.ts` import path, (3) migrate all app imports and remove the `~quent/types` alias.

---

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| clsx | 2.1.1 (installed) | Class name joining | Already in app deps |
| tailwind-merge | 3.5.0 (installed) | Tailwind class deduplication | Already in app deps |
| typescript | ^5.9.3 | Type checking | Already in package devDeps |
| tsup | ^8.0.0 | Build for publish | Already in package devDeps |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| vitest | ^4.0.18 (app) | Unit testing | Test for `parseJsonWithBigInt` and pure util functions |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Direct barrel re-export of ts-bindings | Copy type files into package | Copying creates a sync problem; re-export is the correct approach |
| Delete source files after migration | Convert to thin re-exports | Thin re-exports add unnecessary indirection — delete if no other code is in the file |

**Installation (changes to `@quent/utils/package.json` only):**
```bash
# From ui/ directory, add deps to the utils package
pnpm --filter @quent/utils add clsx tailwind-merge
```

This is the only pnpm install needed. Both packages are already in the workspace root `node_modules`.

---

## Architecture Patterns

### Recommended Package Structure
```
ui/packages/@quent/utils/
├── src/
│   ├── index.ts         # Public barrel — named exports only, no export *
│   ├── cn.ts            # cn() function
│   ├── parseJsonWithBigInt.ts   # parseJsonWithBigInt function
│   ├── colors.ts        # All color utilities (moved verbatim from ui/src/services/colors.ts)
│   ├── formatters.ts    # All formatter utilities (moved verbatim, imports updated)
│   └── types/
│       └── index.ts     # Barrel re-exporting all 52 ts-binding files
├── package.json         # Add clsx + tailwind-merge as dependencies
├── tsconfig.json        # Existing — no changes needed
└── tsup.config.ts       # Existing — no changes needed
```

### Pattern 1: Source-First Export (already established in Phase 1)
**What:** `package.json` `"main": "src/index.ts"` and `"exports": { ".": "./src/index.ts" }`. Vite resolves directly from source; no build needed in dev.
**When to use:** Always during workspace development. tsup build only needed for publish.
**Example:**
```json
// @quent/utils/package.json (existing, no change needed)
{
  "main": "src/index.ts",
  "exports": { ".": "./src/index.ts" }
}
```

### Pattern 2: Types Barrel via Relative Paths
**What:** `src/types/index.ts` uses `export type { … } from '../../../../examples/simulator/server/ts-bindings/Foo'` for all 52 files.
**When to use:** When re-exporting types from a directory outside the package boundary.
**Example:**
```typescript
// src/types/index.ts
export type { Attribute } from '../../../../examples/simulator/server/ts-bindings/Attribute';
export type { BinnedSpanSec } from '../../../../examples/simulator/server/ts-bindings/BinnedSpanSec';
// ... 50 more
```

**Relative path:** `ui/packages/@quent/utils/src/types/` → `examples/simulator/server/ts-bindings/` is 4 levels up from `src/types/`, then into `examples/simulator/server/ts-bindings/`. Concretely: `../../../../examples/simulator/server/ts-bindings/Foo`.

**Verify the path.** From `ui/packages/@quent/utils/src/types/`:
- `../` → `ui/packages/@quent/utils/src/`
- `../../` → `ui/packages/@quent/utils/`
- `../../../` → `ui/packages/@quent/`
- `../../../../` → `ui/packages/`
- `../../../../../` → `ui/`
- `../../../../../../` → project root
- `../../../../../../examples/simulator/server/ts-bindings/` — 6 levels up to project root, then into examples.

Wait — let me recount. The workspace root is `ui/`, not the project root:
- `ui/packages/@quent/utils/src/types/` needs to reach `examples/simulator/server/ts-bindings/`
- `examples/` is at the project root (`/Users/johallaron/Projects/quent/examples/`)
- `ui/packages/@quent/utils/src/types/` is 6 levels below the project root

Correct relative path: `../../../../../../examples/simulator/server/ts-bindings/Foo`

**Cross-check:** `ui/vite.config.ts` already resolves `'~quent/types'` via `path.resolve(__dirname, '../examples/simulator/server/ts-bindings')` — meaning from `ui/`, `../examples/` reaches the bindings. From `ui/packages/@quent/utils/src/types/`, that is 5 more levels down, so: `../../../../../../examples/simulator/server/ts-bindings/Foo`. Confirm by counting: `ui/packages/@quent/utils/src/types/` → `../` = `src/`, `../../` = `utils/`, `../../../` = `@quent/`, `../../../../` = `packages/`, `../../../../../` = `ui/`, `../../../../../../` = project root. Then `examples/simulator/server/ts-bindings/Foo`. **6 levels up** from `src/types/`.

### Pattern 3: Intra-Package Formatter Imports (resolves the ~quent/types dependency)
**What:** After `formatters.ts` is moved into `@quent/utils`, it imports `PrefixSystem`, `QuantitySpec`, `CapacityKind` from the types barrel within the same package.
**When to use:** When a module that used a path alias is moved into the package that owns the alias target.
**Example:**
```typescript
// src/formatters.ts (after migration)
import type { PrefixSystem } from './types/index';
import type { QuantitySpec } from './types/index';
import type { CapacityKind } from './types/index';
```
Or, more cleanly, since they all come from the same barrel:
```typescript
import type { PrefixSystem, QuantitySpec, CapacityKind } from './types/index';
```
No circular dependency: `formatters.ts` imports from `types/index.ts`; `types/index.ts` re-exports from external files; `index.ts` (the root barrel) exports from both. No cycle.

### Pattern 4: Migrating ~quent/types/* App Imports
**What:** All `import type { Foo } from '~quent/types/Foo'` in app files change to `import type { Foo } from '@quent/utils'`.
**When to use:** 40+ files across `ui/src/` that currently use the path alias.
**Example:**
```typescript
// Before (in ui/src/services/api.ts)
import type { QueryBundle } from '~quent/types/QueryBundle';

// After
import type { QueryBundle } from '@quent/utils';
```

### Pattern 5: Named Index Barrel (no `export *`)
**What:** `src/index.ts` lists every export by name. No `export * from './types'`. Explicit exports only.
**Why:** REQUIREMENTS.md COMP-07 (mirrored in UTILS intent): "index.ts barrel export lists all public exports by name (no `export *`)". This is a project-wide convention.
**Example:**
```typescript
// src/index.ts
export { cn } from './cn';
export { parseJsonWithBigInt } from './parseJsonWithBigInt';
export * from './types/index'; // EXCEPTION: types barrel may use export * since each type file has one named export
// ... or enumerate all 52 type names explicitly
```
**Recommendation:** Use `export * from './types/index'` for the types sub-barrel (52 files would make the root barrel unreadable), but use named exports for all utility functions. The types sub-barrel itself should use explicit `export type { Foo } from '...'` per file (already the pattern).

### Anti-Patterns to Avoid
- **Importing `@quent/utils` from within `@quent/utils`**: `formatters.ts` and `colors.ts` must use relative imports (`./types/index`) not `import from '@quent/utils'` — that would create a circular self-reference.
- **Leaving `~quent/types` alias in place**: After migration, the alias must be removed from both `vite.config.ts` and `tsconfig.json`. Leaving it creates a false sense of correctness and will break Phase 3.
- **Adding `~quent/types` to vitest config**: The app vitest config (`ui/vitest.config.ts`) does not include the `~quent/types` alias. Currently `api.test.ts` imports from `./api` not from types directly — this is fine. But if any test imports via `~quent/types`, it will fail. Audit needed.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Merging Tailwind classes | Custom string concat logic | `tailwind-merge` + `clsx` (already in `cn()`) | Handles specificity rules, arbitrary values, responsive variants |
| TypeScript barrel generation | Script to write 52 export lines | Write the barrel once by hand or via a one-off script | 52 files is manageable; generation adds tooling complexity |
| Import path rewriting | Regex-only mass replacement | Targeted search-replace per import pattern | Mechanical but requires per-file precision; one pattern at a time |

---

## Common Pitfalls

### Pitfall 1: getOperationTypeColor Does Not Exist
**What goes wrong:** REQUIREMENTS.md UTILS-04 lists `getOperationTypeColor` as a required export. This function does not exist in `ui/src/services/colors.ts`.
**Why it happens:** The requirements may have been written anticipating a function that was never implemented, or the functionality is handled by CVA variants in `QueryPlanNode.tsx` (verified: operation type coloring is done via `cva` in that file, not via a colors.ts function).
**How to avoid:** Do not fabricate the function. The planner must either: (a) create a new `getOperationTypeColor` function in `@quent/utils/src/colors.ts` based on the CVA variants in `QueryPlanNode.tsx`, or (b) acknowledge the gap and satisfy UTILS-04 with all other color exports. All other color utility exports (`PALETTES`, `getColorForKey`, `assignColors`, `withOpacity`, `lightenColor`, `darkenColor`, `getColorByIndex`, `getActivePalette`, `setActivePalette`, `getPalette`, `resetColorAssignments`, canvas pattern functions, `BLACK`, `WHITE`) are present and ready.
**Warning signs:** Any plan that says "extract getOperationTypeColor from colors.ts" without first creating it is wrong.

### Pitfall 2: Wrong Relative Path to ts-bindings
**What goes wrong:** The barrel `src/types/index.ts` uses the wrong number of `../` segments to reach `examples/simulator/server/ts-bindings/`.
**Why it happens:** The package is deeply nested (`ui/packages/@quent/utils/src/types/`). Miscounting levels is easy.
**How to avoid:** The correct path is 6 levels up from `src/types/` to the project root, then `examples/simulator/server/ts-bindings/Foo`. Use: `../../../../../../examples/simulator/server/ts-bindings/Foo`. Verify by confirming `ui/` is at level 5 (one level up from project root is wrong — project root IS where `ui/` and `examples/` live side by side).
**Warning signs:** TypeScript errors claiming the import path cannot be found; or types resolving but showing wrong file paths.

### Pitfall 3: ts-bindings Inter-References Must Stay Intact
**What goes wrong:** Some ts-binding files import from sibling files using `./` relative paths (e.g., `QuantitySpec.ts` imports `PrefixSystem.ts`). These relative imports work because both files are in the same directory.
**Why it happens:** ts-rs generates files with relative sibling imports.
**How to avoid:** Do NOT touch the ts-binding files themselves. The barrel only adds `export type { Foo } from '…/Foo'` wrappers. The original files remain in place and their internal imports remain unchanged.
**Warning signs:** TypeScript errors inside the ts-bindings files themselves after adding the barrel.

### Pitfall 4: vitest Config Missing ~quent/types Alias
**What goes wrong:** `ui/vitest.config.ts` does not define a `~quent/types` path alias. After migration, this doesn't matter since the alias is removed. But if tests still import from `~quent/types/*` at test execution time, vitest will fail.
**Why it happens:** `vitest.config.ts` defines only `@` alias; it did not replicate the `~quent/types` alias from `vite.config.ts`.
**How to avoid:** Migrate all `~quent/types` imports in test files to `@quent/utils` as part of the import migration sweep. Also update `api.test.ts` import from `./api` to either `./api` (still OK, function stays in api.ts, now re-exported) or from `@quent/utils` directly.
**Warning signs:** vitest run fails with "Cannot find module '~quent/types/...'" after the alias is removed from vite.config.ts.

### Pitfall 5: api.test.ts Import Path
**What goes wrong:** `ui/src/services/api.test.ts` currently imports `parseJsonWithBigInt` from `'./api'`. After extraction, `api.ts` re-imports `parseJsonWithBigInt` from `@quent/utils` and re-exports nothing (it no longer exports the function). The test import breaks.
**Why it happens:** The test imports from the original location. After extraction, `api.ts` no longer exports `parseJsonWithBigInt`.
**How to avoid:** Either (a) update `api.test.ts` to import `parseJsonWithBigInt` from `'@quent/utils'` directly, or (b) keep a re-export from `api.ts` temporarily. Option (a) is cleaner. The test file stays in `ui/src/services/api.test.ts` (it exercises the function, not the file location).
**Warning signs:** `api.test.ts` fails with "parseJsonWithBigInt is not exported from './api'".

### Pitfall 6: tsconfig.json ~quent/types Removal Scope
**What goes wrong:** `ui/tsconfig.json` contains `~quent/types/*` in both `compilerOptions.paths` AND `include`. Removing only `paths` but not the `include` entry (or vice versa) leaves the config inconsistent.
**Why it happens:** The `include` at line 31 (`"../examples/simulator/server/ts-bindings"`) was added to allow TypeScript to type-check those files. After migration, the `@quent/utils` package handles this via its own `tsconfig.json` (which includes `src/` — and the types barrel is in `src/types/`).
**How to avoid:** Remove BOTH the `paths` entry for `~quent/types` (line 28-29) AND evaluate whether the `include` entry for ts-bindings (line 31) is still needed. After the alias is removed and all app imports go through `@quent/utils`, the app `tsconfig.json` no longer needs to directly include the ts-bindings directory. However, the `@quent/utils` package tsconfig only includes `src/` — it does not include the external ts-bindings. TypeScript project references (`"references"`) handle cross-package type checking. This is safe: the package's own `src/types/index.ts` contains the imports, which TypeScript follows transitively.
**Warning signs:** TypeScript errors in `ui/src/` about types not found after alias removal.

---

## Code Examples

Verified patterns from source file inspection:

### cn() — Direct Copy
```typescript
// src/cn.ts (copied verbatim from ui/src/lib/utils.ts, minus the SPDX header if desired)
import { type ClassValue, clsx } from 'clsx';
import { twMerge } from 'tailwind-merge';

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}
```

### parseJsonWithBigInt — Direct Copy
```typescript
// src/parseJsonWithBigInt.ts
export function parseJsonWithBigInt<T>(text: string): T {
  // (lines 31–53 of ui/src/services/api.ts — copy verbatim)
}
```

### src/index.ts — Barrel Structure
```typescript
// src/index.ts
// Utilities
export { cn } from './cn';
export { parseJsonWithBigInt } from './parseJsonWithBigInt';

// Color utilities
export {
  PALETTES,
  getColorForKey,
  assignColors,
  getColorByIndex,
  withOpacity,
  resetColorAssignments,
  lightenColor,
  darkenColor,
  getActivePalette,
  setActivePalette,
  getPalette,
  createStripePattern,
  createDotPattern,
  createCrosshatchPattern,
  BLACK,
  WHITE,
} from './colors';
export type { PaletteName, ChartColor } from './colors';

// Formatter utilities
export {
  formatDuration,
  formatDurationForWindow,
  formatQuantity,
} from './formatters';

// Rust-generated TypeScript types
export * from './types/index';
```

### @quent/utils/package.json — After Adding Dependencies
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
  "dependencies": {
    "clsx": "^2.1.1",
    "tailwind-merge": "^3.5.0"
  },
  "devDependencies": {
    "tsup": "^8.0.0",
    "typescript": "^5.9.3"
  }
}
```

### vite.config.ts — Alias Section After Removal
```typescript
// Remove lines 68-69 from resolve.alias:
// '~quent/types': path.resolve(__dirname, '../examples/simulator/server/ts-bindings'),
// Result:
alias: {
  '@': path.resolve(__dirname, './src'),
  elkjs: 'elkjs/lib/elk.bundled.js',
},
```

### tsconfig.json — After Alias Removal
```json
// Remove from compilerOptions.paths:
// "~quent/types/*": ["../examples/simulator/server/ts-bindings/*"]
// Remove from include array:
// "../examples/simulator/server/ts-bindings"
// Result paths section:
"paths": {
  "@/*": ["./src/*"]
},
"include": ["src"],
```

---

## Runtime State Inventory

Step 2.5 SKIPPED — this is an extraction/import migration phase, not a rename/refactor of identifiers. No runtime state (databases, services, OS registrations, secrets) references the symbols being moved. The `~quent/types` alias exists only in build/editor config files (`vite.config.ts`, `tsconfig.json`), all of which are in git.

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| pnpm | Package install | ✓ | 10.20.0 | — |
| Node.js | Build tooling | ✓ | v24.11.0 | — |
| clsx | cn() implementation | ✓ (workspace) | 2.1.1 | — |
| tailwind-merge | cn() implementation | ✓ (workspace) | 3.5.0 | — |
| TypeScript | Type checking | ✓ (devDep) | ^5.9.3 | — |
| tsup | Build | ✓ (devDep) | ^8.0.0 | — |
| vitest | Tests | ✓ (app workspace) | ^4.0.18 | — |

**Missing dependencies with no fallback:** None.

**Note:** `clsx` and `tailwind-merge` must be explicitly added to `@quent/utils/package.json` `dependencies` even though they are already installed in the workspace root. The source-first pattern means they are resolvable at dev time, but `package.json` must declare them for publishability and correctness.

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | vitest ^4.0.18 |
| Config file | `ui/vitest.config.ts` (app); `ui/vitest.workspace.ts` (workspace) |
| Quick run command | `cd ui && pnpm test:run -- src/services/api.test.ts` |
| Full suite command | `cd ui && pnpm test:run` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| UTILS-01 | `cn()` exported from `@quent/utils` | smoke (type-check + import) | `cd ui && pnpm --filter @quent/utils typecheck` | ❌ Wave 0 — no unit test needed; typecheck suffices |
| UTILS-02 | 52 types accessible via `@quent/utils` | smoke (type-check) | `cd ui && pnpm --filter @quent/utils typecheck` | ❌ Wave 0 — typecheck verifies export surface |
| UTILS-03 | `parseJsonWithBigInt` exported, all behavior preserved | unit | `cd ui && pnpm test:run -- src/services/api.test.ts` | ✅ exists (must update import path) |
| UTILS-04 | Color utilities exported from `@quent/utils` | smoke (type-check) | `cd ui && pnpm --filter @quent/utils typecheck` | ❌ Wave 0 |
| UTILS-05 | Formatter utilities exported from `@quent/utils` | smoke (type-check) | `cd ui && pnpm --filter @quent/utils typecheck` | ❌ Wave 0 |
| All | `pnpm dev` starts, app renders (imports resolved) | integration / manual | `cd ui && pnpm build` (smoke) | — |

### Sampling Rate
- **Per task commit:** `cd ui && pnpm --filter @quent/utils typecheck`
- **Per wave merge:** `cd ui && pnpm test:run`
- **Phase gate:** `cd ui && pnpm build` without errors + full test suite green

### Wave 0 Gaps
- [ ] `api.test.ts` import updated from `'./api'` to `'@quent/utils'` (or the function re-exported from api.ts) — this is a migration task, not a new test file
- [ ] No new vitest config needed for `@quent/utils` — utility functions have no JSdom dependency; `vitest.workspace.ts` will auto-pick up `packages/@quent/utils/vitest.config.ts` if created, but it is not required for this phase

---

## Open Questions

1. **getOperationTypeColor**
   - What we know: REQUIREMENTS.md UTILS-04 requires it; it does not exist in `colors.ts`; color logic for operation types is embedded in `QueryPlanNode.tsx` as CVA variants.
   - What's unclear: Was this an aspirational requirement (the function doesn't exist yet) or an error?
   - Recommendation: Create a minimal `getOperationTypeColor(operationType: string): string` function in `@quent/utils/src/colors.ts` that maps operation type strings to hex colors, based on the CVA variant values in `QueryPlanNode.tsx`. This satisfies the requirement and establishes the right pattern for Phase 4 when `QueryPlanNode` is extracted to `@quent/components`.

2. **tsconfig.json include array after alias removal**
   - What we know: `ui/tsconfig.json` currently includes `../examples/simulator/server/ts-bindings` so TypeScript can see those files directly.
   - What's unclear: After migration, can this `include` entry be safely removed, or does some app code still directly reference ts-bindings files?
   - Recommendation: After migrating all `~quent/types/*` imports to `@quent/utils`, grep for any remaining direct references to `../examples/simulator/server/ts-bindings` in `ui/src/`. If none found, remove the `include` entry. TypeScript project references will handle cross-package type visibility.

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| CJS package exports | ESM-only (`"type": "module"`) | Project start | tsup output is ESM-only; aligns with Vite's bundler mode |
| Build-first package resolution | Source-first (`main: src/index.ts`) | Phase 1 decision | No tsup build needed for dev; avoids stale dist issues |
| Path aliases for cross-boundary types | Workspace package re-exports | Phase 2 (this phase) | `~quent/types` alias removed; all type imports go through `@quent/utils` |

---

## Sources

### Primary (HIGH confidence)
- Direct source file inspection: `ui/src/lib/utils.ts`, `ui/src/services/colors.ts`, `ui/src/services/formatters.ts`, `ui/src/services/api.ts` — actual content verified
- `ui/packages/@quent/utils/` skeleton files — actual structure verified
- `ui/vite.config.ts`, `ui/tsconfig.json` — alias locations confirmed
- `examples/simulator/server/ts-bindings/` — 52 files confirmed; inter-file relative imports confirmed by grep
- `ui/vitest.config.ts`, `ui/vitest.workspace.ts` — test infrastructure confirmed
- `ui/src/services/api.test.ts` — existing test for `parseJsonWithBigInt` confirmed

### Secondary (MEDIUM confidence)
- pnpm workspace docs (source-first resolution pattern) — consistent with Phase 1 established decisions
- tsup ESM-only pattern — consistent with Phase 1 `tsup.config.ts`

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all packages installed and versions confirmed from `node_modules`
- Architecture: HIGH — based on direct source inspection, not assumptions
- Pitfalls: HIGH — each pitfall derived from actual file content discrepancies (getOperationTypeColor gap, relative path calculation, test import break)
- Open questions: MEDIUM — require planner decision, not further research

**Research date:** 2026-04-01
**Valid until:** 2026-05-01 (stable extraction phase — no external dependencies on fast-moving APIs)
