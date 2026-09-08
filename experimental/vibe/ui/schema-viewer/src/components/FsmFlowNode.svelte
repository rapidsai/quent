<!-- SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved. -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

<script lang="ts">
  import {
    Handle,
    Position,
    type NodeProps,
  } from '@xyflow/svelte';

  import type { FsmFlowNodeData } from '../lib/types';

  let {
    data,
    sourcePosition,
    targetPosition,
  }: NodeProps & { data: FsmFlowNodeData } = $props();
</script>

<Handle
  type="target"
  position={targetPosition ?? Position.Top}
  isConnectable={false}
  class="quent-entity-flow__handle"
/>
{#if !data.entry && !data.exit}
  <span
    class="quent-schema-details__fsm-flow-node-title quent-schema-name quent-schema-name--title"
    data-quent-role="fsm-state-title"
    data-quent-schema-name="true"
  >
    {data.label}
  </span>
{/if}
<Handle
  type="source"
  position={sourcePosition ?? Position.Bottom}
  isConnectable={false}
  class="quent-entity-flow__handle"
/>
