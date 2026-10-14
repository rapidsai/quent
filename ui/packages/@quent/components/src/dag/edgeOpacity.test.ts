// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it } from 'vitest';
import { shouldDimEdgeFromInteraction } from './edgeOpacity';

const idle = {
  edgeId: 'edge',
  selectedNodeIds: new Set<string>(),
  highlightedNodeIds: null,
};

describe('shouldDimEdgeFromInteraction', () => {
  it('does not dim an edge when nothing is selected, inspected, hovered or highlighted', () => {
    expect(shouldDimEdgeFromInteraction(idle)).toBe(false);
  });

  it('dims every edge while an operator is highlighted, like the other nodes', () => {
    expect(
      shouldDimEdgeFromInteraction({ ...idle, highlightedNodeIds: new Set(['highlighted']) })
    ).toBe(true);
  });

  it('dims every edge while an operator is selected', () => {
    expect(shouldDimEdgeFromInteraction({ ...idle, selectedNodeIds: new Set(['selected']) })).toBe(
      true
    );
  });
});

describe('inspected pipes', () => {
  const common = { ...idle, inspectedEdgeIds: new Set(['one', 'two']) };

  it('keeps every inspected pipe undimmed and dims the rest', () => {
    expect(shouldDimEdgeFromInteraction({ ...common, edgeId: 'one' })).toBe(false);
    expect(shouldDimEdgeFromInteraction({ ...common, edgeId: 'two' })).toBe(false);
    expect(shouldDimEdgeFromInteraction({ ...common, edgeId: 'three' })).toBe(true);
  });

  it('keeps an inspected pipe undimmed while an operator is selected or highlighted', () => {
    const busy = {
      ...common,
      selectedNodeIds: new Set(['selected']),
      highlightedNodeIds: new Set(['highlighted']),
    };
    expect(shouldDimEdgeFromInteraction({ ...busy, edgeId: 'one' })).toBe(false);
  });
});

describe('hovered pipe', () => {
  const hovered = { ...idle, hoveredEdgeId: 'one', inspectedEdgeIds: new Set(['two']) };

  it('emphasizes the hovered pipe in addition to the inspected ones', () => {
    expect(shouldDimEdgeFromInteraction({ ...hovered, edgeId: 'one' })).toBe(false);
    expect(shouldDimEdgeFromInteraction({ ...hovered, edgeId: 'two' })).toBe(false);
    expect(shouldDimEdgeFromInteraction({ ...hovered, edgeId: 'three' })).toBe(true);
  });

  it('dims every other edge when nothing is inspected', () => {
    const only = { ...idle, hoveredEdgeId: 'one' };
    expect(shouldDimEdgeFromInteraction({ ...only, edgeId: 'one' })).toBe(false);
    expect(shouldDimEdgeFromInteraction({ ...only, edgeId: 'two' })).toBe(true);
  });
});
