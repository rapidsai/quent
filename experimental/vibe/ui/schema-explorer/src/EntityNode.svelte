<!-- SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved. -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

<script lang="ts">
  import {
    samePath,
    type EntityNodeProps,
  } from '@quent/schema-viewer';

  let { schema, path }: EntityNodeProps = $props();
  let entity = $derived(
    schema.entities.find(([entityPath]) => samePath(entityPath, path))?.[1],
  );
  let eventNames = $derived(Object.keys(entity?.events ?? {}));
  let visibleEvents = $derived(eventNames.slice(0, 2));
  let remainingEvents = $derived(eventNames.length - visibleEvents.length);
</script>

<strong
  class="quent-entity-graph__node-title quent-schema-name quent-schema-name--title"
  data-quent-schema-name="true"
>
  {path.name}
</strong>
<small class="quent-entity-graph__node-meta">
  {#if visibleEvents.length === 0}
    No events
  {:else}
    {#each visibleEvents as eventName, index (eventName)}
      {#if index > 0}<span
          class="quent-entity-graph__event-separator"
          aria-hidden="true"
        >,</span>{/if}<span
        class="quent-schema-name"
        data-quent-schema-name="true"
      >{eventName}</span>
    {/each}
    {#if remainingEvents > 0}, … {remainingEvents} more{/if}
  {/if}
</small>
