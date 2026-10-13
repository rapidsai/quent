// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it } from 'vitest';
import { shouldDimEdgeFromInteraction } from './edgeOpacity';

describe('shouldDimEdgeFromInteraction', () => {
  it('dims an edge that is neither selected nor highlighted', () => {
    expect(
      shouldDimEdgeFromInteraction({
        sourceId: 'other-source',
        targetId: 'other-target',
        selectedNodeIds: new Set(['selected']),
        highlightedNodeIds: new Set(['highlighted']),
      })
    ).toBe(true);
  });

  it('keeps an edge touching a selected node visible during a highlight elsewhere', () => {
    expect(
      shouldDimEdgeFromInteraction({
        sourceId: 'selected',
        targetId: 'other',
        selectedNodeIds: new Set(['selected']),
        highlightedNodeIds: new Set(['highlighted']),
      })
    ).toBe(false);
  });
});

describe('inspected pipes', () => {
  const common = {
    sourceId: 's',
    targetId: 't',
    selectedNodeIds: new Set<string>(),
    highlightedNodeIds: null,
    inspectedEdgeIds: new Set(['one', 'two']),
  };

  it('keeps every inspected pipe undimmed and dims the rest', () => {
    expect(shouldDimEdgeFromInteraction({ ...common, edgeId: 'one' })).toBe(false);
    expect(shouldDimEdgeFromInteraction({ ...common, edgeId: 'two' })).toBe(false);
    expect(shouldDimEdgeFromInteraction({ ...common, edgeId: 'three' })).toBe(true);
  });

  it('also keeps pipes touching selected operators undimmed', () => {
    const selected = { ...common, selectedNodeIds: new Set(['s']) };
    expect(shouldDimEdgeFromInteraction({ ...selected, edgeId: 'three' })).toBe(false);
    expect(
      shouldDimEdgeFromInteraction({ ...selected, edgeId: 'three', sourceId: 'x', targetId: 'y' })
    ).toBe(true);
  });

  it('narrows emphasis to the hovered pipe alone', () => {
    const hovered = { ...common, hoveredEdgeId: 'one' };
    expect(shouldDimEdgeFromInteraction({ ...hovered, edgeId: 'one' })).toBe(false);
    expect(shouldDimEdgeFromInteraction({ ...hovered, edgeId: 'two' })).toBe(true);
  });
});
