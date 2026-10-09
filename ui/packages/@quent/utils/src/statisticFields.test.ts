// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it } from 'vitest';
import {
  flattenStatistics,
  statisticFieldId,
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
    expect(fields[4].key).toBe('[["Volume",0],["bytes",0]]');
    expect(fields.map(f => statisticFieldName(f))).toEqual([
      'bytes',
      'bytes',
      'bytes',
      'Volume › bytes',
      '[["Volume",0],["bytes",0]]',
    ]);
    expect(fields.slice(0, 3).map(f => statisticFieldLabel(f))).toEqual([
      'Volume › bytes',
      'Volume › bytes [2]',
      'Work › bytes',
    ]);
  });
  it('keeps flat field IDs stable', () => {
    expect(flattenStatistics([{ key: 'bytes', value: 0 }])[0].key).toBe('bytes');
  });

  it('resolves selected field IDs while preserving name and empty fallbacks', () => {
    const fields = flattenStatistics([
      { key: 'Volume', value: { kind: 'struct', fields: [{ key: 'bytes', value: 1 }] } },
    ]);
    expect(statisticFieldName(fields[0].key, fields)).toBe('bytes');
    expect(statisticFieldName('unknown', fields)).toBe('unknown');
    expect(statisticFieldName(null, fields)).toBe('');
  });

  it('distinguishes literal path separators from nested paths in labels', () => {
    const fields = flattenStatistics([
      { key: 'Volume', value: { kind: 'struct', fields: [{ key: 'bytes', value: 1 }] } },
      { key: 'Volume › bytes', value: 2 },
    ]);
    expect(fields.map(statisticFieldLabel)).toEqual(['Volume › bytes', '"Volume › bytes"']);
  });

  it('retains long field names and explicit structured paths', () => {
    const name = 'very long producer name '.repeat(100);
    const fields = flattenStatistics([
      { key: name, value: 1 },
      { key: name, value: { kind: 'struct', fields: [{ key: name, value: 2 }] } },
    ]);
    expect(fields.map(statisticFieldName)).toEqual([name, name]);
    expect(fields[1].path).toEqual([
      [name, 1],
      [name, 0],
    ]);
    expect(fields[1].key).toBe(statisticFieldId(fields[1].path));
  });

  it('keeps nested identities stable when values or unrelated sibling order change', () => {
    const first = flattenStatistics([
      { key: 'Volume', value: { kind: 'struct', fields: [{ key: 'bytes', value: 1 }] } },
    ])[0];
    const second = flattenStatistics([
      { key: 'other', value: 5 },
      { key: 'Volume', value: { kind: 'struct', fields: [{ key: 'bytes', value: 99 }] } },
    ])[1];
    expect(first.key).toBe(second.key);
    expect(first.key).toBe(
      statisticFieldId([
        ['Volume', 0],
        ['bytes', 0],
      ])
    );
  });

  it('distinguishes occurrence suffixes and reserved IDs from literal producer names', () => {
    const repeated = flattenStatistics([
      { key: 'bytes', value: 1 },
      { key: 'bytes', value: 2 },
    ]);
    const fields = flattenStatistics([
      { key: 'bytes', value: 1 },
      { key: 'bytes', value: 2 },
      { key: 'bytes [2]', value: 3 },
      { key: repeated[1].key, value: 4 },
    ]);
    expect(new Set(fields.map(field => field.key)).size).toBe(4);
    expect(new Set(fields.map(statisticFieldLabel)).size).toBe(4);
  });
});
