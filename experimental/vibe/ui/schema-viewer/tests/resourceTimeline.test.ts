// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, test } from 'vitest';

import { sampleSchema } from './fixtures/sample-schema';
import { buildResourceTimeline } from '../src/lib/resourceTimeline';
import { buildEntityGraph } from '../src/lib/schema';
import type {
  EntityGraphModel,
  EntityGraphNode,
} from '../src/lib/types';

function path(id: string): EntityGraphNode['path'] {
  const segments = id.split('::');
  return {
    namespace: segments.slice(0, -1),
    name: segments.at(-1)!,
  };
}

function node(id: string, resource = false): EntityGraphNode {
  return {
    id,
    path: path(id),
    eventCount: 1,
    referenceCount: 1,
    fsm: false,
    resource,
  };
}

describe('resource timeline layout', () => {
  test('separates resource branches from branches without resources', () => {
    const graph: EntityGraphModel = {
      nodes: [
        node('Resource', true),
        node('Root'),
        node('Branch'),
        node('Unrelated'),
      ],
      references: [
        {
          id: 'branch-parent',
          source: path('Branch'),
          target: path('Root'),
          event: 'Created',
          fieldPath: ['parent'],
          tree: true,
        },
        {
          id: 'resource-parent',
          source: path('Resource'),
          target: path('Branch'),
          event: 'Created',
          fieldPath: ['parent'],
          tree: true,
        },
        {
          id: 'unrelated-parent',
          source: path('Unrelated'),
          target: path('Root'),
          event: 'Created',
          fieldPath: ['parent'],
          tree: true,
        },
      ],
    };

    const layout = buildResourceTimeline(graph, null);

    expect(layout.rows.map(({ node: item }) => item.id)).toEqual([
      'Root',
      'Branch',
      'Resource',
      'Unrelated',
    ]);
    expect(layout.rows.map(({ depth }) => depth)).toEqual([0, 1, 2, 0]);
    expect(layout.rows.map(({ resourceInScope }) => resourceInScope)).toEqual(
      [true, true, true, false],
    );
    expect(layout.rows[0]?.sequences).toEqual([]);
    expect(layout.rows[2]?.node.resource).toBe(true);
    expect(layout.filteredCount).toBe(1);
  });

  test('separates a resource cycle from unrelated entities', () => {
    const graph: EntityGraphModel = {
      nodes: [node('A', true), node('B'), node('C')],
      references: [
        {
          id: 'a-parent',
          source: path('A'),
          target: path('B'),
          event: 'Created',
          fieldPath: ['parent'],
          tree: true,
        },
        {
          id: 'b-parent',
          source: path('B'),
          target: path('A'),
          event: 'Created',
          fieldPath: ['parent'],
          tree: true,
        },
      ],
    };

    const layout = buildResourceTimeline(graph, null);

    expect(layout.rows).toHaveLength(3);
    expect(new Set(layout.rows.map(({ node: item }) => item.id))).toEqual(
      new Set(['A', 'B', 'C']),
    );
    expect(
      layout.rows.find((row) => row.node.id === 'C')?.resourceInScope,
    ).toBe(false);
  });

  test('shows only FSM sequences that use each resource', () => {
    const layout = buildResourceTimeline(
      buildEntityGraph(sampleSchema),
      sampleSchema,
    );
    const resourceRows = layout.rows.filter((row) => row.node.resource);
    const ancestorRows = layout.rows.filter((row) => !row.node.resource);
    const sequences = resourceRows.flatMap((row) => row.sequences);

    expect(resourceRows.length).toBeGreaterThan(0);
    expect(
      resourceRows.flatMap((row) => row.capacities).length,
    ).toBeGreaterThan(0);
    expect(sequences.length).toBeGreaterThan(1);
    expect(
      sequences.every((sequence) =>
        sequence.states.some((state) => state.usesResource),
      ),
    ).toBe(true);
    expect(
      sequences
        .flatMap((sequence) => sequence.states)
        .filter((state) => state.usesResource)
        .every((state) => state.capacities.length > 0),
    ).toBe(true);
    expect(
      resourceRows
        .flatMap((row) => row.capacities)
        .every(
          (capacity) =>
            capacity.bins.length === 24 &&
            capacity.bins.every(
              (bin) => bin.height > 0,
            ),
        ),
    ).toBe(true);
    expect(
      sequences.every((sequence) =>
        sequence.states.every((state) => state.width > 0),
      ),
    ).toBe(true);
    expect(ancestorRows.every((row) => row.sequences.length === 0)).toBe(
      true,
    );
  });
});
