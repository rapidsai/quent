<!-- SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved. -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

<script lang="ts">
  import type { Entity } from '@quent/schema';

  import { fieldsOf, pathKey } from '../../lib/schema';
  import { selectionMatches } from '../../lib/selection';
  import type {
    SchemaDetailsClasses,
    SchemaSelection,
  } from '../../lib/types';
  import DataTypeDisplay from './DataTypeDisplay.svelte';

  interface Props {
    entities: Entity[];
    selection: SchemaSelection | null;
    classes: SchemaDetailsClasses;
    onSelect: (selection: SchemaSelection) => void;
    eventName?: string | null;
  }

  let {
    entities,
    selection,
    classes,
    onSelect,
    eventName = null,
  }: Props = $props();

  function classNames(...values: Array<string | undefined | false>): string {
    return values.filter(Boolean).join(' ');
  }
</script>

<section
  class={classNames('quent-schema-details__section', classes.section)}
  data-quent-role="events"
>
  {#each entities as entity (pathKey(entity.path))}
    <div class="quent-schema-details__entity">
      <button
        type="button"
        class={classNames(
          'quent-schema-details__entity-title',
          classes.itemTitle,
        )}
        onclick={() =>
          onSelect({ kind: 'entity', entity: entity.path })}
      >
        <span
          class="quent-schema-name quent-schema-name--title"
          data-quent-role="entity-title"
          data-quent-schema-name="true"
        >
          {pathKey(entity.path)}
        </span>
        <span
          class={classNames(
            'quent-schema-viewer__badge',
            'quent-schema-viewer__badge--entity',
            classes.badge,
            classes.entityBadge,
          )}
          data-quent-role="entity-badge"
        >
          Entity
        </span>
      </button>
      <div class="quent-schema-details__items">
        {#each Object.values(entity.events).filter((event) => !eventName || event.name === eventName) as event (event.name)}
          {@const eventSelection = {
            kind: 'event',
            entity: entity.path,
            event: event.name,
          } as const}
          <button
            type="button"
            class={classNames(
              'quent-schema-details__item',
              classes.item,
              selectionMatches(selection, eventSelection) &&
                'quent-schema-details__item--selected',
              selectionMatches(selection, eventSelection) &&
                classes.selectedItem,
            )}
            data-quent-role="event"
            data-event={`${pathKey(entity.path)}.${event.name}`}
            onclick={() => onSelect(eventSelection)}
          >
            <span
              class={classNames(
                'quent-schema-details__item-title',
                classes.itemTitle,
              )}
            >
              <span
                class="quent-schema-name quent-schema-name--title"
                data-quent-role="event-title"
                data-quent-schema-name="true"
              >
                {event.name}
              </span>
              <span
                class={classNames(
                  'quent-schema-viewer__badge',
                  'quent-schema-viewer__badge--entity',
                  classes.badge,
                  classes.entityBadge,
                )}
                data-quent-role="event-badge"
              >
                Event
              </span>
            </span>
            <span
              class={classNames(
                'quent-schema-details__item-meta',
                classes.itemMeta,
              )}
            >
              {event.cardinality}
            </span>
            {#each fieldsOf(event.payload) as field (field.name)}
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
    </div>
  {/each}
</section>
