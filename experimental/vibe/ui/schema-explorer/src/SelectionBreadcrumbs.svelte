<!-- SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved. -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

<script lang="ts">
  import {
    pathKey,
    referenceLabel,
    type SchemaSelection,
  } from '@quent/schema-viewer';

  interface Props {
    selection: SchemaSelection;
    entityKind: 'Entity' | 'FSM';
    onSelect: (selection: SchemaSelection) => void;
    onPreview: (selection: SchemaSelection) => void;
    onPreviewEnd: () => void;
  }

  interface Breadcrumb {
    label: string;
    kind: string;
    selection?: SchemaSelection;
  }

  let {
    selection,
    entityKind,
    onSelect,
    onPreview,
    onPreviewEnd,
  }: Props = $props();
  let breadcrumbs = $derived(buildBreadcrumbs(selection, entityKind));

  function buildBreadcrumbs(
    value: SchemaSelection,
    selectedEntityKind: 'Entity' | 'FSM',
  ): Breadcrumb[] {
    switch (value.kind) {
      case 'entity':
        return [{
          label: pathKey(value.entity),
          kind: selectedEntityKind,
        }];
      case 'event':
        return [
          {
            label: pathKey(value.entity),
            kind: selectedEntityKind,
            selection: { kind: 'entity', entity: value.entity },
          },
          { label: value.event, kind: 'Event' },
        ];
      case 'fsm-state':
        return [
          {
            label: pathKey(value.entity),
            kind: 'FSM',
            selection: { kind: 'entity', entity: value.entity },
          },
          { label: value.state, kind: 'State' },
        ];
      case 'record':
        return [{ label: pathKey(value.record), kind: 'Record' }];
      case 'reference':
        return [
          {
            label: pathKey(value.reference.source),
            kind: selectedEntityKind,
            selection: {
              kind: 'entity',
              entity: value.reference.source,
            },
          },
          { label: referenceLabel(value.reference), kind: 'Reference' },
        ];
      case 'resource':
        return [{ label: pathKey(value.resource), kind: 'Resource' }];
      case 'resource-record':
        return [
          {
            label: pathKey(value.resource),
            kind: 'Resource',
            selection: {
              kind: 'resource',
              resource: value.resource,
            },
          },
          { label: pathKey(value.record), kind: 'Record' },
        ];
    }
  }
</script>

<nav
  class="breadcrumbs min-w-0 overflow-x-auto border-b border-base-300 px-3 py-2 text-xs"
  aria-label="Selection path"
>
  <ul class="items-stretch">
    {#each breadcrumbs as breadcrumb, index (`${breadcrumb.kind}:${breadcrumb.label}`)}
      <li class="min-w-0 items-center">
        {#if breadcrumb.selection}
          <button
            class="grid min-h-14 max-w-56 min-w-28 grid-rows-[1fr_auto] items-center justify-items-start gap-1 rounded-field border border-base-300 bg-base-200/50 px-2.5 py-1.5 text-left hover:border-primary/40 hover:bg-base-200"
            type="button"
            onclick={() => onSelect(breadcrumb.selection!)}
            onpointerenter={() => onPreview(breadcrumb.selection!)}
            onpointerleave={onPreviewEnd}
            onfocus={() => onPreview(breadcrumb.selection!)}
            onblur={onPreviewEnd}
          >
            <span
              class="quent-schema-name quent-schema-name--title w-full self-end truncate"
              data-quent-schema-name="true"
            >
              {breadcrumb.label}
            </span>
            <span class="badge badge-ghost badge-xs">
              {breadcrumb.kind}
            </span>
          </button>
        {:else}
          <span
            class="grid min-h-14 max-w-64 min-w-28 grid-rows-[1fr_auto] items-center justify-items-start gap-1 rounded-field border border-primary/30 bg-primary/5 px-2.5 py-1.5"
            aria-current={index === breadcrumbs.length - 1
              ? 'page'
              : undefined}
          >
            <span
              class="quent-schema-name quent-schema-name--title w-full self-end truncate"
              data-quent-schema-name="true"
            >
              {breadcrumb.label}
            </span>
            <span class="badge badge-ghost badge-xs">
              {breadcrumb.kind}
            </span>
          </span>
        {/if}
      </li>
    {/each}
  </ul>
</nav>
