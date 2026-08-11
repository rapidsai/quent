<!-- SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved. -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

<script lang="ts">
  import { pathKey } from '../../lib/schema';
  import { selectionMatches } from '../../lib/selection';
  import type {
    ResourceDefinition,
    SchemaDetailsClasses,
    SchemaSelection,
  } from '../../lib/types';

  interface Props {
    resources: ResourceDefinition[];
    selection: SchemaSelection | null;
    classes: SchemaDetailsClasses;
    onSelect: (selection: SchemaSelection) => void;
  }

  let { resources, selection, classes, onSelect }: Props = $props();

  function classNames(...values: Array<string | undefined | false>): string {
    return values.filter(Boolean).join(' ');
  }

  function resourceGroups(resource: ResourceDefinition): Array<{
    role: 'usage' | 'bounds';
    records: ResourceDefinition['usages'];
  }> {
    return [
      { role: 'usage', records: resource.usages },
      { role: 'bounds', records: resource.bounds },
    ];
  }
</script>

<section
  class={classNames('quent-schema-details__section', classes.section)}
  data-quent-role="resources"
>
  {#if resources.length === 0}
    <p class="quent-schema-details__muted">No resource definitions.</p>
  {:else}
    <div class="quent-schema-details__items">
      {#each resources as resource (pathKey(resource.resource))}
        {@const resourceSelection = {
          kind: 'resource',
          resource: resource.resource,
        } as const}
        <article
          class={classNames(
            'quent-schema-details__resource',
            classes.resource,
            selectionMatches(selection, resourceSelection) &&
              'quent-schema-details__item--selected',
            selectionMatches(selection, resourceSelection) &&
              classes.selectedItem,
          )}
          data-quent-role="resource"
          data-resource={pathKey(resource.resource)}
        >
          <button
            type="button"
            class={classNames(
              'quent-schema-details__item-title',
              classes.itemTitle,
            )}
            onclick={() => onSelect(resourceSelection)}
          >
            <span
              class="quent-schema-name quent-schema-name--title"
              data-quent-role="resource-title"
              data-quent-schema-name="true"
            >
              {pathKey(resource.resource)}
            </span>
            <span
              class={classNames(
                'quent-schema-viewer__badge',
                'quent-schema-viewer__badge--resource',
                classes.badge,
                classes.resourceBadge,
              )}
              data-quent-role="resource-badge"
            >
              Resource
            </span>
          </button>
          <div class="quent-schema-details__capacities">
            {#if resource.capacities.length === 0}
              <span
                class={classNames(
                  'quent-schema-details__capacity',
                  classes.capacity,
                )}
              >
                unit resource
              </span>
            {:else}
              {#each resource.capacities as capacity (capacity.name)}
                <span
                  class={classNames(
                    'quent-schema-details__capacity',
                    classes.capacity,
                  )}
                  data-quent-role="capacity"
                >
                  <strong
                    class="quent-schema-name"
                    data-quent-schema-name="true"
                  >
                    {capacity.name}
                  </strong>
                  {capacity.kind}
                  {capacity.bounded ? 'bounded' : 'unbounded'}
                </span>
              {/each}
            {/if}
          </div>
          {#each resourceGroups(resource) as group (group.role)}
            {#each group.records as record (pathKey(record.record))}
              {@const roleSelection = {
                kind: 'resource-record',
                record: record.record,
                resource: resource.resource,
                role: group.role,
              } as const}
              <button
                type="button"
                class={classNames(
                  'quent-schema-details__resource-record',
                  classes.item,
                  selectionMatches(selection, roleSelection) &&
                    'quent-schema-details__item--selected',
                  selectionMatches(selection, roleSelection) &&
                    classes.selectedItem,
                )}
                data-quent-role={`resource-${group.role}`}
                onclick={() => onSelect(roleSelection)}
              >
                <span>
                  {group.role}:
                  <span
                    class="quent-schema-name"
                    data-quent-schema-name="true"
                  >
                    {pathKey(record.record)}
                  </span>
                </span>
                <small>
                  {#if record.fields.length === 0}
                    no explicit capacities
                  {:else}
                    {#each record.fields as fieldName, index (fieldName)}
                      {#if index > 0}, {/if}<span
                        class="quent-schema-name"
                        data-quent-schema-name="true"
                      >{fieldName}</span>
                    {/each}
                  {/if}
                </small>
                {#each record.consumers as consumer (`${pathKey(consumer.entity)}:${consumer.event}:${consumer.fieldPath.join('.')}`)}
                  <small>
                    used by
                    <span
                      class="quent-schema-name"
                      data-quent-schema-name="true"
                    >
                      {pathKey(consumer.entity)}.{consumer.event}
                    </span>
                  </small>
                {/each}
              </button>
            {/each}
          {/each}
        </article>
      {/each}
    </div>
  {/if}
</section>
