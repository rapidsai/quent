<!-- SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved. -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

# Quent UI — Review Foundation

Foundation for reviewing Quent UI changes. Prefer existing workspace libraries
and package boundaries over new dependencies or one-off app-shell code.

## Goals

1. **Reuse foundational libraries first** — React, Tailwind, shadcn/ui (Radix),
   TanStack (Router / Query / Table / Virtual), and Jotai cover most UI needs.
   Do not introduce a parallel library or hand-roll what these already provide.
2. **Keep packages modular** — Reusable logic lives in `packages/@quent/*` so it
   can ship outside this app. The app shell (`ui/src/`) is routing, layout, and
   glue only.
3. **Keep client and server APIs aligned** — Every HTTP route the UI calls has a
   matching `fetch*` in `@quent/client`, typed against generated bindings.
4. **Server-serialized types come from ts-bindings** — Never duplicate Rust
   request/response shapes as hand-written TypeScript interfaces.

## Foundational stack (prefer these)

| Concern                 | Use                                                                          | Avoid                                                          |
| ----------------------- | ---------------------------------------------------------------------------- | -------------------------------------------------------------- |
| Components / primitives | React 19 + shadcn/ui in `@quent/components/src/ui/` (Radix + `lucide-react`) | New UI kits, ad-hoc primitive wrappers                         |
| Styling                 | Tailwind 4 + `cn()` from `@quent/utils`                                      | String-concatenated classNames, inline style sprawl, CSS-in-JS |
| Routing                 | TanStack Router file routes under `ui/src/routes/`                           | React Router / custom history hacks                            |
| Server state            | TanStack Query via `@quent/client` `queryOptions` / hooks                    | Ad-hoc `useEffect` + `fetch`, React Context as a data store    |
| UI / interaction state  | Jotai atoms (in `@quent/hooks` or app `atoms/`)                              | Context stores for selection/zoom/viewport                     |
| Large lists / tables    | `@tanstack/react-table` + `@tanstack/react-virtual`                          | Rendering full trace/event arrays unvirtualized                |
| Charts                  | ECharts via `@quent/components` helpers                                      | Parallel chart libs for the same timelines                     |

### Library best practices

- **TanStack Query**: wrap fetches in
  `queryOptions({ queryKey, queryFn, staleTime, enabled })` with stable
  `queryKey` arrays. Prefer shared options factories in `@quent/client` over
  inline `useQuery` in components.
- **Jotai**: atoms only from React components/hooks/providers — never from plain
  utils. Server data stays in Query; atoms hold UI interaction state (selection,
  zoom, expanded rows).
- **TanStack Router**: route params are navigation source of truth; prefetch
  with loaders + shared `queryOptions` when useful. Do not hand-edit
  `routeTree.gen.ts`.
- **shadcn/ui**: add new primitives under `@quent/components/src/ui/` and export
  from the package root. Compose with `cn()` / CVA; keep reusable default styles
  and variants in the primitive instead of repeating structural classes at call
  sites.
- **Tailwind**: use design tokens / CSS variables in `ui/src/index.css`. Package
  classes are scanned via `@source` — consumers must include `@quent/*` sources
  when embedding. Prefer theme scale utilities or explicit arbitrary values over
  undefined class names.
- **BigInt / u64**: parse API JSON with `parseJsonWithBigInt` from
  `@quent/utils`, never plain `JSON.parse`. Keep values as `bigint` through
  formatting/calculation unless conversion to `number` is proven safe.

## Workspace structure

```text
ui/
├── src/                      # App shell only (routes, pages, nav, theme glue)
├── packages/@quent/
│   ├── utils/                # Foundation: cn, BigInt JSON, types re-exports
│   ├── client/               # fetch* + queryOptions + thin Query hooks
│   ├── hooks/                # Jotai atoms, QuentProvider orchestration
│   └── components/           # Visualizations + shadcn primitives
└── examples/                 # Opt-in consumers; NOT root workspace members
```

Dependency direction (do not invert):

```text
@quent/utils → @quent/client → @quent/hooks → @quent/components
```

- Put reusable code in the lowest package that fits; keep app-specific wiring in
  `ui/src/`.
- Import from package roots only (`@quent/components`), never deep paths.
- Cross-package imports use package roots; inside a package, use relative
  sibling imports.
- The `@/` alias is app-shell only (`ui/src/`). `@quent/*` packages do not
  define it.
- Cross-package deps use `"workspace:*"`. Catalogued libs use `"catalog:"` in
  deps/devDeps; declare peer ranges that stay in major lockstep with the
  workspace catalog.
- New shared UI belongs in `@quent/*`, not a one-off under `ui/src/components/`
  unless it is truly app-shell-only (nav chrome, route layout).
- Search before adding helpers; consolidate duplicate utilities and tests into
  the lowest reusable package, usually `@quent/utils`.

## Client API ↔ server API

Server routes for the analyzer UI live primarily in
`domains/query_engine/server/src/ui.rs` (mounted under `/api/engines`).

Client counterparts live in `ui/packages/@quent/client/src/` (`api.ts` +
`*QueryOptions` modules). Base URL via `getApiBaseUrl()` / `setApiBaseUrl()`.

When reviewing:

- New/changed server routes → matching `fetch*` + `queryOptions` (and hook if
  needed).
- New/changed client endpoints → verify path, method, and body match the Rust
  handler.
- Prefer bulk timeline APIs (`useBulkTimelineFetch` / bulk endpoints) over N+1
  single-timeline calls.
- Feature-unavailable responses (e.g. HTTP 501 for unsupported data-flow) should
  be handled explicitly, not treated as generic failures when the product
  expects null/empty.
- Scope response data and metadata to the requested engine/query/filter. Reject
  unknown requested values instead of silently accepting them.
- Avoid sentinel strings that can collide with real identifiers; use typed
  variants, opaque IDs, or another collision-free representation.

## React and component correctness

- Async effects must discard stale completions; only the latest
  layout/fetch/calculation may commit state.
- Timers, subscriptions, DOM listeners, and synchronized chart state must clean
  up on dependency changes, disablement, missing data, and unmount. Returning
  `null` does not unmount the component.
- Preserve semantic accessibility: native interactive elements, labeled
  controls, keyboard behavior, and scoped row/column headers for data tables.
- Interactive affordances such as `cursor-pointer` must match the real clickable
  and keyboard-operable target.
- Keep comments terse and contract-focused; do not narrate implementation
  mechanics.

## Tests

- Test observable behavior and meaningful boundaries: fallback/unknown inputs,
  empty/error states, and both `number` and `bigint` precision-sensitive paths
  (mirror both when code accepts either).
- Avoid duplicate or trivial cases that only restate implementation details.
- Build fixtures from canonical production or generated types with `Pick`,
  `Partial`, or shared builders instead of hand-written lookalike interfaces.

## ts-bindings (server-serialized types)

- Generated by ts-rs into `ui/generated/ts-bindings/` (do not
  hand-edit).
- UI consumes them via `@quent/utils` re-exports
  (`import type { … } from '@quent/utils'`).
- Changing a Rust `Serialize`/`Deserialize` API type: update the Rust type with
  `#[derive(TS)]`, regenerate bindings with
  `cargo run -p quent-simulator-ui-bindings`, and re-export from `@quent/utils`
  if the type is newly public.
- Do not invent parallel interfaces for request/response payloads in app or
  package code.
- FE-only view models are fine as local types; anything that crosses the wire
  must use bindings.

## Dependency updates

- Shared versions live in `ui/pnpm-workspace.yaml` `catalog:`. Use `"catalog:"`
  for catalogued deps/devDeps instead of inlining version ranges.
- Prefer in-range bumps by refreshing `pnpm-lock.yaml` when the catalog range
  already allows the target.
- Do not add `overrides` / `pnpm.overrides` when the desired version is already
  in range — update the lockfile instead.
- Change catalog ranges, `package.json` ranges, or overrides only when the
  needed version is outside the existing range, or when an override is the only
  viable fix (and say why).
- When a catalog entry's major version changes, keep every `@quent/*`
  `peerDependencies` range for that package in major lockstep with the catalog
  (at least `^<major>.0.0`). Do not leave peers on an older major than catalog.

## Generated files

Never hand-edit:

- `ui/src/routeTree.gen.ts` (TanStack Router)
- `ui/generated/ts-bindings/**` (ts-rs)

Fix the generator or source instead.
