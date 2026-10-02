// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it } from 'vitest';
import {
  aggregateNumericValues,
  aggregateToNumber,
  getAggregateValue,
  resolveGroupedValue,
} from './statAggregation';

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

describe('aggregateToNumber', () => {
  it('aggregates values with the requested mode as a plain number', () => {
    expect(aggregateToNumber([2, 4n, 6], 'sum')).toBe(12);
    expect(aggregateToNumber([2, 4, 6], 'mean')).toBe(4);
    expect(aggregateToNumber([2n, 6n], 'max')).toBe(6);
  });

  it('returns undefined for no values', () => {
    expect(aggregateToNumber([], 'sum')).toBeUndefined();
  });

  it('returns undefined when the mode has no value (stdev of one item)', () => {
    expect(aggregateToNumber([5], 'stdev')).toBeUndefined();
  });
});

describe('resolveGroupedValue', () => {
  it('prefers the direct value and ignores related values', () => {
    expect(resolveGroupedValue(5, [10, 20], 'sum')).toEqual({ value: 5, source: 'direct' });
  });

  it('keeps a non-numeric direct value as-is', () => {
    expect(resolveGroupedValue('scan', [1], 'sum')).toEqual({ value: 'scan', source: 'direct' });
  });

  it('aggregates related values when there is no direct value', () => {
    expect(resolveGroupedValue(undefined, [4, 6n], 'sum')).toEqual({
      value: 10,
      source: 'aggregated',
    });
    expect(resolveGroupedValue(null, [2, 4], 'mean')).toEqual({ value: 3, source: 'aggregated' });
  });

  it('returns undefined with no direct value and nothing to aggregate', () => {
    expect(resolveGroupedValue(undefined, [], 'sum')).toBeUndefined();
    expect(resolveGroupedValue(undefined, [5], 'stdev')).toBeUndefined();
  });
});
