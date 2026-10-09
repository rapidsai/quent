// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, it, expect } from 'vitest';
import { EDGE_SCALE_TYPE } from '@quent/utils';
import { edgeScaleMidpoint, normalizeEdgeValue } from './edgeScale';

// Values 0, 3, 15 and 63: the log scale starts at the smallest positive value (3),
// the linear scale at the true minimum (0).
const range = { min: 0, logMin: 3, max: 63 };

describe('normalizeEdgeValue', () => {
  it('log: starts at the smallest positive value, so a zero sits at the bottom', () => {
    expect(normalizeEdgeValue(0, range, EDGE_SCALE_TYPE.LOG)).toBe(0);
    expect(normalizeEdgeValue(3, range, EDGE_SCALE_TYPE.LOG)).toBe(0);
    expect(normalizeEdgeValue(15, range, EDGE_SCALE_TYPE.LOG)).toBeCloseTo(0.5, 9);
    expect(normalizeEdgeValue(63, range, EDGE_SCALE_TYPE.LOG)).toBe(1);
  });

  it('linear: starts at the true minimum and keeps proportions', () => {
    expect(normalizeEdgeValue(0, range, EDGE_SCALE_TYPE.LINEAR)).toBe(0);
    expect(normalizeEdgeValue(3, range, EDGE_SCALE_TYPE.LINEAR)).toBeCloseTo(3 / 63, 9);
    expect(normalizeEdgeValue(15, range, EDGE_SCALE_TYPE.LINEAR)).toBeCloseTo(15 / 63, 9);
    expect(normalizeEdgeValue(63, range, EDGE_SCALE_TYPE.LINEAR)).toBe(1);
  });
});

describe('edgeScaleMidpoint', () => {
  it('log: the value halfway along the log scale', () => {
    expect(edgeScaleMidpoint(range, EDGE_SCALE_TYPE.LOG)).toBeCloseTo(15, 9);
  });

  it('linear: the arithmetic middle of the true range', () => {
    expect(edgeScaleMidpoint(range, EDGE_SCALE_TYPE.LINEAR)).toBe(31.5);
  });
});
