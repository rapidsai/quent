// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it } from 'vitest';
import {
  flattenStatistics,
  normalizeEdgeWidth,
  statisticFieldLabel,
  statisticFieldName,
} from './statisticFields';

describe('statistic fields', () => {
  it('distinguishes nested fields, delimiter-containing names, and repeated siblings', () => {
    const fields = flattenStatistics([
      {
        key: 'Volume',
        value: {
          kind: 'struct',
          fields: [
            { key: 'bytes', value: 1 },
            { key: 'bytes', value: 2 },
          ],
        },
      },
      { key: 'Work', value: { kind: 'struct', fields: [{ key: 'bytes', value: 3 }] } },
      { key: 'Volume › bytes', value: 4 },
      { key: '[["Volume",0],["bytes",0]]', value: 5 },
      { key: 'list', value: [1, 2] },
    ]);
    expect(new Set(fields.map(f => f.key)).size).toBe(5);
    expect(fields.map(f => f.value)).toEqual([1, 2, 3, 4, 5]);
    expect(fields.map(f => statisticFieldName(f.key))).toEqual([
      'bytes',
      'bytes',
      'bytes',
      'Volume › bytes',
      '[["Volume",0],["bytes",0]]',
    ]);
    expect(fields.slice(0, 3).map(f => statisticFieldLabel(f.key))).toEqual([
      'Volume › bytes',
      'Volume › bytes [2]',
      'Work › bytes',
    ]);
  });
  it('keeps flat field IDs stable', () => {
    expect(flattenStatistics([{ key: 'bytes', value: 0 }])[0].key).toBe('bytes');
  });
  it('uses full-range logarithmic widths with equal-value and zero handling', () => {
    expect(normalizeEdgeWidth(0, 0, 1e9)).toBe(0);
    expect(normalizeEdgeWidth(1e9, 0, 1e9)).toBe(1);
    expect(normalizeEdgeWidth(1000, 0, 1e9)).toBeGreaterThan(0.3);
    expect(normalizeEdgeWidth(7, 7, 7)).toBe(0.5);
    expect(normalizeEdgeWidth(0, 0, 0)).toBe(0.5);
  });
});
