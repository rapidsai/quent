// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, it, expect } from 'vitest';
import { getActivePalette } from '@quent/utils';
import type { DAGNode, DAGEdge } from '@quent/utils';
import {
  computeNodeColoring,
  computeEdgeColoring,
  computeEdgeWidthConfig,
} from './dagFieldProcessing';

// ---- Helpers ---------------------------------------------------------------

/** Build a DAGNode whose rawNode carries the given custom_statistics map. */
function makeNode(id: string, stats: Record<string, unknown> = {}): DAGNode {
  return {
    id,
    label: id,
    type: 'operator',
    metadata: {
      rawNode: { statistics: { custom_statistics: stats } },
    },
  };
}

/** Build a DAGEdge with optional portStats. */
function makeEdge(id: string, portStats: DAGEdge['portStats'] = []): DAGEdge {
  return { id, source: 's', target: 't', portStats };
}

function tagged(variant: string, value: unknown) {
  return { [variant]: value };
}

// ---- computeNodeColoring ---------------------------------------------------

describe('computeNodeColoring', () => {
  it('returns null when field is null', () => {
    const nodes = [makeNode('n1', { rows: tagged('UInt64', 10) })];
    expect(computeNodeColoring(nodes, null, 'light')).toBeNull();
  });

  it('returns null when nodes array is empty', () => {
    expect(computeNodeColoring([], 'rows', 'light')).toBeNull();
  });

  it('returns null when no node has the requested field', () => {
    const nodes = [makeNode('n1', { bytes: tagged('UInt64', 100) })];
    expect(computeNodeColoring(nodes, 'rows', 'light')).toBeNull();
  });

  it('returns null when all matching stat values are null', () => {
    const nodes = [makeNode('n1', { rows: null })];
    expect(computeNodeColoring(nodes, 'rows', 'light')).toBeNull();
  });

  it('returns continuous coloring for numeric values', () => {
    const nodes = [
      makeNode('n1', { rows: tagged('UInt64', 10) }),
      makeNode('n2', { rows: tagged('UInt64', 50) }),
      makeNode('n3', { rows: tagged('UInt64', 20) }),
    ];
    const result = computeNodeColoring(nodes, 'rows', 'light');
    expect(result?.type).toBe('continuous');
    if (result?.type !== 'continuous') return;
    expect(result.min).toBe(10);
    expect(result.max).toBe(50);
    expect(result.values.get('n1')).toBe(10);
    expect(result.values.get('n2')).toBe(50);
    expect(result.values.get('n3')).toBe(20);
  });

  it('returns continuous coloring with equal min/max for a single node', () => {
    const nodes = [makeNode('n1', { rows: tagged('UInt64', 7) })];
    const result = computeNodeColoring(nodes, 'rows', 'light');
    expect(result?.type).toBe('continuous');
    if (result?.type !== 'continuous') return;
    expect(result.min).toBe(7);
    expect(result.max).toBe(7);
  });

  it('returns categorical coloring for string values', () => {
    const nodes = [
      makeNode('n1', { state: tagged('String', 'active') }),
      makeNode('n2', { state: tagged('String', 'idle') }),
      makeNode('n3', { state: tagged('String', 'active') }),
    ];
    const result = computeNodeColoring(nodes, 'state', 'light');
    expect(result?.type).toBe('categorical');
    if (result?.type !== 'categorical') return;
    // 'active' and 'idle' must have colors; n1 and n3 should share the same color
    expect(result.colorMap.get('n1')).toBe(result.colorMap.get('n3'));
    expect(result.colorMap.get('n1')).not.toBe(result.colorMap.get('n2'));
  });

  it('assigns palette colors in order of first appearance', () => {
    const palette = getActivePalette('light');
    const nodes = [
      makeNode('n1', { state: tagged('String', 'alpha') }),
      makeNode('n2', { state: tagged('String', 'beta') }),
    ];
    const result = computeNodeColoring(nodes, 'state', 'light');
    expect(result?.type).toBe('categorical');
    if (result?.type !== 'categorical') return;
    expect(result.categoryMap.get('alpha')).toBe(palette[0]);
    expect(result.categoryMap.get('beta')).toBe(palette[1]);
  });

  it('skips nodes that do not have the requested field', () => {
    const nodes = [
      makeNode('n1', { rows: tagged('UInt64', 10) }),
      makeNode('n2', {}), // no 'rows' stat
    ];
    const result = computeNodeColoring(nodes, 'rows', 'light');
    expect(result?.type).toBe('continuous');
    if (result?.type !== 'continuous') return;
    expect(result.values.has('n1')).toBe(true);
    expect(result.values.has('n2')).toBe(false);
  });
});

// ---- computeEdgeColoring ---------------------------------------------------

describe('computeEdgeColoring', () => {
  it('returns null when field is null', () => {
    const edges = [makeEdge('e1', [{ key: 'rows', value: 5 }])];
    expect(computeEdgeColoring(edges, null, 'light')).toBeNull();
  });

  it('returns null when edges array is empty', () => {
    expect(computeEdgeColoring([], 'rows', 'light')).toBeNull();
  });

  it('returns null when no edge has the requested field', () => {
    const edges = [makeEdge('e1', [{ key: 'bytes', value: 100 }])];
    expect(computeEdgeColoring(edges, 'rows', 'light')).toBeNull();
  });

  it('returns null when all matching stat values are null', () => {
    const edges = [makeEdge('e1', [{ key: 'rows', value: null }])];
    expect(computeEdgeColoring(edges, 'rows', 'light')).toBeNull();
  });

  it('returns continuous coloring for numeric values', () => {
    const edges = [
      makeEdge('e1', [{ key: 'rows', value: 10 }]),
      makeEdge('e2', [{ key: 'rows', value: 40 }]),
    ];
    const result = computeEdgeColoring(edges, 'rows', 'light');
    expect(result?.type).toBe('continuous');
    if (result?.type !== 'continuous') return;
    expect(result.min).toBe(10);
    expect(result.max).toBe(40);
    expect(result.values.get('e1')).toBe(10);
    expect(result.values.get('e2')).toBe(40);
  });

  it('returns categorical coloring for string values', () => {
    const edges = [
      makeEdge('e1', [{ key: 'type', value: 'hash' }]),
      makeEdge('e2', [{ key: 'type', value: 'merge' }]),
    ];
    const result = computeEdgeColoring(edges, 'type', 'light');
    expect(result?.type).toBe('categorical');
    if (result?.type !== 'categorical') return;
    expect(result.colorMap.has('e1')).toBe(true);
    expect(result.colorMap.has('e2')).toBe(true);
    expect(result.colorMap.get('e1')).not.toBe(result.colorMap.get('e2'));
  });

  it('populates labelMap with the string value of each edge', () => {
    const edges = [
      makeEdge('e1', [{ key: 'type', value: 'hash' }]),
      makeEdge('e2', [{ key: 'type', value: 'merge' }]),
    ];
    const result = computeEdgeColoring(edges, 'type', 'light');
    expect(result?.type).toBe('categorical');
    if (result?.type !== 'categorical') return;
    expect(result.labelMap.get('e1')).toBe('hash');
    expect(result.labelMap.get('e2')).toBe('merge');
  });

  it('edges without the field are excluded from results', () => {
    const edges = [
      makeEdge('e1', [{ key: 'rows', value: 10 }]),
      makeEdge('e2', []), // no portStats for 'rows'
    ];
    const result = computeEdgeColoring(edges, 'rows', 'light');
    expect(result?.type).toBe('continuous');
    if (result?.type !== 'continuous') return;
    expect(result.values.has('e1')).toBe(true);
    expect(result.values.has('e2')).toBe(false);
  });
});

// ---- computeEdgeWidthConfig ------------------------------------------------

describe('computeEdgeWidthConfig', () => {
  it('returns null when field is null', () => {
    const edges = [makeEdge('e1', [{ key: 'rows', value: 5 }])];
    expect(computeEdgeWidthConfig(edges, null)).toBeNull();
  });

  it('returns null when edges array is empty', () => {
    expect(computeEdgeWidthConfig([], 'rows')).toBeNull();
  });

  it('returns null when no edge has a numeric value for the field', () => {
    const edges = [makeEdge('e1', [{ key: 'rows', value: 'not-a-number' }])];
    expect(computeEdgeWidthConfig(edges, 'rows')).toBeNull();
  });

  it('returns null when field value is null', () => {
    const edges = [makeEdge('e1', [{ key: 'rows', value: null }])];
    expect(computeEdgeWidthConfig(edges, 'rows')).toBeNull();
  });

  it('computes min, max, and values map for numeric fields', () => {
    const edges = [
      makeEdge('e1', [{ key: 'rows', value: 5 }]),
      makeEdge('e2', [{ key: 'rows', value: 20 }]),
      makeEdge('e3', [{ key: 'rows', value: 12 }]),
    ];
    const result = computeEdgeWidthConfig(edges, 'rows');
    expect(result).not.toBeNull();
    expect(result!.min).toBe(5);
    expect(result!.max).toBe(20);
    expect(result!.values.get('e1')).toBe(5);
    expect(result!.values.get('e2')).toBe(20);
    expect(result!.values.get('e3')).toBe(12);
  });

  it('returns equal min/max for a single edge', () => {
    const edges = [makeEdge('e1', [{ key: 'rows', value: 7 }])];
    const result = computeEdgeWidthConfig(edges, 'rows');
    expect(result!.min).toBe(7);
    expect(result!.max).toBe(7);
  });

  it('excludes edges whose field value is not a number', () => {
    const edges = [
      makeEdge('e1', [{ key: 'rows', value: 10 }]),
      makeEdge('e2', [{ key: 'rows', value: 'skip' }]),
    ];
    const result = computeEdgeWidthConfig(edges, 'rows');
    expect(result).not.toBeNull();
    expect(result!.values.has('e1')).toBe(true);
    expect(result!.values.has('e2')).toBe(false);
  });
});
