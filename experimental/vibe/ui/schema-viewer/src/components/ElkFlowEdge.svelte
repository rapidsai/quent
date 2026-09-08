<!-- SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved. -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

<script lang="ts">
  import {
    BaseEdge,
    type Edge,
    type EdgeProps,
  } from '@xyflow/svelte';
  import type { ElkFlowEdgeData } from '../lib/types';

  let {
    id,
    data,
    label,
    labelStyle,
    markerStart,
    markerEnd,
    interactionWidth,
    style,
    sourceX,
    sourceY,
    targetX,
    targetY,
  }: EdgeProps<Edge<ElkFlowEdgeData, 'quent-elk'>> = $props();
  let path = $derived(
    data?.path ?? `M ${sourceX} ${sourceY} L ${targetX} ${targetY}`,
  );
  let labelX = $derived(data?.labelX ?? (sourceX + targetX) / 2);
  let labelY = $derived(data?.labelY ?? (sourceY + targetY) / 2);
</script>

<BaseEdge
  {id}
  {path}
  {label}
  {labelX}
  {labelY}
  {labelStyle}
  {markerStart}
  {markerEnd}
  {interactionWidth}
  {style}
/>
