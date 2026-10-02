// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { AggMode } from './aggMode';

export interface NumericAggregates {
  sum: number | bigint | null;
  mean: number | null;
  min: number | bigint | null;
  max: number | bigint | null;
  stdev: number | null;
}

/** Computes numeric aggregates, preserving bigint sum/min/max when every value is bigint. */
export function aggregateNumericValues(
  values: Iterable<number | bigint>
): NumericAggregates | null {
  const numbers: number[] = [];
  const bigints: bigint[] = [];
  for (const value of values) {
    if (typeof value === 'bigint') {
      bigints.push(value);
    } else {
      numbers.push(value);
    }
  }

  const count = numbers.length + bigints.length;
  if (count === 0) {
    return null;
  }

  const normalized = [...numbers, ...bigints.map(Number)];
  let numericSum = 0;
  let numericMin = Infinity;
  let numericMax = -Infinity;
  for (const value of normalized) {
    numericSum += value;
    numericMin = Math.min(numericMin, value);
    numericMax = Math.max(numericMax, value);
  }

  let sum: number | bigint = numericSum;
  let min: number | bigint = numericMin;
  let max: number | bigint = numericMax;
  if (numbers.length === 0) {
    sum = 0n;
    min = bigints[0]!;
    max = bigints[0]!;
    for (const value of bigints) {
      sum += value;
      min = value < min ? value : min;
      max = value > max ? value : max;
    }
  }

  const mean = Number(sum) / count;
  let stdev: number | null = null;
  if (count > 1) {
    let squaredDifferenceSum = 0;
    for (const value of normalized) {
      squaredDifferenceSum += (value - mean) ** 2;
    }
    stdev = Math.sqrt(squaredDifferenceSum / (count - 1));
  }

  return { sum, mean, min, max, stdev };
}

export function getAggregateValue(
  aggregates: NumericAggregates,
  mode: AggMode
): number | bigint | null {
  switch (mode) {
    case 'mean':
      return aggregates.mean;
    case 'min':
      return aggregates.min;
    case 'max':
      return aggregates.max;
    case 'stdev':
      return aggregates.stdev;
    case 'sum':
    case 'value':
    default:
      return aggregates.sum;
  }
}

/**
 * Aggregates values with `mode` and returns the result as a plain number, or
 * `undefined` when there are no values or the mode has no result (e.g. sample
 * standard deviation of a single value). Shared by every place that derives a
 * grouped node's value from its related operators.
 */
export function aggregateToNumber(
  values: Iterable<number | bigint>,
  mode: AggMode
): number | undefined {
  const aggregates = aggregateNumericValues(values);
  if (!aggregates) {
    return undefined;
  }
  const value = getAggregateValue(aggregates, mode);
  return value === null ? undefined : Number(value);
}

export type GroupedValue<D> =
  { value: D; source: 'direct' } | { value: number; source: 'aggregated' };

/**
 * The one rule for a node that may group other operators: its own value wins;
 * otherwise its value is the numeric related values aggregated with `mode`.
 * Callers only differ in where they read the values from.
 */
export function resolveGroupedValue<D>(
  direct: D | null | undefined,
  related: Iterable<number | bigint>,
  mode: AggMode
): GroupedValue<D> | undefined {
  if (direct !== undefined && direct !== null) {
    return { value: direct, source: 'direct' };
  }
  const value = aggregateToNumber(related, mode);
  return value === undefined ? undefined : { value, source: 'aggregated' };
}
