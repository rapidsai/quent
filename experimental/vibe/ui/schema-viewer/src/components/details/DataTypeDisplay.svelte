<!-- SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved. -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

<script lang="ts">
  import type { DataType } from '@quent/schema';

  import {
    dataTypeParts,
    formatDataType,
  } from '../../lib/schema';
  import type { SchemaDetailsClasses } from '../../lib/types';

  interface Props {
    type: DataType;
    classes: SchemaDetailsClasses;
  }

  let { type, classes }: Props = $props();
  let parts = $derived(dataTypeParts(type));
  let reference = $derived(
    parts.some((part) => part.kind === 'reference'),
  );

  function classNames(...values: Array<string | undefined | false>): string {
    return values.filter(Boolean).join(' ');
  }
</script>

<span
  class={classNames(
    'quent-schema-details__field-type',
    reference && 'quent-schema-details__field-type--reference',
    classes.fieldType,
    reference && classes.referenceType,
  )}
  data-quent-role="data-type"
  title={formatDataType(type)}
>
  {#each parts as part, index (`${index}:${part.kind}:${part.value}`)}
    {#if part.kind === 'reference'}
      <span
        class={classNames(
          'quent-schema-viewer__badge',
          'quent-schema-viewer__badge--reference',
          classes.badge,
          classes.referenceBadge,
        )}
        data-quent-role="reference-type-badge"
      >
        {part.value}
      </span>
    {:else if part.kind === 'reference-label'}
      <span
        class={classNames(
          'quent-schema-details__reference-label',
          classes.referenceLabel,
        )}
        data-quent-role={part.value === 'target:'
          ? 'reference-target-label'
          : 'reference-data-label'}
      >
        {part.value}
      </span>
    {:else if part.kind === 'reference-target'}
      <code
        class={classNames(
          'quent-schema-name',
          'quent-schema-details__reference-target',
          classes.referenceTarget,
        )}
        data-quent-role="reference-target"
        data-quent-schema-name="true"
      >
        {part.value}
      </code>
    {:else if part.kind === 'none'}
      <code class="quent-schema-details__reference-none">
        {part.value}
      </code>
    {:else}
      <code
        class={classNames(
          part.kind === 'type' && 'quent-schema-name',
          part.kind === 'syntax' &&
            'quent-schema-details__type-syntax',
        )}
        data-quent-schema-name={part.kind === 'type' ? 'true' : undefined}
      >
        {part.value}
      </code>
    {/if}
  {/each}
</span>
