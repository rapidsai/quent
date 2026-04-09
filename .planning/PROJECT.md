# Quent UI Modularization

## What This Is

A refactor of the existing `ui/` React application into a set of well-scoped internal packages living at `ui/packages/*`, within the existing pnpm workspace. The packages expose clear, stable interfaces so that both AI coding agents and human developers can compose new query-engine UIs from atomic building blocks — without needing to understand the full codebase. The existing app remains the primary consumer and continues to function throughout.

## Core Value

Components, state, and API access are each independently importable with zero coupling to the app shell — an agent can read the package exports and assemble a functional UI without reading implementation details.

## Requirements

### Validated

- ✓ React 19 + TypeScript + Jotai + TanStack Query + TanStack Router — existing stack
- ✓ XYFlow DAG visualization with ELK auto-layout — existing
- ✓ ECharts timeline visualization — existing
- ✓ Radix UI + Tailwind CSS v4 + CVA component system — existing
- ✓ pnpm 9+ workspace with Vite + Vitest — existing
- ✓ `@quent/utils` package — cn(), 52 Rust-generated types, formatters, colors, getOperationTypeColor — Validated in Phase 02: extract-quent-utils
- ✓ Existing `ui/src` app migrated from legacy utils imports to `@quent/utils` — Validated in Phase 02: extract-quent-utils
- ✓ `@quent/client` package — 6 fetch functions, 6 queryOptions factories, 5 pure TanStack Query hooks, DEFAULT_STALE_TIME — Validated in Phase 03: extract-quent-client-and-quent-hooks
- ✓ `@quent/hooks` package — all Jotai atoms internal; 28+ named hooks; atomFamily replaced with record-based atoms; jotai-family removed — Validated in Phase 03: extract-quent-client-and-quent-hooks
- ✓ Existing `ui/src` app migrated from @/services/api and @/atoms/* to @quent/client and @quent/hooks — Validated in Phase 03: extract-quent-client-and-quent-hooks

### Active

- [ ] `@quent/components` package — UI component library (DAG chart, timeline, query plan tree, node detail, resource tree, common UI primitives) with shadcn-style design, clear props interfaces, and no internal state coupling
- [ ] Existing `ui/src` app migrated to consume only from packages — no direct imports of internal modules that have been extracted
- [ ] Each package has a clean `index.ts` barrel export listing everything an agent would need
- [ ] Packages designed for eventual npm publishability (no app-shell coupling, no relative paths to app internals)

### Out of Scope

- Per-package versioning or changelogs — single workspace lock, no independent semver until publish decision is made
- Publishing to npm in this milestone — design for it, but don't execute
- Domain-granular packages (e.g. separate dag package from timeline) — coarse split keeps management simple; can refine later
- New UI features — this is a structural refactor, not a feature sprint
- Backend changes — Rust crates are out of scope entirely

## Context

**Codebase:** Full-stack Rust + TypeScript monorepo. Frontend is a single Vite app in `ui/src/` consuming a Rust analyzer API. TypeScript types are generated from Rust structs via `ts-rs` and live in `crates/server/ts-bindings/` and `examples/simulator/server/ts-bindings/`.

**Current state (Phase 3 complete):** `@quent/utils`, `@quent/client`, and `@quent/hooks` fully extracted. App imports route through packages. Only `@quent/components` and final app-shell migration remain (Phase 4).

**Motivation:** An agent tasked with building a new visualization or adapting the UI for a new domain currently has to trace across the entire codebase. The goal is to make the component/state/data layers independently legible — a package's `index.ts` should be a complete API surface.

**Key UI domains:** DAG visualization (XYFlow + ELK), timeline charts (ECharts), query plan tree (recursive tree), resource usage tree, operator node detail view.

**State architecture:** Jotai atoms track: selected DAG node, selected plan, hovered workers — all cross-cutting concerns that multiple components react to. TanStack Query handles all server state.

## Constraints

- **Monorepo:** Must stay as single repo — no splitting packages into separate repos
- **No versioning churn:** `workspace:*` protocol throughout; packages not independently versioned until npm publish decision
- **Zero breakage:** Existing app must work throughout the refactor — incremental extraction, not a rewrite
- **Publishability-ready:** Package boundaries, exports, and naming should be clean enough to publish to npm later without major rework
- **pnpm workspace:** Packages live at `ui/packages/*`, configured in `ui/pnpm-workspace.yaml`

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| `ui/packages/*` location (not top-level `packages/`) | Keeps frontend workspace self-contained; no interference with Rust workspace | — Pending |
| Coarse 4-package split (components, hooks, client, utils) | Avoids versioning complexity of fine-grained packages; easy for agents to reason about | — Pending |
| Hooks wrap atoms (no raw Jotai exports) | Hides atom implementation from consumers; can swap state library without breaking API | — Pending |
| Design for publishability but don't publish | Avoids npm overhead now while preserving the option | — Pending |

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd:transition`):
1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone** (via `/gsd:complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-04-09
