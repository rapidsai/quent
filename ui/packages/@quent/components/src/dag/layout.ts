// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { graph, sugiyama } from 'd3-dag';
import type { Node, Edge } from '@xyflow/react';

export const NODE_LAYOUT_WIDTH = 200;
const NODE_LAYOUT_HEIGHT = 60;
// Spacing between adjacent nodes in the same layer (horizontal gap)
const NODE_SPACING = 50;
// Spacing between layers (vertical gap)
const LAYER_SPACING = 100;

// nodeSize encodes center-to-center distance per axis:
//   x (within a layer): node width + gap between siblings
//   y (between layers): node height + gap between layers
const NODE_SIZE = [NODE_LAYOUT_WIDTH + NODE_SPACING, NODE_LAYOUT_HEIGHT + LAYER_SPACING] as const;

export async function calculateLayout<TData extends Record<string, unknown>>(
  nodes: Node<TData>[],
  edges: Edge[]
): Promise<{ nodes: Node<TData>[]; edges: Edge[] }> {
  const grf = graph<string, undefined>();
  const nodeById = new Map<string, ReturnType<typeof grf.node>>();
  for (const n of nodes) nodeById.set(n.id, grf.node(n.id));
  for (const e of edges) {
    const src = nodeById.get(e.source);
    const tgt = nodeById.get(e.target);
    if (src && tgt) grf.link(src, tgt, undefined);
  }

  // sugiyama is synchronous and top-to-bottom by default
  sugiyama().nodeSize(NODE_SIZE)(grf);

  // d3-dag reports node centers; ReactFlow expects top-left
  const posMap = new Map<string, { x: number; y: number }>();
  for (const node of grf.nodes()) {
    posMap.set(node.data, {
      x: (node.x ?? 0) - NODE_LAYOUT_WIDTH / 2,
      y: (node.y ?? 0) - NODE_LAYOUT_HEIGHT / 2,
    });
  }

  return {
    nodes: nodes.map(n => ({ ...n, position: posMap.get(n.id) ?? { x: 0, y: 0 } })),
    edges,
  };
}
