// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, it, expect } from 'vitest';
import { continuousColor } from '@quent/utils';
import type { StatValue } from '@quent/utils';
import {
  formatNumericStat,
  itemHasId,
  formatStatValue,
  isNumericValue,
  gradientBg,
  rowGroupKey,
  getGroupKeys,
  getUniqueStatNames,
  getSchemaStatNames,
  expandRowsFromSchema,
  computeRowSpans,
  getSortValue,
  type GroupIndexDef,
} from './utils';
import type {
  StatGroupExpandedRow,
  PivotedRow,
  PivotedRowAgg,
  PivotedStatTableSchema,
} from './types';

// ---- Helpers ---------------------------------------------------------------

function makeExpandedRow(
  statisticName: string,
  value: StatValue,
  groups: Record<string, { id: string; label?: string }> = {},
  opts: { itemId?: string; scopeId?: string; itemType?: string } = {}
): StatGroupExpandedRow {
  const normalized: Record<string, { id: string; label: string }> = {};
  for (const [k, v] of Object.entries(groups)) {
    normalized[k] = { id: v.id, label: v.label ?? v.id };
  }
  return {
    groups: normalized,
    itemType: opts.itemType ?? '-',
    itemId: opts.itemId ?? 'item-1',
    scopeId: opts.scopeId ?? 'scope-1',
    statisticName,
    value,
  };
}

function makeGroupIdx(key: string): GroupIndexDef {
  return {
    key,
    getId: r => r.groups[key]?.id ?? '',
    getLabel: r => r.groups[key]?.label ?? '',
  };
}

function makePivotedRow(
  values: Record<string, StatValue>,
  aggs: Record<string, Partial<PivotedRowAgg>> = {}
): PivotedRow {
  const aggMap = new Map<string, PivotedRowAgg>();
  for (const [k, v] of Object.entries(aggs)) {
    aggMap.set(k, {
      sum: v.sum ?? null,
      mean: v.mean ?? null,
      min: v.min ?? null,
      max: v.max ?? null,
      stdev: v.stdev ?? null,
      count: v.count ?? 0,
      isNumeric: v.isNumeric ?? true,
    });
  }
  return {
    groupKeys: [],
    rowKey: '',
    values: new Map(Object.entries(values)),
    aggs: aggMap,
    itemIds: new Set(),
    itemType: '-',
    itemScopeIds: new Map(),
  };
}

// ---- formatNumericStat -----------------------------------------------------

describe('formatNumericStat', () => {
  it('returns "-" for null', () => {
    expect(formatNumericStat(null, 'rows')).toBe('-');
  });

  it('delegates to inferFieldFormatter for a number', () => {
    // inferFieldFormatter('exec_ns') divides by 1e6 before formatting
    const result = formatNumericStat(1_000_000, 'exec_ns');
    expect(typeof result).toBe('string');
    expect(result).not.toBe('-');
  });

  it('uses the field name to pick the right formatter (bytes suffix)', () => {
    const bytesResult = formatNumericStat(1024, 'output_bytes');
    const numericResult = formatNumericStat(1024, 'row_count');
    // bytes formatter produces a different string than the plain number formatter
    expect(bytesResult).not.toBe(numericResult);
  });
});

// ---- itemHasId -------------------------------------------------------------

describe('itemHasId', () => {
  it('returns true when the first item matches', () => {
    expect(itemHasId(['a', 'b', 'c'], new Set(['a']))).toBe(true);
  });

  it('returns true when a later item matches', () => {
    expect(itemHasId(['a', 'b', 'c'], new Set(['c']))).toBe(true);
  });

  it('returns false when no item matches', () => {
    expect(itemHasId(['a', 'b'], new Set(['x', 'y']))).toBe(false);
  });

  it('returns false for an empty iterable', () => {
    expect(itemHasId([], new Set(['a']))).toBe(false);
  });

  it('returns false when the target set is empty', () => {
    expect(itemHasId(['a', 'b'], new Set())).toBe(false);
  });
});

// ---- formatStatValue -------------------------------------------------------

describe('formatStatValue', () => {
  it('returns "-" for null', () => {
    expect(formatStatValue(null, 'rows')).toBe('-');
  });

  it('returns "-" for undefined', () => {
    expect(formatStatValue(undefined, 'rows')).toBe('-');
  });

  it('formats a number using inferFieldFormatter', () => {
    const result = formatStatValue(42, 'row_count');
    expect(typeof result).toBe('string');
    expect(result).not.toBe('-');
  });

  it('returns "true" for boolean true', () => {
    expect(formatStatValue(true, 'enabled')).toBe('true');
  });

  it('returns "false" for boolean false', () => {
    expect(formatStatValue(false, 'enabled')).toBe('false');
  });

  it('joins an array with ", "', () => {
    expect(formatStatValue(['a', 'b', 'c'], 'tags')).toBe('a, b, c');
  });

  it('converts a string value using String()', () => {
    expect(formatStatValue('hello', 'label')).toBe('hello');
  });
});

// ---- isNumericValue --------------------------------------------------------

describe('isNumericValue', () => {
  it('returns true for a number', () => {
    expect(isNumericValue(42)).toBe(true);
  });

  it('returns false for null', () => {
    expect(isNumericValue(null)).toBe(false);
  });

  it('returns false for a string', () => {
    expect(isNumericValue('42')).toBe(false);
  });

  it('returns false for a boolean', () => {
    expect(isNumericValue(true)).toBe(false);
  });

  it('returns false for an array', () => {
    expect(isNumericValue([1, 2])).toBe(false);
  });
});

// ---- gradientBg ------------------------------------------------------------

describe('gradientBg', () => {
  it('returns undefined when min equals max', () => {
    expect(gradientBg(5, 5, 5)).toBeUndefined();
  });

  it('returns the neutral color at the minimum (t=0)', () => {
    const result = gradientBg(0, 0, 10, 'blue', false);
    expect(result).toBe(continuousColor(0, 'blue', false));
  });

  it('returns the full color at the maximum (t=1)', () => {
    const result = gradientBg(10, 0, 10, 'blue', false);
    expect(result).toBe(continuousColor(1, 'blue', false));
  });

  it('interpolates at the midpoint (t=0.5)', () => {
    const result = gradientBg(5, 0, 10, 'teal', false);
    expect(result).toBe(continuousColor(0.5, 'teal', false));
  });

  it('passes darkMode to continuousColor', () => {
    const light = gradientBg(10, 0, 10, 'blue', false);
    const dark = gradientBg(10, 0, 10, 'blue', true);
    expect(light).toBe(continuousColor(1, 'blue', false));
    expect(dark).toBe(continuousColor(1, 'blue', true));
  });
});

// ---- rowGroupKey -----------------------------------------------------------

describe('rowGroupKey', () => {
  it('joins a single index id', () => {
    const row = makeExpandedRow('x', 1, { brand: { id: 'Ford' } });
    const result = rowGroupKey(row, [makeGroupIdx('brand')]);
    expect(result).toBe('Ford');
  });

  it('joins multiple index ids with a null byte separator', () => {
    const row = makeExpandedRow('x', 1, {
      brand: { id: 'Ford' },
      fuel: { id: 'Hybrid' },
    });
    const result = rowGroupKey(row, [makeGroupIdx('brand'), makeGroupIdx('fuel')]);
    expect(result).toBe('Ford\0Hybrid');
  });

  it('returns an empty string for zero indices', () => {
    const row = makeExpandedRow('x', 1, { brand: { id: 'Ford' } });
    expect(rowGroupKey(row, [])).toBe('');
  });
});

// ---- getGroupKeys ----------------------------------------------------------

describe('getGroupKeys', () => {
  it('maps indices to GroupKeyEntry objects', () => {
    const row = makeExpandedRow('x', 1, {
      brand: { id: 'Ford', label: 'Ford Motor Co' },
      fuel: { id: 'Hybrid' },
    });
    const result = getGroupKeys(row, [makeGroupIdx('brand'), makeGroupIdx('fuel')]);
    expect(result).toEqual([
      { key: 'brand', id: 'Ford', label: 'Ford Motor Co' },
      { key: 'fuel', id: 'Hybrid', label: 'Hybrid' },
    ]);
  });

  it('returns an empty array for zero indices', () => {
    const row = makeExpandedRow('x', 1, { brand: { id: 'Ford' } });
    expect(getGroupKeys(row, [])).toEqual([]);
  });
});

// ---- getUniqueStatNames ----------------------------------------------------

describe('getUniqueStatNames', () => {
  it('returns [] for an empty array', () => {
    expect(getUniqueStatNames([])).toEqual([]);
  });

  it('returns stat names in first-appearance order', () => {
    const rows = [
      makeExpandedRow('exec_ns', 100),
      makeExpandedRow('output_bytes', 1024),
      makeExpandedRow('exec_ns', 200),
    ];
    expect(getUniqueStatNames(rows)).toEqual(['exec_ns', 'output_bytes']);
  });

  it('deduplicates without changing non-duplicate order', () => {
    const rows = [
      makeExpandedRow('c', 1),
      makeExpandedRow('a', 1),
      makeExpandedRow('b', 1),
      makeExpandedRow('a', 1),
    ];
    expect(getUniqueStatNames(rows)).toEqual(['c', 'a', 'b']);
  });
});

// ---- getSchemaStatNames ----------------------------------------------------

describe('getSchemaStatNames', () => {
  const schema: PivotedStatTableSchema<{ id: string; stats: Record<string, number> }> = {
    groups: {},
    itemId: r => r.id,
    scopeId: r => r.id,
    stats: r => r.stats,
  };

  it('returns [] for empty rows', () => {
    expect(getSchemaStatNames([], schema)).toEqual([]);
  });

  it('returns unique stat names in first-appearance order', () => {
    const rows = [
      { id: 'a', stats: { exec_ns: 1, rows: 2 } },
      { id: 'b', stats: { rows: 3, bytes: 4 } },
    ];
    expect(getSchemaStatNames(rows, schema)).toEqual(['exec_ns', 'rows', 'bytes']);
  });

  it('deduplicates across rows', () => {
    const rows = [
      { id: 'a', stats: { x: 1 } },
      { id: 'b', stats: { x: 2 } },
    ];
    expect(getSchemaStatNames(rows, schema)).toEqual(['x']);
  });
});

// ---- expandRowsFromSchema --------------------------------------------------

describe('expandRowsFromSchema', () => {
  type SimpleRow = { id: string; plan: string; stats: Record<string, StatValue> };

  const schema: PivotedStatTableSchema<SimpleRow> = {
    groups: {
      partition: { id: r => r.plan, label: r => `Plan ${r.plan}` },
    },
    itemId: r => r.id,
    scopeId: r => r.plan,
    stats: r => r.stats,
  };

  it('returns [] for empty rows', () => {
    expect(expandRowsFromSchema([], schema)).toEqual([]);
  });

  it('produces one expanded row per stat entry', () => {
    const rows: SimpleRow[] = [{ id: 'op1', plan: 'p1', stats: { exec_ns: 100, rows: 50 } }];
    const result = expandRowsFromSchema(rows, schema);
    expect(result).toHaveLength(2);
    expect(result.map(r => r.statisticName).sort()).toEqual(['exec_ns', 'rows'].sort());
  });

  it('populates groups from the schema selectors', () => {
    const rows: SimpleRow[] = [{ id: 'op1', plan: 'p1', stats: { x: 1 } }];
    const [row] = expandRowsFromSchema(rows, schema);
    expect(row!.groups.partition).toEqual({ id: 'p1', label: 'Plan p1' });
  });

  it('sets itemId and scopeId from the schema', () => {
    const rows: SimpleRow[] = [{ id: 'op1', plan: 'p1', stats: { x: 1 } }];
    const [row] = expandRowsFromSchema(rows, schema);
    expect(row!.itemId).toBe('op1');
    expect(row!.scopeId).toBe('p1');
  });

  it('uses schema.itemType when provided', () => {
    const schemaWithType: PivotedStatTableSchema<SimpleRow> = {
      ...schema,
      itemType: r => `TYPE:${r.id}`,
    };
    const rows: SimpleRow[] = [{ id: 'op1', plan: 'p1', stats: { x: 1 } }];
    const [row] = expandRowsFromSchema(rows, schemaWithType);
    expect(row!.itemType).toBe('TYPE:op1');
  });

  it('falls back to groups.item_type, then groups.item, then groups.partition, then "-"', () => {
    // Fall back to groups.partition since item_type and item are not set
    const rows: SimpleRow[] = [{ id: 'op1', plan: 'p1', stats: { x: 1 } }];
    const [row] = expandRowsFromSchema(rows, schema);
    expect(row!.itemType).toBe('p1');
  });

  it('uses label fallback equal to id when schema.label is undefined', () => {
    const schemaNoLabel: PivotedStatTableSchema<SimpleRow> = {
      ...schema,
      groups: { partition: { id: r => r.plan } },
    };
    const rows: SimpleRow[] = [{ id: 'op1', plan: 'p1', stats: { x: 1 } }];
    const [row] = expandRowsFromSchema(rows, schemaNoLabel);
    expect(row!.groups.partition).toEqual({ id: 'p1', label: 'p1' });
  });
});

// ---- computeRowSpans -------------------------------------------------------

describe('computeRowSpans', () => {
  it('returns an empty array for empty rows', () => {
    expect(computeRowSpans([])).toEqual([]);
  });

  it('assigns span=1 to every row when all group ids are distinct', () => {
    const rows = [
      { groupKeys: [{ id: 'a' }] },
      { groupKeys: [{ id: 'b' }] },
      { groupKeys: [{ id: 'c' }] },
    ];
    expect(computeRowSpans(rows)).toEqual([[1], [1], [1]]);
  });

  it('merges consecutive same-id rows in the outer column', () => {
    const rows = [
      { groupKeys: [{ id: 'a' }, { id: 'x' }] },
      { groupKeys: [{ id: 'a' }, { id: 'y' }] },
      { groupKeys: [{ id: 'b' }, { id: 'z' }] },
    ];
    const spans = computeRowSpans(rows);
    // col 0: 'a' spans rows 0-1, 'b' spans row 2
    expect(spans[0]![0]).toBe(2);
    expect(spans[1]![0]).toBeNull();
    expect(spans[2]![0]).toBe(1);
  });

  it('merges a column when both the id and its parent are the same', () => {
    // rows 0 and 1 share the same (col0, col1) pair
    const rows = [
      { groupKeys: [{ id: 'a' }, { id: 'x' }] },
      { groupKeys: [{ id: 'a' }, { id: 'x' }] },
      { groupKeys: [{ id: 'b' }, { id: 'z' }] },
    ];
    const spans = computeRowSpans(rows);
    expect(spans[0]![0]).toBe(2); // 'a' spans 2 rows
    expect(spans[0]![1]).toBe(2); // 'x' under 'a' also spans 2
    expect(spans[1]![0]).toBeNull();
    expect(spans[1]![1]).toBeNull();
  });

  it('splits inner spans when the parent group changes', () => {
    // 'x' appears under both 'a' and 'b' — must not be merged across parents
    const rows = [
      { groupKeys: [{ id: 'a' }, { id: 'x' }] },
      { groupKeys: [{ id: 'b' }, { id: 'x' }] },
    ];
    const spans = computeRowSpans(rows);
    expect(spans[0]![1]).toBe(1);
    expect(spans[1]![1]).toBe(1);
  });
});

// ---- getSortValue ----------------------------------------------------------

describe('getSortValue', () => {
  describe('non-aggregating mode (isAgg=false)', () => {
    it('returns the numeric value directly', () => {
      const row = makePivotedRow({ exec_ns: 42 });
      expect(getSortValue(row, 'exec_ns', false, 'sum')).toBe(42);
    });

    it('returns null when the stat is missing', () => {
      const row = makePivotedRow({});
      expect(getSortValue(row, 'exec_ns', false, 'sum')).toBeNull();
    });

    it('returns null for a non-numeric value', () => {
      const row = makePivotedRow({ label: 'hello' });
      expect(getSortValue(row, 'label', false, 'sum')).toBeNull();
    });
  });

  describe('aggregating mode (isAgg=true)', () => {
    it('returns null when the stat has no agg entry', () => {
      const row = makePivotedRow({});
      expect(getSortValue(row, 'exec_ns', true, 'sum')).toBeNull();
    });

    it('returns null when the agg is not numeric', () => {
      const row = makePivotedRow({}, { exec_ns: { isNumeric: false, count: 2 } });
      expect(getSortValue(row, 'exec_ns', true, 'sum')).toBeNull();
    });

    it('returns agg.sum for aggMode "sum"', () => {
      const row = makePivotedRow({}, { exec_ns: { sum: 100, mean: 50, min: 10, max: 90 } });
      expect(getSortValue(row, 'exec_ns', true, 'sum')).toBe(100);
    });

    it('returns agg.mean for aggMode "mean"', () => {
      const row = makePivotedRow({}, { exec_ns: { sum: 100, mean: 50, min: 10, max: 90 } });
      expect(getSortValue(row, 'exec_ns', true, 'mean')).toBe(50);
    });

    it('returns agg.min for aggMode "min"', () => {
      const row = makePivotedRow({}, { exec_ns: { sum: 100, mean: 50, min: 10, max: 90 } });
      expect(getSortValue(row, 'exec_ns', true, 'min')).toBe(10);
    });

    it('returns agg.max for aggMode "max"', () => {
      const row = makePivotedRow({}, { exec_ns: { sum: 100, mean: 50, min: 10, max: 90 } });
      expect(getSortValue(row, 'exec_ns', true, 'max')).toBe(90);
    });

    it('returns agg.stdev for aggMode "stdev"', () => {
      const row = makePivotedRow(
        {},
        { exec_ns: { sum: 100, mean: 50, min: 10, max: 90, stdev: 5 } }
      );
      expect(getSortValue(row, 'exec_ns', true, 'stdev')).toBe(5);
    });
  });
});
