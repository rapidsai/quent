// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, it, expect } from 'vitest';
import { SCALE_TYPE } from './dagTypes';
import { continuousColor } from './colors';
import { computeScaleRange, scaleColor, scaleMidpoint, normalizeScaleValue } from './scale';

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

describe('computeScaleRange', () => {
  it('starts the log scale at the smallest positive value for non-negative data', () => {
    expect(computeScaleRange([0, 3, 15, 63])).toEqual(range);
  });

  it('keeps the true minimum for signed data', () => {
    expect(computeScaleRange([-5, 0, 7])).toEqual({ min: -5, logMin: -5, max: 7 });
  });

  it('keeps the true minimum when nothing is positive', () => {
    expect(computeScaleRange([0, 0])).toEqual({ min: 0, logMin: 0, max: 0 });
  });
});

describe('scaleColor', () => {
  it('places the value on the chosen scale before coloring', () => {
    for (const scale of [SCALE_TYPE.LOG, SCALE_TYPE.LINEAR]) {
      const t = normalizeScaleValue(15, range, scale);
      expect(scaleColor(15, range, scale, 'blue', false)).toBe(continuousColor(t, 'blue', false));
    }
    expect(scaleColor(15, range, SCALE_TYPE.LOG, 'blue', false)).not.toBe(
      scaleColor(15, range, SCALE_TYPE.LINEAR, 'blue', false)
    );
  });
});
