<!-- SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved. -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

<svelte:options customElement={{ tag: 'quent-resource-details', shadow: 'none' }} />

<script lang="ts">
  import type { Path, Schema } from '@quent/schema';

  import { buildResources, samePath } from '../lib/schema';
  import type {
    SchemaDetailsClasses,
    SchemaSelection,
  } from '../lib/types';
  import ResourcesSection from './details/ResourcesSection.svelte';

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
  let resources = $derived(
    buildResources(schema).filter(
      (resource) => !path || samePath(resource.resource, path),
    ),
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

<div data-quent-component="resource-details">
  {#if !schema}
    <div class="quent-schema-viewer__empty" data-quent-role="empty">
      No schema selected.
    </div>
  {:else}
    <ResourcesSection
      {resources}
      {selection}
      {classes}
      onSelect={emitSelection}
    />
  {/if}
</div>
