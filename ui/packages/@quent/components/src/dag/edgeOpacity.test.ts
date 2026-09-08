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
