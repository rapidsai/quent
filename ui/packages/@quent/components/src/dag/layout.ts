// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { graph, sugiyama } from 'd3-dag';
import type { Node, Edge } from '@xyflow/react';
import { DAG_LAYOUT_DIRECTION, type DagLayoutDirection } from '@quent/utils';

export const NODE_LAYOUT_WIDTH = 200;
export const NODE_LAYOUT_HEIGHT = 60;

// NodeFlowBar geometry — these drive both the DAG node height calculation and
// the inline styles in NodeFlowBar so the two stay in sync automatically.
export const FLOW_BAR_TOP_MARGIN = 6; // mt-1.5
export const FLOW_BAR_TRACK_HEIGHT = 12; // h-[12px] for each bar track
export const FLOW_BAR_TRACK_GAP = 2; // mt-[2px] between the two tracks
export const FLOW_BAR_LABEL_HEIGHT = 12; // leading-3 totals label
export const FLOW_BAR_HEIGHT =
  FLOW_BAR_TOP_MARGIN +
  FLOW_BAR_TRACK_HEIGHT +
  FLOW_BAR_TRACK_GAP +
  FLOW_BAR_TRACK_HEIGHT +
  FLOW_BAR_LABEL_HEIGHT;
// Spacing between adjacent nodes in the same layer (horizontal gap)
const NODE_SPACING = 50;
// Spacing between layers (vertical gap)
const LAYER_SPACING = 100;

export async function calculateLayout<TData extends Record<string, unknown>>(
  nodes: Node<TData>[],
  edges: Edge[],
  direction: DagLayoutDirection = DAG_LAYOUT_DIRECTION.BOTTOM_TO_TOP,
  nodeHeight = NODE_LAYOUT_HEIGHT
): Promise<{ nodes: Node<TData>[]; edges: Edge[] }> {
  const grf = graph<string, undefined>();
  const nodeById = new Map<string, ReturnType<typeof grf.node>>();
  for (const n of nodes) nodeById.set(n.id, grf.node(n.id));
  // sugiyama layers link sources above targets, so flip links to put the root on top
  const flip = direction === DAG_LAYOUT_DIRECTION.BOTTOM_TO_TOP;
  for (const e of edges) {
    const src = nodeById.get(e.source);
    const tgt = nodeById.get(e.target);
    if (src && tgt) grf.link(flip ? tgt : src, flip ? src : tgt, undefined);
  }

  // nodeSize encodes center-to-center distance per axis:
  //   x (within a layer): node width + gap between siblings
  //   y (between layers): node height + gap between layers
  const nodeSize = [NODE_LAYOUT_WIDTH + NODE_SPACING, nodeHeight + LAYER_SPACING] as const;
  // sugiyama is synchronous and layers sources at the shallowest depth by default
  sugiyama().nodeSize(nodeSize)(grf);

  // d3-dag reports node centers; ReactFlow expects top-left
  const posMap = new Map<string, { x: number; y: number }>();
  for (const node of grf.nodes()) {
    posMap.set(node.data, {
      x: (node.x ?? 0) - NODE_LAYOUT_WIDTH / 2,
      y: (node.y ?? 0) - nodeHeight / 2,
    });
  }

  return {
    nodes: nodes.map(n => ({ ...n, position: posMap.get(n.id) ?? { x: 0, y: 0 } })),
    edges,
  };
}
