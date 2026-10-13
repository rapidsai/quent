<!-- SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved. -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

# UI dependency boundaries

Quent UI code follows a one-way dependency graph:

```text
applications → Quent widgets → Quent views → generic UI
                    ↓               ↓
             client / hooks → types / utilities
```

Dependencies may only point to the right or down. Lower layers never import
applications or higher-level presentation layers.

## Generic UI

Generic UI has no Quent types, generated bindings, API calls, atoms, or domain
concepts in its public API. The currently enforced generic zone is:

```text
packages/@quent/components/src/ui/**
```

`cn` and `clamp` are temporarily imported from `@quent/utils`; ESLint permits
only those two names. `sliderField.utils.ts` is also temporarily allowed until
the generic zone becomes a physical package.

Timeline, Gantt, and grouped-table primitives are extraction candidates, but
remain outside this zone until their Quent state adapters are separated.

## Quent views and widgets

Quent views may use Quent model types, but reusable views should receive data,
state, and actions through props. Widgets may bind those views to
`@quent/client` and `@quent/hooks`.

The current `@quent/components` package contains both roles. Keep imports
flowing through the existing package stack:

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

## Enforcement

- `pnpm boundaries:check` validates source and package-to-package dependency edges.
- ESLint provides immediate errors for forbidden imports.
- Package typechecks compile each package independently.
- CI runs all three checks before the aggregate build.

When adding a dependency exception, update this document and the graph rule in
the same change. Exceptions must name a concrete migration boundary; broad
directory exemptions are not allowed.
