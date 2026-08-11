<!-- SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved. -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

<svelte:options customElement={{ tag: 'quent-entity-graph', shadow: 'none' }} />

<script lang="ts">
  import type { Schema } from '@quent/schema';

  import { resolveEntityGraphConfig } from '../lib/config';
  import { buildEntityGraph } from '../lib/schema';
  import type {
    EntityGraphClasses,
    EntityGraphConfig,
    EntityGraphLayoutComplete,
    EntityGraphLayoutError,
    EntityGraphLayoutStart,
    EntityGraphView,
    EntityGraphViewChange,
    EntityNodeComponent,
    SchemaSelection,
  } from '../lib/types';
  import { ENTITY_GRAPH_VIEW_REGISTRY } from '../lib/viewRegistry';

  interface Props {
    schema?: Schema | null;
    selection?: SchemaSelection | null;
    classes?: EntityGraphClasses;
    config?: EntityGraphConfig;
    nodeComponent?: EntityNodeComponent | null;
  }

  let {
    schema = null,
    selection = null,
    classes = {},
    config = {},
    nodeComponent = null,
  }: Props = $props();
  let view = $state<EntityGraphView>('graph');
  let model = $derived(buildEntityGraph(schema));
  let resolvedConfig = $derived(resolveEntityGraphConfig(config));

  function emit<Detail>(name: string, detail: Detail): void {
    $host().dispatchEvent(
      new CustomEvent<Detail>(name, {
        detail,
        bubbles: true,
        composed: true,
      }),
    );
  }

  function setView(next: EntityGraphView): void {
    if (next === view) {
      return;
    }
    view = next;
    emit<EntityGraphViewChange>('quent-view-change', { view: next });
  }
</script>

<div data-quent-component="entity-graph">
  {#if !schema || model.nodes.length === 0}
    <div
      class={`quent-schema-viewer__empty ${classes.empty ?? ''}`}
      data-quent-role="empty"
    >
      No entities in this schema.
    </div>
  {:else}
    {#if resolvedConfig.showViewSwitcher}
      <div
        class={`quent-schema-view-switcher ${classes.viewSwitcher ?? ''}`}
        data-quent-role="view-switcher"
        role="tablist"
        aria-label="Schema explorer view"
      >
        {#each ENTITY_GRAPH_VIEW_REGISTRY as candidate (candidate.id)}
          <button
            type="button"
            role="tab"
            aria-selected={candidate.id === view}
            class={[
              'quent-schema-view-switcher__option',
              candidate.id === view &&
                'quent-schema-view-switcher__option--active',
              classes.viewOption,
              candidate.id === view && classes.activeViewOption,
            ].filter(Boolean).join(' ')}
            data-quent-role={`view-${candidate.id}`}
            onclick={() => setView(candidate.id)}
          >
            {candidate.label}
          </button>
        {/each}
      </div>
    {/if}

    <div class="quent-schema-view__content">
      {#each ENTITY_GRAPH_VIEW_REGISTRY as candidate (candidate.id)}
        {@const ViewComponent = candidate.component}
        <div
          class="quent-schema-view__panel"
          data-quent-view-panel={candidate.id}
          hidden={candidate.id !== view}
          aria-hidden={candidate.id !== view}
        >
          <ViewComponent
            {schema}
            {model}
            active={candidate.id === view}
            {selection}
            config={resolvedConfig}
            {classes}
            {nodeComponent}
            onSelect={(value: SchemaSelection) =>
              emit<SchemaSelection>('quent-select', value)}
            onHover={(value: SchemaSelection) =>
              emit<SchemaSelection>('quent-hover', value)}
            onHoverEnd={() => emit('quent-hover-end', undefined)}
            onLayoutStart={(detail: EntityGraphLayoutStart) =>
              emit<EntityGraphLayoutStart>('quent-layout-start', detail)}
            onLayoutComplete={(detail: EntityGraphLayoutComplete) =>
              emit<EntityGraphLayoutComplete>(
                'quent-layout-complete',
                detail,
              )}
            onLayoutError={(detail: EntityGraphLayoutError) =>
              emit<EntityGraphLayoutError>('quent-layout-error', detail)}
          />
        </div>
      {/each}
    </div>
  {/if}
</div>
