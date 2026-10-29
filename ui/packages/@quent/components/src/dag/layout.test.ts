// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, it, expect } from 'vitest';
import type { Node, Edge } from '@xyflow/react';
import { DAG_LAYOUT_DIRECTION } from '@quent/utils';
import { calculateLayout } from './layout';

// Linear chain: Scan (source) -> Filter -> Root (sink), matching trace edge
// direction (child.out -> parent.in).
const nodes: Node<Record<string, unknown>>[] = [
  { id: 'scan', position: { x: 0, y: 0 }, data: {} },
  { id: 'filter', position: { x: 0, y: 0 }, data: {} },
  { id: 'root', position: { x: 0, y: 0 }, data: {} },
];
const edges: Edge[] = [
  { id: 'e1', source: 'scan', target: 'filter' },
  { id: 'e2', source: 'filter', target: 'root' },
];

describe('calculateLayout', () => {
  it('defaults to bottom-to-top: sources render below the root', async () => {
    const result = await calculateLayout(nodes, edges);
    const yById = new Map(result.nodes.map(n => [n.id, n.position.y]));
    expect(yById.get('root')).toBeLessThan(yById.get('filter')!);
    expect(yById.get('filter')).toBeLessThan(yById.get('scan')!);
  });

  it('top-to-bottom: sources render above the root', async () => {
    const result = await calculateLayout(nodes, edges, DAG_LAYOUT_DIRECTION.TOP_TO_BOTTOM);
    const yById = new Map(result.nodes.map(n => [n.id, n.position.y]));
    expect(yById.get('scan')).toBeLessThan(yById.get('filter')!);
    expect(yById.get('filter')).toBeLessThan(yById.get('root')!);
  });

  it('leaves rendered edges pointing from source to sink regardless of direction', async () => {
    const result = await calculateLayout(nodes, edges);
    expect(result.edges).toEqual(edges);
  });

  it('preserves layer spacing when rendered nodes grow taller', async () => {
    const tallNodes: Node<Record<string, unknown>>[] = [
      {
        ...nodes[0],
        measured: { width: 200, height: 120 },
      },
      {
        ...nodes[2],
        measured: { width: 200, height: 180 },
      },
    ];
    const result = await calculateLayout(tallNodes, [
      { id: 'tall-edge', source: 'scan', target: 'root' },
    ]);
    const byId = new Map(result.nodes.map(node => [node.id, node]));
    const root = byId.get('root')!;
    const scan = byId.get('scan')!;

    expect(scan.position.y - (root.position.y + root.measured!.height!)).toBeGreaterThanOrEqual(
      100
    );
  });

  it('preserves sibling spacing when rendered nodes grow wider', async () => {
    const wideNodes: Node<Record<string, unknown>>[] = [
      {
        id: 'left',
        position: { x: 0, y: 0 },
        data: {},
        measured: { width: 300, height: 72 },
      },
      {
        id: 'right',
        position: { x: 0, y: 0 },
        data: {},
        measured: { width: 240, height: 72 },
      },
      nodes[2],
    ];
    const result = await calculateLayout(wideNodes, [
      { id: 'left-root', source: 'left', target: 'root' },
      { id: 'right-root', source: 'right', target: 'root' },
    ]);
    const siblings = result.nodes
      .filter(node => node.id !== 'root')
      .sort((left, right) => left.position.x - right.position.x);

    expect(
      siblings[1].position.x - (siblings[0].position.x + siblings[0].measured!.width!)
    ).toBeGreaterThanOrEqual(50);
  });
});
