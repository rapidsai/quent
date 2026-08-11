<!-- SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved. -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

<script lang="ts">
  import type { Entity } from '@quent/schema';
  import {
    SvelteFlow,
    type NodeEventWithPointer,
  } from '@xyflow/svelte';

  import {
    layoutFsmTopology,
    type FsmGraphLayout,
  } from '../../lib/layout';
  import { pathKey } from '../../lib/schema';
  import type {
    FsmFlowEdge,
    FsmFlowNode,
    FsmTopology,
    SchemaDetailsClasses,
    SchemaSelection,
  } from '../../lib/types';
  import {
    READ_ONLY_FLOW_CONFIG,
    toFsmFlowElements,
  } from '../../lib/xyflow';
  import ElkFlowEdge from '../ElkFlowEdge.svelte';
  import FsmFlowNodeComponent from '../FsmFlowNode.svelte';

  interface Props {
    entity: Entity;
    topology: FsmTopology;
    selection: SchemaSelection | null;
    classes: SchemaDetailsClasses;
    onSelect: (selection: SchemaSelection) => void;
  }

  let { entity, topology, selection, classes, onSelect }: Props = $props();
  let layout = $state<FsmGraphLayout | null>(null);
  let nodes = $state.raw<FsmFlowNode[]>([]);
  let edges = $state.raw<FsmFlowEdge[]>([]);
  let failed = $state(false);

  const edgeTypes = {
    'quent-elk': ElkFlowEdge,
  };
  const nodeTypes = {
    'quent-fsm': FsmFlowNodeComponent,
  };

  $effect(() => {
    const value = topology;
    let active = true;
    layout = null;
    nodes = [];
    edges = [];
    failed = false;
    void layoutFsmTopology(value)
      .then((result) => {
        if (!active) {
          return;
        }
        layout = result;
        ({ nodes, edges } = toFsmFlowElements({
          path: entity.path,
          topology,
          layout: result,
          selection,
          classes,
        }));
      })
      .catch(() => {
        if (active) {
          failed = true;
        }
      });
    return () => {
      active = false;
    };
  });

  $effect(() => {
    if (layout) {
      ({ nodes, edges } = toFsmFlowElements({
        path: entity.path,
        topology,
        layout,
        selection,
        classes,
      }));
    }
  });

  function selectState(
    { node }: Parameters<NodeEventWithPointer<MouseEvent | TouchEvent, FsmFlowNode>>[0],
  ): void {
    if (!node.data.entry && !node.data.exit) {
      onSelect({
        kind: 'fsm-state',
        entity: entity.path,
        state: node.data.state,
      });
    }
  }
</script>

<div
  class="quent-schema-details__fsm-graph"
  data-quent-role="fsm-graph"
  style:height={layout
    ? `${Math.min(Math.max(layout.height, 240), 480)}px`
    : '12rem'}
>
  {#if failed}
    <p class="quent-schema-details__muted">FSM layout failed.</p>
  {:else if !layout}
    <p class="quent-schema-details__muted">Laying out FSM topology.</p>
  {:else}
    <SvelteFlow
      bind:nodes
      bind:edges
      {nodeTypes}
      {edgeTypes}
      fitView
      fitViewOptions={{ padding: 0.12, minZoom: 0.5, maxZoom: 1.5 }}
      minZoom={0.5}
      maxZoom={1.5}
      {...READ_ONLY_FLOW_CONFIG}
      onnodeclick={selectState}
      aria-label={`FSM topology for ${pathKey(entity.path)}`}
      colorMode="light"
      proOptions={{ hideAttribution: true }}
    />
  {/if}
</div>
