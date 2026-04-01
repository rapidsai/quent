# Phase 2: Extract @quent/utils - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-04-01
**Phase:** 02-extract-quent-utils
**Mode:** --auto (all gray areas auto-resolved with recommended defaults)
**Areas discussed:** Type re-export strategy, Color utilities scope, parseJsonWithBigInt extraction, Migration scope

---

## Type Re-export Strategy

| Option | Description | Selected |
|--------|-------------|----------|
| Barrel re-export from existing path | Re-export all 52 ts-binding files via relative path in package barrel; bindings stay at `examples/simulator/server/ts-bindings/` | ✓ |
| Copy types into package src/ | Copy all 52 files into `@quent/utils/src/types/`; package is fully self-contained | |
| Keep alias, route through package | Keep `~quent/types` alias but proxy through package index | |

**Auto-selected:** Barrel re-export (recommended default)
**Notes:** Simplest approach; single source of truth for generated types; publishability concern noted and deferred to v2.

---

## Color Utilities Scope

| Option | Description | Selected |
|--------|-------------|----------|
| Include all of colors.ts | All functions including canvas pattern functions go in @quent/utils | ✓ |
| Exclude canvas functions | createStripePattern, createDotPattern, createCrosshatchPattern deferred to @quent/components | |

**Auto-selected:** Include all (recommended default)
**Notes:** @quent/utils is browser-consumed; keeping color module together is simpler. Canvas functions annotated as browser-only in JSDoc.

---

## parseJsonWithBigInt Extraction

| Option | Description | Selected |
|--------|-------------|----------|
| Extract to @quent/utils; api.ts re-imports | Clean extraction now; api.ts imports from @quent/utils | ✓ |
| Leave in api.ts until Phase 3 | Extract together with the rest of api.ts content | |

**Auto-selected:** Extract now (recommended default)
**Notes:** Establishes @quent/client → @quent/utils dependency direction for Phase 3.

---

## Migration Scope

| Option | Description | Selected |
|--------|-------------|----------|
| Full extraction + migration in Phase 2 | All app imports updated to @quent/utils in this phase | ✓ |
| Extract only, defer migration to Phase 4 | Package created but app imports unchanged | |

**Auto-selected:** Full migration (recommended default)
**Notes:** Phase 2 success criteria explicitly requires app imports to resolve through the package.

---

## Claude's Discretion

- Whether to delete source files after migration or convert to thin re-exports
- Exact barrel export structure (flat vs grouped)
- Whether to use one or multiple plan files

## Deferred Ideas

- Publishability: bundling/copying types when npm publish is executed (Phase V2 scope)
