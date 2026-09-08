<!-- SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved. -->
<!-- SPDX-License-Identifier: Apache-2.0 -->

<script lang="ts">
  import {
    SvelteFlow,
    type NodeEventWithPointer,
  } from '@xyflow/svelte';
  import '@xyflow/svelte/dist/style.css';

  import { createEmptyEntityGraphLayout, layoutEntityGraph } from '../lib/layout';
  import { referenceMatchesFilter } from '../lib/schema';
  import {
    entitySelectionFromFlowNode,
    READ_ONLY_FLOW_CONFIG,
    referenceSelectionFromFlowEdge,
    toEntityFlowElements,
  } from '../lib/xyflow';
  import type {
    EntityFlowEdge,
    EntityFlowNode,
    EntityGraphClasses,
    EntityGraphLayoutComplete,
    EntityGraphLayoutError,
    EntityGraphLayoutStart,
    EntityGraphModel,
    EntityNodeComponent,
    ResolvedEntityGraphConfig,
    SchemaSelection,
  } from '../lib/types';
  import type { Schema } from '@quent/schema';

  import ElkFlowEdge from './ElkFlowEdge.svelte';
  import EntityFlowNodeComponent from './EntityFlowNode.svelte';
  import FitViewOnLayout from './FitViewOnLayout.svelte';
  import GraphViewportController from './GraphViewportController.svelte';
  import NamespaceFlowNode from './NamespaceFlowNode.svelte';

  interface Props {
    schema: Schema;
    model: EntityGraphModel;
    active: boolean;
    selection: SchemaSelection | null;
    config: ResolvedEntityGraphConfig;
    classes: EntityGraphClasses;
    nodeComponent: EntityNodeComponent | null;
    onSelect: (selection: SchemaSelection) => void;
    onHover: (selection: SchemaSelection) => void;
    onHoverEnd: () => void;
    onLayoutStart: (detail: EntityGraphLayoutStart) => void;
    onLayoutComplete: (detail: EntityGraphLayoutComplete) => void;
    onLayoutError: (detail: EntityGraphLayoutError) => void;
  }

  let {
    schema,
    model,
    active,
    selection,
    config,
    classes,
    nodeComponent,
    onSelect,
    onHover,
    onHoverEnd,
    onLayoutStart,
    onLayoutComplete,
    onLayoutError,
  }: Props = $props();
  let layout = $state(createEmptyEntityGraphLayout());
  let layoutStatus = $state<'loading' | 'ready' | 'error'>('loading');
  let nodes = $state.raw<EntityFlowNode[]>([]);
  let edges = $state.raw<EntityFlowEdge[]>([]);
  let layoutVersion = $state(0);
  let fitVersion = $state(0);
  let layoutMetrics = $state<EntityGraphLayoutComplete | null>(null);
  let lastFittedModel: EntityGraphModel | null = null;
  let viewportActions = $state<{
    zoomIn: () => Promise<boolean>;
    zoomOut: () => Promise<boolean>;
    fitView: () => Promise<boolean>;
  } | null>(null);

  const nodeTypes = {
    'quent-entity': EntityFlowNodeComponent,
    'quent-namespace': NamespaceFlowNode,
  };
  const edgeTypes = {
    'quent-elk': ElkFlowEdge,
  };

  $effect(() => {
    const currentModel = model;
    const currentConfig = config;
    let active = true;
    const referenceCount = currentModel.references.filter(
      (reference) =>
        reference.target &&
        referenceMatchesFilter(reference, currentConfig.references),
    ).length;
    const started = performance.now();
    layoutStatus = 'loading';
    onLayoutStart({
      nodeCount: currentModel.nodes.length,
      referenceCount,
    });
    void layoutEntityGraph(currentModel, currentConfig)
      .then((result) => {
        if (!active) {
          return;
        }
        const elements = toEntityFlowElements({
          schema,
          layout: result,
          config,
          classes,
          selection,
          nodeComponent,
        });
        layout = result;
        nodes = elements.nodes;
        edges = elements.edges;
        layoutVersion += 1;
        if (lastFittedModel !== currentModel) {
          lastFittedModel = currentModel;
          fitVersion += 1;
        }
        layoutStatus = 'ready';
        const detail = {
          width: result.width,
          height: result.height,
          nodeCount: currentModel.nodes.length,
          referenceCount,
          durationMs: performance.now() - started,
        };
        layoutMetrics = detail;
        onLayoutComplete(detail);
      })
      .catch((error: unknown) => {
        if (!active) {
          return;
        }
        layoutStatus = 'error';
        onLayoutError({
          nodeCount: currentModel.nodes.length,
          referenceCount,
          error,
        });
      });
    return () => {
      active = false;
    };
  });

  $effect(() => {
    if (layoutStatus !== 'ready') {
      return;
    }
    const elements = toEntityFlowElements({
      schema,
      layout,
      config,
      classes,
      selection,
      nodeComponent,
    });
    nodes = elements.nodes;
    edges = elements.edges;
  });

  function selectNode(
    { node }: Parameters<NodeEventWithPointer<MouseEvent | TouchEvent, EntityFlowNode>>[0],
  ): void {
    const value = entitySelectionFromFlowNode(node);
    if (value) {
      onSelect(value);
    }
  }

  function hoverNode(
    { node }: Parameters<NodeEventWithPointer<PointerEvent, EntityFlowNode>>[0],
  ): void {
    const value = entitySelectionFromFlowNode(node);
    if (value) {
      onHover(value);
    }
  }

  function selectEdge(
    { edge }: { edge: EntityFlowEdge; event: MouseEvent },
  ): void {
    onSelect(referenceSelectionFromFlowEdge(edge));
  }

  function hoverEdge(
    { edge }: { edge: EntityFlowEdge; event: PointerEvent },
  ): void {
    onHover(referenceSelectionFromFlowEdge(edge));
  }

  function endEdgeHover(): void {
    onHoverEnd();
  }
</script>

{#if layoutStatus === 'loading'}
  <div class="quent-schema-viewer__empty" data-quent-role="loading">
    Laying out entity graph.
  </div>
{:else if layoutStatus === 'error'}
  <div class="quent-schema-viewer__empty" data-quent-role="layout-error">
    Entity graph layout failed.
  </div>
{:else}
  <div
    class={`quent-entity-flow ${classes.viewport ?? ''}`}
    data-quent-role="viewport"
    data-quent-view="graph"
    data-layout-version={layoutVersion}
    data-fit-version={fitVersion}
  >
    <div class="quent-entity-flow__canvas">
      <SvelteFlow
        bind:nodes
        bind:edges
        {nodeTypes}
        {edgeTypes}
        minZoom={config.minZoom}
        maxZoom={config.maxZoom}
        {...READ_ONLY_FLOW_CONFIG}
        onnodeclick={selectNode}
        onnodepointerenter={hoverNode}
        onnodepointerleave={onHoverEnd}
        onedgeclick={selectEdge}
        onedgepointerenter={hoverEdge}
        onedgepointerleave={endEdgeHover}
        colorMode="light"
        proOptions={{ hideAttribution: true }}
      >
        <FitViewOnLayout
          {active}
          version={fitVersion}
          padding={config.fitPadding}
          minZoom={config.minZoom}
          maxZoom={config.maxZoom}
        />
        <GraphViewportController
          onReady={(actions) => (viewportActions = actions)}
          padding={config.fitPadding}
          minZoom={config.minZoom}
          maxZoom={config.maxZoom}
        />
      </SvelteFlow>
    </div>
    <div
      class={`quent-entity-flow__controls ${classes.controls ?? ''}`}
      data-quent-role="viewport-controls"
      aria-label="Graph viewport controls"
    >
      {#if layoutMetrics}
        <span
          class="quent-entity-flow__status"
          data-quent-role="layout-status"
        >
          {`${layoutMetrics.nodeCount} nodes · ${layoutMetrics.referenceCount} references · ${Math.round(layoutMetrics.durationMs)} ms`}
        </span>
      {/if}
      <button
        type="button"
        aria-label="Zoom out"
        title="Zoom out"
        disabled={!viewportActions}
        onclick={() => void viewportActions?.zoomOut()}
      >−</button>
      <button
        type="button"
        aria-label="Fit graph to view"
        disabled={!viewportActions}
        onclick={() => void viewportActions?.fitView()}
      >Fit</button>
      <button
        type="button"
        aria-label="Zoom in"
        title="Zoom in"
        disabled={!viewportActions}
        onclick={() => void viewportActions?.zoomIn()}
      >+</button>
    </div>
  </div>
{/if}
