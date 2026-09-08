<!-- SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved. -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

<script lang="ts">
  import type { Event } from '@quent/schema';

  import { fieldsOf } from '../../lib/schema';
  import type { SchemaDetailsClasses } from '../../lib/types';
  import DataTypeDisplay from './DataTypeDisplay.svelte';

  interface Props {
    event: Event;
    classes: SchemaDetailsClasses;
    selected?: boolean;
  }

  let { event, classes, selected = false }: Props = $props();
  let fields = $derived(fieldsOf(event.payload));
  let section = $state<HTMLElement>();

  $effect(() => {
    if (!selected || !section) {
      return;
    }
    const frame = requestAnimationFrame(() => {
      section?.scrollIntoView?.({
        behavior: 'smooth',
        block: 'nearest',
      });
    });
    return () => cancelAnimationFrame(frame);
  });

  function classNames(...values: Array<string | undefined | false>): string {
    return values.filter(Boolean).join(' ');
  }
</script>

<section
  bind:this={section}
  class={classNames(
    'quent-schema-details__fsm-attributes',
    selected && 'quent-schema-details__fsm-attributes--selected',
  )}
  data-quent-role="fsm-attributes"
  data-state={event.name}
  data-selected={selected}
>
  <header>
    <h4
      class="quent-schema-name quent-schema-name--title"
      data-quent-role="fsm-state-title"
      data-quent-schema-name="true"
    >
      {event.name}
    </h4>
    <span
      class={classNames(
        'quent-schema-viewer__badge',
        'quent-schema-viewer__badge--fsm',
        classes.badge,
        classes.fsmBadge,
      )}
    >
      State
    </span>
  </header>
  <span
    class={classNames(
      'quent-schema-details__item-meta',
      classes.itemMeta,
    )}
  >
    {event.cardinality}
  </span>
  {#if event.annotations.docs}
    <p>{event.annotations.docs}</p>
  {/if}
  <h5>Attributes</h5>
  {#if fields.length === 0}
    <p class="quent-schema-details__muted">No transition attributes.</p>
  {:else}
    <div class="quent-schema-details__fsm-attribute-list">
      {#each fields as field (field.name)}
        <div
          class={classNames(
            'quent-schema-details__field',
            classes.field,
          )}
          data-quent-role="fsm-attribute"
        >
          <span
            class="quent-schema-name"
            data-quent-schema-name="true"
          >
            {field.name}
          </span>
          <span
            class="quent-schema-details__field-separator"
            aria-hidden="true"
          >
            :
          </span>
          <DataTypeDisplay type={field.ty} {classes} />
        </div>
      {/each}
    </div>
  {/if}
</section>
