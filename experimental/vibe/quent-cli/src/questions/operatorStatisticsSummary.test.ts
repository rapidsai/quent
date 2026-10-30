// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { EntityRef, Operator, QueryBundle } from '@quent/utils';
import { describe, expect, it, vi } from 'vitest';
import type { OperatorStatisticsSummaryResult } from './operatorStatisticsSummary';
import { getQuestion } from './questionRegistry';

function operator(
  id: string,
  statistics: Record<string, { value: unknown; quantity: string | null }>
): Operator {
  return {
    id,
    plan_id: 'physical',
    parent_operator_ids: ['logical-join'],
    instance_name: id,
    operator_type_name: 'HashJoin',
    custom_attributes: [],
    statistics: {
      custom_statistics: Object.entries(statistics).map(([key, statistic]) => ({
        value: { key, value: statistic.value },
        quantity: statistic.quantity,
      })),
    },
    active_span: null,
  } as unknown as Operator;
}

function bundle(operators: Operator[]): QueryBundle<EntityRef> {
  const logical = {
    id: 'logical-join',
    plan_id: 'logical',
    parent_operator_ids: [],
    instance_name: 'Logical join',
    operator_type_name: 'Join',
    custom_attributes: [],
    statistics: {
      custom_statistics: [
        {
          value: { key: 'rows', value: 999n },
          quantity: 'rows',
        },
      ],
    },
    active_span: null,
  } as unknown as Operator;
  return {
    query_id: 'query-1',
    start_time_unix_ns: 0n,
    duration_s: 10,
    unique_operator_names: ['HashJoin', 'Join'],
    quantity_specs: {
      rows: {
        symbol: 'rows',
        singular: 'row',
        plural: 'rows',
        occupancy_prefix: 'Si',
        rate_prefix: 'Si',
      },
      milliseconds: {
        symbol: 'ms',
        singular: 'millisecond',
        plural: 'milliseconds',
        occupancy_prefix: 'Si',
        rate_prefix: 'Si',
      },
    },
    entities: {
      plans: {
        logical: {
          id: 'logical',
          instance_name: 'Logical',
          parent: null,
          worker_id: null,
          edges: [],
        },
        physical: {
          id: 'physical',
          instance_name: 'Physical',
          parent: 'logical',
          worker_id: 'worker-1',
          edges: [],
        },
      },
      workers: {
        'worker-1': {
          id: 'worker-1',
          parent_engine_id: 'engine-1',
          instance_name: 'Worker 1',
          start_unix_ns: 0n,
          end_unix_ns: 10_000_000_000n,
        },
      },
      operators: Object.fromEntries([logical, ...operators].map(item => [item.id, item])),
      ports: {},
    },
    plan_tree: {
      id: 'logical',
      worker: null,
      children: [{ id: 'physical', worker: 'worker-1', children: [] }],
    },
    resource_tree: { Resource: { Resource: 'unused' } },
  } as unknown as QueryBundle<EntityRef>;
}

function context(queryBundle: QueryBundle<EntityRef>) {
  return {
    engineId: 'engine-1',
    queryId: 'query-1',
    queryBundle,
    api: { fetchEntityList: vi.fn() },
  };
}

describe('operator-statistics-summary', () => {
  it('calculates stable distributions, extrema sources, and missing values', async () => {
    const queryBundle = bundle([
      operator('join-1', {
        latency: { value: 5, quantity: 'milliseconds' },
        memory: {
          value: {
            Struct: [
              { key: 'bytes', value: { U64: 100 } },
              { key: 'bytes', value: { U64: 200 } },
            ],
          },
          quantity: 'bytes',
        },
        rows: { value: 10n, quantity: 'rows' },
        label: { value: { String: 'build' }, quantity: null },
        samples: { value: { List: { U64: [1, 2] } }, quantity: null },
      }),
      operator('join-2', {
        latency: { value: 5, quantity: 'milliseconds' },
        rows: { value: 20n, quantity: 'rows' },
      }),
      operator('join-3', {
        rows: { value: 2n, quantity: 'bytes' },
        mystery: { value: 7, quantity: 'mystery-unit' },
      }),
    ]);
    const question = getQuestion('operator-statistics-summary');
    const result = (await question.run(
      context(queryBundle),
      {}
    )) as OperatorStatisticsSummaryResult;

    const physicalRows = result.finding.aggregates.find(
      aggregate =>
        aggregate.scope === 'physical' &&
        aggregate.operatorType === 'HashJoin' &&
        JSON.stringify(aggregate.statisticPath) === JSON.stringify([['rows', 0]]) &&
        aggregate.quantity === 'rows'
    );
    expect(physicalRows).toMatchObject({
      operatorCount: 3,
      numericValueCount: 2,
      missingValueCount: 1,
      nonNumericValueCount: 0,
      sum: '30',
      minimum: '10',
      maximum: '20',
      mean: 15,
      median: 15,
      populationVariance: 25,
      populationStandardDeviation: 5,
      percentile25: 12.5,
      percentile75: 17.5,
      minimumSources: [{ operatorId: 'join-1', planId: 'physical', workerId: 'worker-1' }],
      maximumSources: [{ operatorId: 'join-2', planId: 'physical', workerId: 'worker-1' }],
      approximateFloatingPoint: false,
    });
    const logicalRows = result.finding.aggregates.find(
      aggregate =>
        aggregate.scope === 'logical' &&
        aggregate.operatorType === 'Join' &&
        JSON.stringify(aggregate.statisticPath) === JSON.stringify([['rows', 0]]) &&
        aggregate.quantity === 'rows'
    );
    expect(logicalRows?.sum).toBe('30');
    expect(result.finding.aggregates.some(aggregate => aggregate.sum === '999')).toBe(false);

    const latency = result.finding.aggregates.find(
      aggregate =>
        aggregate.scope === 'physical' &&
        JSON.stringify(aggregate.statisticPath) === JSON.stringify([['latency', 0]])
    );
    expect(latency?.minimumSources.map(item => item.operatorId)).toEqual(['join-1', 'join-2']);
    expect(latency?.maximumSources.map(item => item.operatorId)).toEqual(['join-1', 'join-2']);
    expect(latency).toMatchObject({
      populationVariance: 0,
      populationStandardDeviation: 0,
      percentile25: 5,
      percentile75: 5,
    });
    expect(question.formatHuman(result)).toContain('│ Plan');
    expect(question.formatHuman(result)).toContain('│ Physical');
  });

  it('preserves nested paths, separates quantities, and reports unsupported values', async () => {
    const queryBundle = bundle([
      operator('join-1', {
        memory: {
          value: {
            Struct: [
              { key: 'bytes', value: { U64: 100 } },
              { key: 'bytes', value: { U64: 200 } },
            ],
          },
          quantity: 'bytes',
        },
        rows: { value: 10, quantity: 'rows' },
        label: { value: 'build', quantity: null },
        samples: { value: { List: { U64: [1, 2] } }, quantity: null },
      }),
      operator('join-2', {
        rows: { value: 2, quantity: 'bytes' },
        mystery: { value: 7, quantity: 'mystery-unit' },
      }),
    ]);
    const question = getQuestion('operator-statistics-summary');
    const result = (await question.run(
      context(queryBundle),
      {}
    )) as OperatorStatisticsSummaryResult;

    const nestedPaths = result.finding.aggregates
      .filter(
        aggregate => aggregate.scope === 'physical' && aggregate.statisticPath[0]?.[0] === 'memory'
      )
      .map(aggregate => aggregate.statisticPath);
    expect(nestedPaths).toEqual([
      [
        ['memory', 0],
        ['bytes', 0],
      ],
      [
        ['memory', 0],
        ['bytes', 1],
      ],
    ]);
    expect(
      result.finding.aggregates
        .filter(
          aggregate => aggregate.scope === 'physical' && aggregate.statisticPath[0]?.[0] === 'rows'
        )
        .map(aggregate => aggregate.quantity)
    ).toEqual(['bytes', 'rows']);
    expect(
      result.finding.skippedValues
        .filter(item => item.scope === 'physical')
        .map(item => item.reason)
        .sort()
    ).toEqual(['list', 'non-numeric']);
    expect(result.finding.undeclaredQuantities).toEqual(['bytes', 'mystery-unit']);
    expect(result.limitations).toContain('Undeclared quantities: bytes, mystery-unit.');
  });

  it('marks floating-point measures from unsafe integers as approximate', async () => {
    const queryBundle = bundle([
      operator('join-1', {
        rows: { value: 9_007_199_254_740_993n, quantity: 'rows' },
      }),
    ]);
    const question = getQuestion('operator-statistics-summary');
    const result = (await question.run(
      context(queryBundle),
      {}
    )) as OperatorStatisticsSummaryResult;
    const aggregate = result.finding.aggregates.find(
      item => item.scope === 'physical' && item.quantity === 'rows'
    );

    expect(aggregate).toMatchObject({
      sum: '9007199254740993',
      minimum: '9007199254740993',
      maximum: '9007199254740993',
      approximateFloatingPoint: true,
      populationVariance: 0,
    });
    expect(result.limitations[result.limitations.length - 1]).toContain(
      'floating-point measures are approximate'
    );
  });

  it('returns an empty aggregate set when no numeric statistics exist', async () => {
    const queryBundle = bundle([
      operator('join-1', {
        label: { value: 'build', quantity: null },
      }),
    ]);
    const question = getQuestion('operator-statistics-summary');
    const result = (await question.run(
      context(queryBundle),
      {}
    )) as OperatorStatisticsSummaryResult;

    expect(result.finding.aggregates).toEqual([]);
    expect(result.finding.skippedValues).toHaveLength(2);
  });
});
