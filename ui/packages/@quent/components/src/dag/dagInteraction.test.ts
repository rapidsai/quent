// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it } from 'vitest';
import type { DAGNode } from '@quent/utils';
import type { HoveredStatInfo } from '@quent/hooks';
import { resolveDagHeatmap, resolveDagHighlightedNodeIds } from './dagInteraction';

const NODES: DAGNode[] = [
  { id: 'plain', label: 'Plain', type: 'scan' },
  {
    id: 'grouped',
    label: 'Grouped',
    type: 'join',
    metadata: { relatedOperatorIds: ['a', 'b'] },
  },
];

function hoveredStat(values: Record<string, number>): HoveredStatInfo {
  return {
    name: 'duration_s',
    values: new Map(Object.entries(values)),
    min: 0,
    max: 100,
    aggMode: 'sum',
  };
}

describe('resolveDagHeatmap', () => {
  it('ranges over direct and grouped values resolved for this DAG', () => {
    const heatmap = resolveDagHeatmap(hoveredStat({ plain: 1, a: 4, b: 5 }), NODES);

    expect(heatmap?.values).toEqual(
      new Map([
        ['plain', 1],
        ['grouped', 9],
      ])
    );
    expect(heatmap?.range).toEqual({ min: 1, max: 9 });
  });

  it('returns null when the hovered stat has no value in this DAG', () => {
    expect(resolveDagHeatmap(hoveredStat({ unrelated: 1 }), NODES)).toBeNull();
  });
});

describe('resolveDagHighlightedNodeIds', () => {
  it('keeps highlights that target a displayed node', () => {
    const highlighted = new Set(['plain']);
    expect(resolveDagHighlightedNodeIds(highlighted, NODES)).toBe(highlighted);
  });

  it('keeps highlights that target a displayed node’s related operator', () => {
    const highlighted = new Set(['a']);
    expect(resolveDagHighlightedNodeIds(highlighted, NODES)).toBe(highlighted);
  });

  it('clears highlights that do not affect this DAG', () => {
    expect(resolveDagHighlightedNodeIds(new Set(['unrelated']), NODES)).toBeNull();
  });
});
