<!-- SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved. -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

<script lang="ts">
  import type { Record as SchemaRecord } from '@quent/schema';

  import { fieldsOf, pathKey } from '../../lib/schema';
  import { selectionMatches } from '../../lib/selection';
  import type {
    SchemaDetailsClasses,
    SchemaSelection,
  } from '../../lib/types';
  import DataTypeDisplay from './DataTypeDisplay.svelte';

  interface Props {
    records: SchemaRecord[];
    selection: SchemaSelection | null;
    classes: SchemaDetailsClasses;
    onSelect: (selection: SchemaSelection) => void;
  }

  let { records, selection, classes, onSelect }: Props = $props();

  function classNames(...values: Array<string | undefined | false>): string {
    return values.filter(Boolean).join(' ');
  }
</script>

<section
  class={classNames('quent-schema-details__section', classes.section)}
  data-quent-role="records"
>
  {#if records.length === 0}
    <p class="quent-schema-details__muted">No record types.</p>
  {:else}
    <div class="quent-schema-details__items">
      {#each records as record (pathKey(record.path))}
        {@const recordSelection = {
          kind: 'record',
          record: record.path,
        } as const}
        <button
          type="button"
          class={classNames(
            'quent-schema-details__item',
            classes.item,
            selectionMatches(selection, recordSelection) &&
              'quent-schema-details__item--selected',
            selectionMatches(selection, recordSelection) &&
              classes.selectedItem,
          )}
          data-quent-role="record"
          data-record={pathKey(record.path)}
          onclick={() => onSelect(recordSelection)}
        >
          <span
            class={classNames(
              'quent-schema-details__item-title',
              classes.itemTitle,
            )}
          >
            <span
              class="quent-schema-name quent-schema-name--title"
              data-quent-role="record-title"
              data-quent-schema-name="true"
            >
              {pathKey(record.path)}
            </span>
            <span
              class={classNames(
                'quent-schema-viewer__badge',
                'quent-schema-viewer__badge--record',
                classes.badge,
                classes.recordBadge,
              )}
              data-quent-role="record-badge"
            >
              Record
            </span>
          </span>
          {#each fieldsOf(record.fields) as field (field.name)}
            <span
              class={classNames(
                'quent-schema-details__field',
                classes.field,
              )}
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
            </span>
          {/each}
        </button>
      {/each}
    </div>
  {/if}
</section>
