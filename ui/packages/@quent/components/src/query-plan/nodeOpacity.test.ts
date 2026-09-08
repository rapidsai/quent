// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it } from 'vitest';
import { getNodeOpacityClass } from './nodeOpacity';

describe('getNodeOpacityClass', () => {
  it('keeps every selected node highlighted during a DAG hover', () => {
    expect(
      getNodeOpacityClass({
        hoveredStatValues: null,
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
        hoveredStatValues: null,
        highlightedNodeIds: new Set(['hovered']),
        operatorId: 'other',
        isDimmed: true,
        isSelected: false,
      })
    ).toBe('opacity-35');
  });
});
