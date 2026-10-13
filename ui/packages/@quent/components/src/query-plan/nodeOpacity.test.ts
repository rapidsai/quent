// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it } from 'vitest';
import { getNodeOpacityClass } from './nodeOpacity';

describe('getNodeOpacityClass', () => {
  it('keeps every selected node highlighted during a DAG hover', () => {
    expect(
      getNodeOpacityClass({
        isHoveredStatActive: false,
        hasHoveredValue: false,
        highlightedNodeIds: new Set(['hovered']),
        operatorId: 'selected',
        isDimmed: false,
        isSelected: true,
      })
    ).toBe('opacity-100');
  });

  it('dims nodes that are neither selected nor hovered', () => {
    expect(
      getNodeOpacityClass({
        isHoveredStatActive: false,
        hasHoveredValue: false,
        highlightedNodeIds: new Set(['hovered']),
        operatorId: 'other',
        isDimmed: true,
        isSelected: false,
      })
    ).toBe('opacity-35');
  });

  it('highlights a logical node whose value came from aggregated related operators', () => {
    expect(
      getNodeOpacityClass({
        isHoveredStatActive: true,
        hasHoveredValue: true,
        highlightedNodeIds: null,
        operatorId: 'logical-node',
        isDimmed: false,
        isSelected: false,
      })
    ).toBe('opacity-100');
  });

  it('dims a node with no direct or aggregated value while a stat is hovered', () => {
    expect(
      getNodeOpacityClass({
        isHoveredStatActive: true,
        hasHoveredValue: false,
        highlightedNodeIds: null,
        operatorId: 'unrelated-node',
        isDimmed: false,
        isSelected: false,
      })
    ).toBe('opacity-20');
  });
});
