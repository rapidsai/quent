// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it } from 'vitest';
import { resolveHoveredStatValue } from './hoveredStatValue';
import type { HoveredStatInfo } from '../atoms/dagControls';

function stat(values: Record<string, number>, aggMode: HoveredStatInfo['aggMode'] = 'sum') {
  return {
    name: 'rows',
    values: new Map(Object.entries(values)),
    min: 0,
    max: 100,
    aggMode,
  } satisfies HoveredStatInfo;
}

describe('resolveHoveredStatValue', () => {
  it('returns the direct value for a physical operator with its own entry', () => {
    expect(resolveHoveredStatValue(stat({ a: 5, b: 10 }), 'a', ['b'])).toBe(5);
  });

  it('sums related operator values for a logical node with no entry of its own', () => {
    expect(resolveHoveredStatValue(stat({ b: 5, c: 10 }, 'sum'), 'a', ['b', 'c'])).toBe(15);
  });

  it('averages related operator values when aggMode is mean', () => {
    expect(resolveHoveredStatValue(stat({ b: 5, c: 15 }, 'mean'), 'a', ['b', 'c'])).toBe(10);
  });

  it('returns undefined when sample standard deviation has fewer than two values', () => {
    expect(resolveHoveredStatValue(stat({ b: 5 }, 'stdev'), 'a', ['b'])).toBeUndefined();
  });

  it('ignores related ids with no value', () => {
    expect(resolveHoveredStatValue(stat({ b: 5 }, 'sum'), 'a', ['b', 'missing'])).toBe(5);
  });

  it('returns undefined when neither the node nor any related operator has a value', () => {
    expect(resolveHoveredStatValue(stat({ x: 1 }), 'a', ['b', 'c'])).toBeUndefined();
  });

  it('returns undefined for a node with no related ids and no direct value', () => {
    expect(resolveHoveredStatValue(stat({ x: 1 }), 'a')).toBeUndefined();
  });
});
