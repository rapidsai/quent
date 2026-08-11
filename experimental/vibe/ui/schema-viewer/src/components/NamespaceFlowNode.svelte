<!-- SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved. -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

<script lang="ts">
  import type { NodeProps } from '@xyflow/svelte';

  import type { NamespaceFlowNodeData } from '../lib/types';

  let { data }: NodeProps & { data: NamespaceFlowNodeData } = $props();
</script>

<span
  class="quent-entity-flow__namespace-label quent-schema-name"
  data-quent-schema-name="true"
>
  {data.label}
</span>
