<!-- SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved. -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

<script lang="ts">
  import { useSvelteFlow } from '@xyflow/svelte';
  import { onMount } from 'svelte';

  import type { EntityFlowEdge, EntityFlowNode } from '../lib/types';

  interface ViewportActions {
    zoomIn: () => Promise<boolean>;
    zoomOut: () => Promise<boolean>;
    fitView: () => Promise<boolean>;
  }

  interface Props {
    onReady: (actions: ViewportActions | null) => void;
    padding: number;
    minZoom: number;
    maxZoom: number;
  }

  let { onReady, padding, minZoom, maxZoom }: Props = $props();
  const { zoomIn, zoomOut, fitView } = useSvelteFlow<
    EntityFlowNode,
    EntityFlowEdge
  >();

  onMount(() => {
    onReady({
      zoomIn: () => zoomIn(),
      zoomOut: () => zoomOut(),
      fitView: () => fitView({ padding, minZoom, maxZoom }),
    });
    return () => onReady(null);
  });
</script>
