<!-- SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved. -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

# UI dependency boundaries

Quent UI code follows a one-way dependency graph:

```text
applications → domain integrations → reusable Quent views → generic UI
       │              │                    │                  │
       └──────────────┴────────────────────┴──────────────────┴──→ utilities
                      └──→ clients / hooks / domain contracts
```

Dependencies point toward lower layers. Domain integrations may import reusable
views, but reusable views and generic UI cannot import upward.

## Generic UI

Generic UI has no Quent types, generated bindings, API calls, atoms, or domain
concepts in its public API. The currently enforced generic zone is:

```text
packages/@quent/components/src/ui/**
```

`cn` and `clamp` are temporarily imported from `@quent/utils`; ESLint permits
only those two names. `sliderField.utils.ts` is also temporarily allowed until
the generic zone becomes a physical package.

`SegmentedBar`, generic legends, `GroupedDataTable`, and domain-free ECharts
bootstrap, resize, wheel, minimum-zoom, and connect helpers now live in this zone.
The controlled Timeline/Gantt rendering split remains the Phase 3 extraction.

## Reusable Quent views

The enforced reusable-view zone is:

```text
packages/@quent/components/src/quent-views/**
```

These views receive feature-scoped data, controlled state, and actions through
props. They may use stable Quent concepts, but cannot import domain contracts,
generated wire bindings, clients, hooks, or domain integration source.
`quent-views/fsm-chart/**` is the first migrated slice. The currently enforced
restrictions name the query-engine contract and integration explicitly.

## Query-engine integration adapters

The currently enforced domain integration zone is:

```text
packages/@quent/components/src/query-engine/**
```

These adapters own QueryBundle interpretation, query-engine API and hook
integration, and conversion into feature-scoped view models. They may import
`quent-views/**` and generic UI. `query-engine/query-plan/**` is the first
migrated slice.

Generated QueryBundle and wire contracts have the explicit import surface
`@quent/utils/query-engine`. Existing `@quent/utils` root exports remain for
compatibility, but new query-engine integration code should use the dedicated
surface.

Future domains may define different analyzer projections and UI APIs. Each
should have a parallel integration adapter rather than extending
`query-engine/**` with unrelated domain concepts.

The package dependency direction remains:

```text
@quent/utils → @quent/client → @quent/hooks → @quent/components
```

An arrow means “may be imported by.” Reverse imports are forbidden.

## Applications

`src/**` and `simulator/**` own routing, URL policy, navigation, application
providers, app-specific composition, and simulator setup. Packages must never
import application code or use the `@/` alias.

Application code imports package roots only. It must not import generated
bindings or `@quent/*/src/**` paths directly.

## Transitional mixed directories

New code belongs in `ui/**`, `quent-views/**`, or `query-engine/**` according to
its contract. The legacy `dag/**`, `query-plan/**`, `timeline/**`,
`resource-tree/**`, `pivot-table/**`, `operator-timeline/**`,
`long-entities/**`, `gantt-chart/**`, and `nvtx-timeline/**` directories remain
mixed while later phases split their rendering and integration responsibilities.

## Enforcement

- `pnpm boundaries:check` validates source and package-to-package dependency
  edges.
- ESLint provides immediate errors for forbidden imports.
- Package typechecks compile each package independently.
- CI runs all three checks before the aggregate build.
- Dependency Cruiser and ESLint reject query-engine integration, client, hook,
  generated-binding, and domain-contract imports from `quent-views/**`.

When adding a dependency exception, update this document and the graph rule in
the same change. Exceptions must name a concrete migration boundary; broad
directory exemptions are not allowed.
