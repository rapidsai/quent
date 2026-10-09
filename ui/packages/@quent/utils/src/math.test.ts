// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, it, expect } from 'vitest';
import { logScaleValueAt, normalizeLinearScale, normalizeLogScale } from './math';

describe('normalizeLinearScale', () => {
  it('maps min to 0 and max to 1 with proportional values between', () => {
    expect(normalizeLinearScale(0, 0, 100)).toBe(0);
    expect(normalizeLinearScale(25, 0, 100)).toBe(0.25);
    expect(normalizeLinearScale(100, 0, 100)).toBe(1);
  });

  it('clamps values outside the range', () => {
    expect(normalizeLinearScale(-5, 0, 100)).toBe(0);
    expect(normalizeLinearScale(500, 0, 100)).toBe(1);
  });

  it('works for signed ranges', () => {
    expect(normalizeLinearScale(0, -100, 100)).toBe(0.5);
  });

  it('returns the neutral midpoint for constant or invalid ranges', () => {
    expect(normalizeLinearScale(5, 5, 5)).toBe(0.5);
    expect(normalizeLinearScale(5, 10, 0)).toBe(0.5);
    expect(normalizeLinearScale(Number.NaN, 0, 10)).toBe(0.5);
    expect(normalizeLinearScale(5, 0, Infinity)).toBe(0.5);
  });
});

describe('logScaleValueAt', () => {
  it('returns the range ends at positions 0 and 1', () => {
    expect(logScaleValueAt(0, 3, 63)).toBeCloseTo(3, 9);
    expect(logScaleValueAt(1, 3, 63)).toBeCloseTo(63, 9);
  });

  it('is the inverse of normalizeLogScale', () => {
    for (const value of [3, 10, 15, 40, 63]) {
      const t = normalizeLogScale(value, 3, 63);
      expect(logScaleValueAt(t, 3, 63)).toBeCloseTo(value, 9);
    }
  });

  it('puts the midpoint at the geometric middle of the range', () => {
    // log1p(3) = ln 4 and log1p(63) = ln 64, so the middle is ln 16, i.e. a value of 15.
    expect(logScaleValueAt(0.5, 3, 63)).toBeCloseTo(15, 9);
  });

  it('inverts signed ranges', () => {
    expect(logScaleValueAt(0.5, -100, 100)).toBeCloseTo(0, 9);
    const t = normalizeLogScale(-10, -100, 100);
    expect(logScaleValueAt(t, -100, 100)).toBeCloseTo(-10, 9);
  });

  it('clamps the position and returns min for constant or invalid ranges', () => {
    expect(logScaleValueAt(-1, 3, 63)).toBeCloseTo(3, 9);
    expect(logScaleValueAt(2, 3, 63)).toBeCloseTo(63, 9);
    expect(logScaleValueAt(0.5, 7, 7)).toBe(7);
    expect(logScaleValueAt(0.5, 10, 0)).toBe(10);
  });
});
