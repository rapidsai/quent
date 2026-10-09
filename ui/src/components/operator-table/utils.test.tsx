// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { render, screen } from '@testing-library/react';
import { Provider, createStore } from 'jotai';
import { describe, expect, it, vi } from 'vitest';
import { PivotedStatTable, formatStatValue, type PivotedStatTableSchema } from '@quent/components';
import type { Operator, QueryEntities, QuantitySpec } from '@quent/utils';
import { buildOperatorRows } from './utils';
import type { OperatorTableRow } from './types';

function makeEntities(statistics: NonNullable<Operator['statistics']>): QueryEntities {
  return {
    engine: {
      id: 'engine',
      start_time_unix_ns: null,
      duration_s: null,
      instance_name: null,
      implementation: null,
      custom_attributes: [],
    },
    query: {
      id: 'query',
      instance_name: null,
      custom_attributes: [],
      start_unix_ns: null,
      planning_s: null,
      executing_s: null,
      completed_s: null,
    },
    plans: {
      plan: { id: 'plan', instance_name: null, parent: null, worker_id: null, edges: [] },
    },
    operators: {
      scan: {
        id: 'scan',
        plan_id: 'plan',
        parent_operator_ids: [],
        instance_name: 'Scan',
        operator_type_name: 'Scan',
        custom_attributes: [],
        active_span: { start: 1, end: 3 },
        statistics,
      },
    },
    workers: {},
    ports: {},
    resources: {},
    resource_groups: {},
    resource_types: {},
    resource_group_types: {},
    fsm_types: {},
  };
}

const schema: PivotedStatTableSchema<OperatorTableRow> = {
  groups: { item: { id: row => row.itemId, label: row => row.itemName } },
  itemId: row => row.itemId,
  scopeId: row => row.scopeId,
  itemType: row => row.itemType,
  stats: row => row.stats,
};

const quantitySpecs: Record<string, QuantitySpec> = {
  bytes: {
    symbol: 'B',
    singular: 'byte',
    plural: 'bytes',
    occupancy_prefix: 'Iec',
    rate_prefix: 'Si',
  },
  seconds: {
    symbol: 's',
    singular: 'second',
    plural: 'seconds',
    occupancy_prefix: 'None',
    rate_prefix: 'None',
  },
};

function renderRows(rows: OperatorTableRow[], isAggregating = false, activeIndices = ['item']) {
  render(
    <Provider store={createStore()}>
      <PivotedStatTable
        rows={rows}
        schema={schema}
        activeIndices={activeIndices}
        isAggregating={isAggregating}
        renderConfig={{
          formatValue: (value, statName, quantity) =>
            formatStatValue(value, statName, quantitySpecs, quantity),
        }}
        interaction={{
          hoveredStat: null,
          setHoveredStat: vi.fn(),
          hoveredItemId: null,
          selectedItemIds: new Set(),
        }}
      />
    </Provider>
  );
}

const structuredStatistics = {
  custom_statistics: [
    {
      value: {
        key: 'device',
        value: [
          { key: 'name', value: 'GPU' },
          { key: 'count', value: 0 },
          { key: 'count', value: 3 },
          { key: 'details', value: [{ key: 'bytes', value: 2048n }] },
        ],
      },
      quantity: null,
    },
    {
      value: {
        key: 'devices',
        value: [[{ key: 'name', value: 'GPU' }], [{ key: 'name', value: ['CPU', 'Disk'] }]],
      },
      quantity: null,
    },
    { value: { key: 'rows', value: 0 }, quantity: null },
    { value: { key: 'rows', value: 7 }, quantity: null },
    { value: { key: 'input_bytes', value: 1024 }, quantity: 'bytes' },
  ],
} satisfies NonNullable<Operator['statistics']>;

describe('operator table structured statistics', () => {
  it('renders structs, nested lists, and repeated names through the real pivot table', () => {
    const rows = buildOperatorRows(makeEntities(structuredStatistics), new Set(['plan']));
    renderRows(rows);

    expect(
      screen.getByRole('cell', { name: 'name: GPU, count: 0, count: 3, details: bytes: 2.00 KiB' })
    ).toBeInTheDocument();
    expect(screen.getByRole('cell', { name: 'name: GPU, name: CPU, Disk' })).toBeInTheDocument();
    expect(screen.getByRole('cell', { name: '0, 7.00' })).toBeInTheDocument();
    expect(screen.queryByText(/\[object Object\]/)).not.toBeInTheDocument();
    expect(rows[0].stats).toContainEqual({ key: 'input_bytes', value: 1024, quantity: 'bytes' });
  });

  it('keeps scalar numeric aggregation and does not sum repeated values', () => {
    const rows = buildOperatorRows(makeEntities(structuredStatistics), new Set(['plan']));
    renderRows(rows, true);

    expect(screen.getByRole('cell', { name: '1.00 KiB' })).toBeInTheDocument();
    expect(screen.queryByRole('cell', { name: '7' })).not.toBeInTheDocument();
    expect(screen.getAllByRole('cell', { name: '-' })).toHaveLength(3);
  });

  it('retains each repeated entry with its own quantity and formats numeric list elements', () => {
    const rows = buildOperatorRows(
      makeEntities({
        custom_statistics: [
          { value: { key: 'work', value: 1024n }, quantity: 'bytes' },
          { value: { key: 'work', value: 2 }, quantity: 'seconds' },
          { value: { key: 'sizes', value: [1024n, 2048n] }, quantity: 'bytes' },
        ],
      }),
      new Set(['plan'])
    );
    renderRows(rows);

    expect(rows[0].stats.slice(1)).toEqual([
      { key: 'work', value: 1024n, quantity: 'bytes' },
      { key: 'work', value: 2, quantity: 'seconds' },
      { key: 'sizes', value: [1024n, 2048n], quantity: 'bytes' },
    ]);
    expect(screen.getByRole('cell', { name: '1.00 KiB, 2.00 s' })).toBeInTheDocument();
    expect(screen.getByRole('cell', { name: '1.00 KiB, 2.00 KiB' })).toBeInTheDocument();
  });

  it.each([
    ['bytes', 'bytes', '2.00 KiB'],
    ['bytes', 'seconds', '-'],
  ])('aggregates quantities %s and %s as %s', (first, second, expected) => {
    const entities = makeEntities({
      custom_statistics: [{ value: { key: 'work', value: 1024 }, quantity: first }],
    });
    entities.operators.other = {
      ...entities.operators.scan,
      id: 'other',
      statistics: {
        custom_statistics: [{ value: { key: 'work', value: 1024 }, quantity: second }],
      },
    };
    renderRows(buildOperatorRows(entities, new Set(['plan'])), true, []);
    expect(screen.getByRole('cell', { name: expected })).toBeInTheDocument();
  });
});
