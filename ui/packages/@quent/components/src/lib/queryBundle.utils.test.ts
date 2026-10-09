// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, it, expect } from 'vitest';
import { flattenStatistics } from '@quent/utils';
import {
  entityRefToEntitiesKey,
  ENTITY_REF_TO_ENTITIES_KEY,
  parseCustomStatistics,
  parsePortStatistics,
  resolveStatisticFields,
} from './queryBundle.utils';

// ---- entityRefToEntitiesKey -----------------------------------------------

describe('entityRefToEntitiesKey', () => {
  it('maps Engine to engine', () => {
    expect(entityRefToEntitiesKey('Engine')).toBe('engine');
  });

  it('maps Query to query', () => {
    expect(entityRefToEntitiesKey('Query')).toBe('query');
  });

  it('maps Plan to plans', () => {
    expect(entityRefToEntitiesKey('Plan')).toBe('plans');
  });

  it('maps Worker to workers', () => {
    expect(entityRefToEntitiesKey('Worker')).toBe('workers');
  });

  it('maps Operator to operators', () => {
    expect(entityRefToEntitiesKey('Operator')).toBe('operators');
  });

  it('maps Port to ports', () => {
    expect(entityRefToEntitiesKey('Port')).toBe('ports');
  });

  it('maps ResourceGroup to resource_groups', () => {
    expect(entityRefToEntitiesKey('ResourceGroup')).toBe('resource_groups');
  });

  it('maps Resource to resources', () => {
    expect(entityRefToEntitiesKey('Resource')).toBe('resources');
  });

  it('returns undefined for Task (no QueryEntities collection)', () => {
    expect(entityRefToEntitiesKey('Task')).toBeUndefined();
  });

  it('ENTITY_REF_TO_ENTITIES_KEY contains exactly the expected entries', () => {
    expect(Object.keys(ENTITY_REF_TO_ENTITIES_KEY)).toHaveLength(8);
  });
});

// ---- parseCustomStatistics -------------------------------------------------

// Helpers to build the shapes that the Rust-generated tagged unions produce.
function makeTagged(variant: string, value: unknown) {
  return { [variant]: value };
}

function makeOperator(custom_statistics: Record<string, unknown> | undefined) {
  return {
    statistics:
      custom_statistics !== undefined
        ? {
            custom_statistics: Object.entries(custom_statistics).map(([key, value]) => ({
              value: { key, value },
              quantity: null,
            })),
          }
        : undefined,
  };
}

describe('parseCustomStatistics', () => {
  it('returns [] for null/undefined input', () => {
    expect(parseCustomStatistics(null)).toEqual([]);
    expect(parseCustomStatistics(undefined)).toEqual([]);
  });

  it('returns [] when statistics is absent', () => {
    expect(parseCustomStatistics(makeOperator(undefined))).toEqual([]);
  });

  it('returns [] when custom_statistics is empty', () => {
    expect(parseCustomStatistics(makeOperator({}))).toEqual([]);
  });

  it('unwraps a number tagged value', () => {
    const op = makeOperator({ rows: makeTagged('UInt64', 42) });
    const result = parseCustomStatistics(op);
    expect(result).toEqual([{ key: 'rows', value: 42 }]);
  });

  it('omits null and absent quantities while preserving empty quantity keys and falsy values', () => {
    expect(
      parseCustomStatistics({
        statistics: {
          custom_statistics: [
            { value: { key: 'zero', value: 0 }, quantity: null },
            { value: { key: 'false', value: false } },
            { value: { key: 'empty', value: '' }, quantity: '' },
            { value: { key: 'missing', value: undefined } },
          ],
        },
      })
    ).toEqual([
      { key: 'zero', value: 0 },
      { key: 'false', value: false },
      { key: 'empty', value: '', quantity: '' },
      { key: 'missing', value: null },
    ]);
  });

  it('preserves a quantity key', () => {
    const op = {
      statistics: {
        custom_statistics: [
          { value: { key: 'bytes', value: makeTagged('UInt64', 1024) }, quantity: 'bytes' },
        ],
      },
    };
    expect(parseCustomStatistics(op)).toEqual([{ key: 'bytes', value: 1024, quantity: 'bytes' }]);
  });

  it('unwraps a string tagged value', () => {
    const op = makeOperator({ label: makeTagged('String', 'hello') });
    expect(parseCustomStatistics(op)).toEqual([{ key: 'label', value: 'hello' }]);
  });

  it('produces null for a null-valued tagged entry', () => {
    const op = makeOperator({ missing: null });
    expect(parseCustomStatistics(op)).toEqual([{ key: 'missing', value: null }]);
  });

  it('unwraps a nested tagged value (double-wrapped)', () => {
    // { Outer: { Inner: 99 } } → unwrap Outer → unwrap Inner → 99
    const op = makeOperator({ nested: makeTagged('Outer', makeTagged('Inner', 99)) });
    expect(parseCustomStatistics(op)).toEqual([{ key: 'nested', value: 99 }]);
  });

  it('unwraps an array of primitives into a string array', () => {
    const op = makeOperator({ tags: makeTagged('List', ['a', 'b', 'c']) });
    const result = parseCustomStatistics(op);
    expect(result).toEqual([{ key: 'tags', value: ['a', 'b', 'c'] }]);
  });

  it('handles an attribute-shaped object {key, value}', () => {
    // The attribute branch: {key: string, value: Value} → "key: value"
    const attrObj = { key: 'color', value: 'red' };
    const op = makeOperator({ attr: makeTagged('Attr', attrObj) });
    expect(parseCustomStatistics(op)).toEqual([{ key: 'attr', value: 'color: red' }]);
  });

  it('falls back to JSON.stringify for multi-key objects that are not attribute-shaped', () => {
    const weirdObj = { a: 1, b: 2, c: 3 };
    const op = makeOperator({ weird: makeTagged('Obj', weirdObj) });
    const result = parseCustomStatistics(op);
    expect(result[0]?.key).toBe('weird');
    expect(result[0]?.value).toBe(JSON.stringify(weirdObj));
  });

  it('returns one entry per statistic key', () => {
    const op = makeOperator({
      rows: makeTagged('UInt64', 10),
      bytes: makeTagged('UInt64', 1024),
    });
    const result = parseCustomStatistics(op);
    expect(result).toHaveLength(2);
    const keys = result.map(r => r.key);
    expect(keys).toContain('rows');
    expect(keys).toContain('bytes');
  });
});

// ---- resolveStatisticFields ------------------------------------------------

function makeQuantifiedOperator(field: string, value: number, quantity?: string) {
  return {
    statistics: {
      custom_statistics: [
        { value: { key: field, value: makeTagged('UInt64', value) }, quantity: quantity ?? null },
      ],
    },
  };
}

const operatorFields = (operator: unknown) => flattenStatistics(parseCustomStatistics(operator));

describe('resolveStatisticFields', () => {
  it('returns the direct statistic without aggregating related values', () => {
    expect(
      resolveStatisticFields(
        operatorFields(makeQuantifiedOperator('bytes', 10, 'bytes')),
        [operatorFields(makeQuantifiedOperator('bytes', 20, 'bytes'))],
        'bytes'
      )
    ).toMatchObject({ key: 'bytes', value: 10, quantity: 'bytes' });
  });

  it('aggregates related values with a shared quantity', () => {
    expect(
      resolveStatisticFields(
        [],
        [
          operatorFields(makeQuantifiedOperator('bytes', 10, 'bytes')),
          operatorFields(makeQuantifiedOperator('bytes', 20, 'bytes')),
        ],
        'bytes'
      )
    ).toMatchObject({ key: 'bytes', value: 30, quantity: 'bytes' });
  });

  it('omits the quantity when related statistics disagree', () => {
    expect(
      resolveStatisticFields(
        [],
        [
          operatorFields(makeQuantifiedOperator('size', 10, 'bytes')),
          operatorFields(makeQuantifiedOperator('size', 20, 'rows')),
        ],
        'size'
      )
    ).toMatchObject({ key: 'size', value: 30 });
    expect(
      resolveStatisticFields(
        [],
        [
          operatorFields(makeQuantifiedOperator('size', 10, 'bytes')),
          operatorFields(makeQuantifiedOperator('size', 20, 'rows')),
        ],
        'size'
      )?.quantity
    ).toBeUndefined();
  });
});

// ---- parsePortStatistics ---------------------------------------------------

function makePort(custom_statistics: Record<string, unknown> | undefined) {
  return {
    statistics:
      custom_statistics !== undefined
        ? {
            custom_statistics: Object.entries(custom_statistics).map(([key, value]) => ({
              key,
              value,
            })),
          }
        : undefined,
  };
}

describe('parsePortStatistics', () => {
  it('returns [] for null/undefined input', () => {
    expect(parsePortStatistics(null)).toEqual([]);
    expect(parsePortStatistics(undefined)).toEqual([]);
  });

  it('returns [] when statistics is absent', () => {
    expect(parsePortStatistics(makePort(undefined))).toEqual([]);
  });

  it('returns [] when custom_statistics is empty', () => {
    expect(parsePortStatistics(makePort({}))).toEqual([]);
  });

  it('unwraps a number tagged value from port statistics', () => {
    const port = makePort({ output_rows: makeTagged('UInt64', 500) });
    expect(parsePortStatistics(port)).toEqual([{ key: 'output_rows', value: 500 }]);
  });

  it('produces null for a null-valued tagged entry', () => {
    const port = makePort({ missing: null });
    expect(parsePortStatistics(port)).toEqual([{ key: 'missing', value: null }]);
  });

  it('falls back to JSON.stringify for multi-key objects that are not attribute-shaped', () => {
    const weirdObj = { a: 1, b: 2, c: 3 };
    const port = makePort({ weird: makeTagged('Obj', weirdObj) });
    const result = parsePortStatistics(port);
    expect(result[0]?.key).toBe('weird');
    expect(result[0]?.value).toBe(JSON.stringify(weirdObj));
  });

  it('returns one entry per statistic key', () => {
    const port = makePort({
      rows: makeTagged('UInt64', 10),
      bytes: makeTagged('UInt64', 1024),
    });
    const result = parsePortStatistics(port);
    expect(result).toHaveLength(2);
    expect(result.map(r => r.key)).toContain('rows');
    expect(result.map(r => r.key)).toContain('bytes');
  });
});

it('preserves producer order and repeated statistic names', () => {
  const custom_statistics = [
    { key: 'z', value: { U64: 3 } },
    { key: 'a', value: { U64: 1 } },
    { key: 'z', value: { U64: 4 } },
  ];
  expect(
    parseCustomStatistics({
      statistics: {
        custom_statistics: custom_statistics.map(value => ({ value, quantity: null })),
      },
    })
  ).toEqual([
    { key: 'z', value: 3 },
    { key: 'a', value: 1 },
    { key: 'z', value: 4 },
  ]);
  expect(parsePortStatistics({ statistics: { custom_statistics } })).toEqual([
    { key: 'z', value: 3 },
    { key: 'a', value: 1 },
    { key: 'z', value: 4 },
  ]);
});
