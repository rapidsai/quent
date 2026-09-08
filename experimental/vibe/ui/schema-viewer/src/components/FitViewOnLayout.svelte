<!-- SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved. -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

<script lang="ts">
  import {
    useNodesInitialized,
    useSvelteFlow,
    useViewportInitialized,
  } from '@xyflow/svelte';
  import { tick } from 'svelte';

  import type { EntityFlowEdge, EntityFlowNode } from '../lib/types';

  interface Props {
    active: boolean;
    version: number;
    padding: number;
    minZoom: number;
    maxZoom: number;
  }

  let {
    active,
    version,
    padding,
    minZoom,
    maxZoom,
  }: Props = $props();
  const { fitView } = useSvelteFlow<EntityFlowNode, EntityFlowEdge>();
  const nodesInitialized = useNodesInitialized();
  const viewportInitialized = useViewportInitialized();
  let fittedVersion = $state(-1);

  $effect(() => {
    const nextVersion = version;
    if (
      !active ||
      !nodesInitialized.current ||
      !viewportInitialized.current ||
      fittedVersion === nextVersion
    ) {
      return;
    }

    let cancelled = false;
    void tick()
      .then(
        () =>
          new Promise<void>((resolve) => {
            requestAnimationFrame(() => resolve());
          }),
      )
      .then(async () => {
        if (cancelled) {
          return;
        }
        const fitted = await fitView({
          padding,
          minZoom,
          maxZoom,
        });
        if (!cancelled && fitted) {
          fittedVersion = nextVersion;
        }
      });

    return () => {
      cancelled = true;
    };
  });
</script>
