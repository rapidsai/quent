<!-- SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved. -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

# `@quent/schema-viewer`

Private Svelte 5 custom elements for exploring a generated Quent `Schema`.

The package provides:

- `<quent-entity-graph>` with entity-graph and resource-timeline views.
- `<quent-entity-events>` for entity events.
- `<quent-fsm-details>` for FSM topology and transition attributes.
- `<quent-record-details>` for record fields.
- `<quent-resource-details>` for resource capacities and consumers.

Import the package and its default stylesheet once:

```ts
import '@quent/schema-viewer';
import '@quent/schema-viewer/styles.css';
```

Then pass generated schema bindings directly to the elements:

```svelte
<script lang="ts">
  import type {
    EntityNodeProps,
    Schema,
    SchemaSelection,
  } from '@quent/schema-viewer';

  let { schema }: { schema: Schema } = $props();
  let selection = $state<SchemaSelection | null>(null);
</script>

<quent-entity-graph
  {schema}
  {selection}
  onquent-select={(event) => (selection = event.detail)}
/>
```

Entity node content can be replaced with a Svelte component accepting
`EntityNodeProps`. Graph layout and interaction remain owned by the viewer.

```svelte
<quent-entity-graph {schema} nodeComponent={EntityNode} />
```

## Responsibilities

- Quent constraints are transformed into graph and resource-timeline models.
- ELK computes entity, edge, and FSM geometry.
- `@xyflow/svelte` renders entity and FSM graphs and owns viewport interaction.
- The host application chooses where focused detail elements appear.

Graph nodes are read-only, selectable, and not draggable. Connections are not
editable. `EntityGraphConfig`, `DEFAULT_ENTITY_GRAPH_CONFIG`, and
`resolveEntityGraphConfig` control layout, filtering, labels, and viewport
behavior. Layered layout tuning includes layer assignment, node placement,
layout effort, hierarchy-aware crossing reduction, and high-degree-node
treatment.

The view emits `quent-select`, `quent-hover`, `quent-hover-end`,
`quent-layout-start`, `quent-layout-complete`, `quent-layout-error`, and
`quent-view-change`.

## Styling

Elements render into light DOM. Default selectors use zero specificity.
Consumers may override CSS variables, stable `data-quent-role` selectors, or
the optional class maps without depending on DaisyUI or the example
application.
