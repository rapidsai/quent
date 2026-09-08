<!-- SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved. -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

<svelte:options customElement={{ tag: 'quent-entity-events', shadow: 'none' }} />

<script lang="ts">
  import type { Path, Schema } from '@quent/schema';

  import { samePath } from '../lib/schema';
  import type {
    SchemaDetailsClasses,
    SchemaSelection,
  } from '../lib/types';
  import EntityEventsSection from './details/EntityEventsSection.svelte';

  interface Props {
    schema?: Schema | null;
    path?: Path | null;
    selection?: SchemaSelection | null;
    classes?: SchemaDetailsClasses;
  }

  let {
    schema = null,
    path = null,
    selection = null,
    classes = {},
  }: Props = $props();
  let entities = $derived(
    schema?.entities
      .map(([, entity]) => entity)
      .filter((entity) => !path || samePath(entity.path, path)) ?? [],
  );

  function emitSelection(detail: SchemaSelection): void {
    $host().dispatchEvent(
      new CustomEvent<SchemaSelection>('quent-select', {
        detail,
        bubbles: true,
        composed: true,
      }),
    );
  }
</script>

<div data-quent-component="entity-events">
  {#if !schema}
    <div class="quent-schema-viewer__empty" data-quent-role="empty">
      No schema selected.
    </div>
  {:else}
    <EntityEventsSection
      {entities}
      {selection}
      {classes}
      eventName={selection?.kind === 'event' ? selection.event : null}
      onSelect={emitSelection}
    />
  {/if}
</div>
