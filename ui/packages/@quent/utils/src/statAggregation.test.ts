// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it } from 'vitest';
import { aggregateNumericValues, getAggregateValue } from './statAggregation';

describe('aggregateNumericValues', () => {
  it('computes number aggregates with sample standard deviation', () => {
    expect(aggregateNumericValues([2, 4, 6])).toEqual({
      sum: 12,
      mean: 4,
      min: 2,
      max: 6,
      stdev: 2,
    });
  });

  it('returns null sample standard deviation for a singleton', () => {
    expect(aggregateNumericValues([5])?.stdev).toBeNull();
  });

  it('preserves bigint sum, min, and max', () => {
    expect(aggregateNumericValues([1000n, 3000n])).toEqual({
      sum: 4000n,
      mean: 2000,
      min: 1000n,
      max: 3000n,
      stdev: Math.sqrt(2_000_000),
    });
  });

  it('uses number aggregates for mixed number and bigint input', () => {
    expect(aggregateNumericValues([1, 3n])).toEqual({
      sum: 4,
      mean: 2,
      min: 1,
      max: 3,
      stdev: Math.sqrt(2),
    });
  });

  it('returns null for empty input', () => {
    expect(aggregateNumericValues([])).toBeNull();
  });
});

describe('getAggregateValue', () => {
  const aggregates = aggregateNumericValues([2, 4, 6])!;

  it.each([
    ['sum', 12],
    ['value', 12],
    ['mean', 4],
    ['min', 2],
    ['max', 6],
    ['stdev', 2],
  ] as const)('selects the %s aggregate', (mode, expected) => {
    expect(getAggregateValue(aggregates, mode)).toBe(expected);
  });
});
