<!-- SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved. -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

<script lang="ts">
  import {
    Handle,
    Position,
    type NodeProps,
  } from '@xyflow/svelte';

  import type {
    EntityFlowNodeData,
  } from '../lib/types';

  let {
    data,
    sourcePosition,
    targetPosition,
    selected,
  }: NodeProps & { data: EntityFlowNodeData } = $props();
  let entityType = $derived(
    data.node.resource ? 'Resource' : data.node.fsm ? 'FSM' : 'Entity',
  );
  let entityTypeClass = $derived(
    data.node.resource
      ? 'quent-schema-viewer__badge--resource'
      : data.node.fsm
        ? 'quent-schema-viewer__badge--fsm'
        : 'quent-schema-viewer__badge--entity',
  );
</script>

<Handle
  type="target"
  position={targetPosition ?? Position.Top}
  isConnectable={false}
  class="quent-entity-flow__handle"
/>
<div
  class:quent-entity-graph__node-content={true}
  class:quent-entity-graph__node-content--selected={selected}
>
  {#if data.nodeComponent}
    <data.nodeComponent schema={data.schema} path={data.node.path} />
  {:else}
    <strong
      class="quent-entity-graph__node-title quent-schema-name quent-schema-name--title"
      data-quent-role="entity-title"
      data-quent-schema-name="true"
    >
      {data.node.path.name}
    </strong>
    {#if data.config.showNodeMetadata}
      <small class="quent-entity-graph__node-meta">
        {data.node.referenceCount} references
      </small>
    {/if}
  {/if}
  <span
    class="quent-schema-viewer__badges"
    data-quent-role="badges"
  >
    <span
      class={`quent-schema-viewer__badge ${entityTypeClass}`}
      data-quent-role="entity-type-badge"
    >
      {entityType}
    </span>
  </span>
</div>
<Handle
  type="source"
  position={sourcePosition ?? Position.Bottom}
  isConnectable={false}
  class="quent-entity-flow__handle"
/>
