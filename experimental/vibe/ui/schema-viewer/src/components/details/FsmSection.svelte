<!-- SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved. -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

<script lang="ts">
  import type { Entity } from '@quent/schema';

  import { parseFsm, pathKey } from '../../lib/schema';
  import type {
    SchemaDetailsClasses,
    SchemaSelection,
  } from '../../lib/types';
  import FsmTopologyGraph from './FsmTopologyGraph.svelte';
  import FsmTransitionAttributes from './FsmTransitionAttributes.svelte';

  interface Props {
    entities: Entity[];
    selection: SchemaSelection | null;
    classes: SchemaDetailsClasses;
    isolateState: boolean;
    onSelect: (selection: SchemaSelection) => void;
  }

  let {
    entities,
    selection,
    classes,
    isolateState,
    onSelect,
  }: Props = $props();
  let fsms = $derived(fsmEntities(entities));
  let selectedState = $derived.by(() => {
    if (!isolateState || selection?.kind !== 'fsm-state') {
      return null;
    }
    const fsm = fsms.find(({ entity }) =>
      pathKey(entity.path) === pathKey(selection.entity));
    const event = fsm?.entity.events[selection.state];
    return event ? { event } : null;
  });

  function classNames(...values: Array<string | undefined | false>): string {
    return values.filter(Boolean).join(' ');
  }

  function fsmEntities(values: Entity[]): Array<{
    entity: Entity;
    topology: NonNullable<ReturnType<typeof parseFsm>>;
  }> {
    return values.flatMap((entity) => {
      const topology = parseFsm(entity);
      return topology ? [{ entity, topology }] : [];
    });
  }

</script>

<section
  class={classNames('quent-schema-details__section', classes.section)}
  data-quent-role="fsms"
>
  {#if fsms.length === 0}
    <p class="quent-schema-details__muted">
      No FSM topology for the selected entities.
    </p>
  {:else}
    {#if selectedState}
      <FsmTransitionAttributes
        event={selectedState.event}
        {classes}
        selected
      />
    {:else}
      {#each fsms as { entity, topology } (pathKey(entity.path))}
        <div
          class={classNames('quent-schema-details__fsm', classes.fsm)}
          data-quent-role="fsm"
          data-entity={pathKey(entity.path)}
        >
          <h3>
            <span
              class="quent-schema-name quent-schema-name--title"
              data-quent-role="fsm-title"
              data-quent-schema-name="true"
            >
              {pathKey(entity.path)}
            </span>
            <span
              class={classNames(
                'quent-schema-viewer__badge',
                'quent-schema-viewer__badge--fsm',
                classes.badge,
                classes.fsmBadge,
              )}
              data-quent-role="fsm-badge"
            >
              FSM
            </span>
          </h3>
          <FsmTopologyGraph
            {entity}
            {topology}
            {selection}
            {classes}
            {onSelect}
          />
        </div>
      {/each}
    {/if}
  {/if}
</section>
