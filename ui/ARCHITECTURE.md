<!-- SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved. -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

# UI dependency boundaries

Quent UI code follows a one-way dependency graph:

```text
applications → query-engine-aware widgets → QueryBundle-unaware views → generic UI
       │                    │                         │
       └────────────────────┴─────────────────────────┴──→ utilities
                            └──→ client / hooks / query-engine contracts
```

Dependencies point toward lower layers. Query-engine-aware code may import
unaware views, but unaware views and generic UI cannot import upward.

## Generic UI

Generic UI has no Quent types, generated bindings, API calls, atoms, or domain
concepts in its public API. The currently enforced generic zone is:

```text
packages/@quent/components/src/ui/**
```

`cn` and `clamp` are temporarily imported from `@quent/utils`; ESLint permits
only those two names. `sliderField.utils.ts` is also temporarily allowed until
the generic zone becomes a physical package.

`SegmentedBar`, Timeline, Gantt, and grouped-table primitives are extraction
candidates, but remain outside this zone until any Quent adapters are separated.

## QueryBundle-unaware Quent views

The enforced unaware zone is:

```text
packages/@quent/components/src/quent-views/**
```

These views receive feature-scoped data, controlled state, and actions through
props. They may use stable Quent concepts, but cannot import QueryBundle,
generated wire bindings, `@quent/utils/query-engine`, `@quent/client`,
`@quent/hooks`, or the aware source zone. `quent-views/fsm-chart/**` is the
first migrated slice.

## Query-engine-aware widgets and transforms

The enforced aware zone is:

```text
packages/@quent/components/src/query-engine/**
```

Aware code owns QueryBundle interpretation, API and hook integration, and
conversion into feature-scoped view models. It may import `quent-views/**` and
generic UI. `query-engine/query-plan/**` is the first migrated slice.

Generated QueryBundle and wire contracts have the explicit import surface
`@quent/utils/query-engine`. Existing `@quent/utils` root exports remain for
compatibility, but new aware code should use the dedicated surface.

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
- Dependency Cruiser and ESLint reject aware, client, hook, generated-binding,
  and query-engine-contract imports from `quent-views/**`.

When adding a dependency exception, update this document and the graph rule in
the same change. Exceptions must name a concrete migration boundary; broad
directory exemptions are not allowed.
