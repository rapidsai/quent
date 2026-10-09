// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, it, expect } from 'vitest';
import { SCALE_TYPE } from './dagTypes';
import { scaleMidpoint, normalizeScaleValue } from './scale';

// Values 0, 3, 15 and 63: the log scale starts at the smallest positive value (3),
// the linear scale at the true minimum (0).
const range = { min: 0, logMin: 3, max: 63 };

describe('normalizeScaleValue', () => {
  it('log: starts at the smallest positive value, so a zero sits at the bottom', () => {
    expect(normalizeScaleValue(0, range, SCALE_TYPE.LOG)).toBe(0);
    expect(normalizeScaleValue(3, range, SCALE_TYPE.LOG)).toBe(0);
    expect(normalizeScaleValue(15, range, SCALE_TYPE.LOG)).toBeCloseTo(0.5, 9);
    expect(normalizeScaleValue(63, range, SCALE_TYPE.LOG)).toBe(1);
  });

  it('linear: starts at the true minimum and keeps proportions', () => {
    expect(normalizeScaleValue(0, range, SCALE_TYPE.LINEAR)).toBe(0);
    expect(normalizeScaleValue(3, range, SCALE_TYPE.LINEAR)).toBeCloseTo(3 / 63, 9);
    expect(normalizeScaleValue(15, range, SCALE_TYPE.LINEAR)).toBeCloseTo(15 / 63, 9);
    expect(normalizeScaleValue(63, range, SCALE_TYPE.LINEAR)).toBe(1);
  });
});

describe('scaleMidpoint', () => {
  it('log: the value halfway along the log scale', () => {
    expect(scaleMidpoint(range, SCALE_TYPE.LOG)).toBeCloseTo(15, 9);
  });

  it('linear: the arithmetic middle of the true range', () => {
    expect(scaleMidpoint(range, SCALE_TYPE.LINEAR)).toBe(31.5);
  });
});
