// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it } from 'vitest';
import type { EntityRef, Operator, QueryBundle } from '@quent/utils';
import { commonQueryMetrics, diffQueryBundles } from './index';

function operator(
  id: string,
  planId: string,
  type: string,
  duration: number | null,
  statistics: Record<string, { value: unknown; quantity: string | null }>,
  parentOperatorIds: string[] = []
): Operator {
  return {
    id,
    plan_id: planId,
    parent_operator_ids: parentOperatorIds,
    instance_name: id,
    operator_type_name: type,
    custom_attributes: [],
    statistics: {
      custom_statistics: Object.entries(statistics).map(([key, statistic]) => ({
        value: { key, value: statistic.value },
        quantity: statistic.quantity,
      })),
    },
    active_span: duration === null ? null : { start: 1, end: 1 + duration },
  } as unknown as Operator;
}

function bundle(
  queryId: string,
  durationSeconds: number,
  operators: Operator[]
): QueryBundle<EntityRef> {
  return {
    query_id: queryId,
    duration_s: durationSeconds,
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
      operators: Object.fromEntries(operators.map(item => [item.id, item])),
    },
  } as unknown as QueryBundle<EntityRef>;
}

describe('query bundle diff', () => {
  it('sums numeric statistics and active spans by logical and physical operator type', () => {
    const baseline = bundle('baseline', 10, [
      operator('logical-join', 'logical', 'Join', null, {
        output_rows: { value: 999n, quantity: 'rows' },
      }),
      operator('logical-scan', 'logical', 'Scan', 3, {
        output_rows: { value: 100n, quantity: 'rows' },
        selectivity: { value: 0.5, quantity: null },
        label: { value: 'scan', quantity: null },
      }),
      operator(
        'physical-join-1',
        'physical',
        'HashJoin',
        2,
        {
          output_rows: { value: 50n, quantity: 'rows' },
          spill_bytes: { value: 0, quantity: 'bytes' },
        },
        ['logical-join']
      ),
      operator(
        'physical-join-2',
        'physical',
        'HashJoin',
        3,
        {
          output_rows: { value: 70n, quantity: 'rows' },
        },
        ['logical-join']
      ),
    ]);
    const candidate = bundle('candidate', 12, [
      operator('logical-join-new', 'logical', 'Join', null, {
        output_rows: { value: 999n, quantity: 'rows' },
      }),
      operator('logical-scan-new', 'logical', 'Scan', 4, {
        output_rows: { value: 150n, quantity: 'rows' },
        selectivity: { value: 0.4, quantity: null },
      }),
      operator('logical-aggregate', 'logical', 'Aggregate', 1, {
        output_rows: { value: 25n, quantity: 'rows' },
      }),
      operator(
        'physical-join-new',
        'physical',
        'HashJoin',
        6,
        {
          output_rows: { value: 140n, quantity: 'rows' },
          spill_bytes: { value: 10, quantity: 'bytes' },
        },
        ['logical-join-new']
      ),
    ]);

    const result = diffQueryBundles({ engineId: 'engine-old', bundle: baseline }, [
      { engineId: 'engine-new', bundle: candidate },
    ]);

    expect(result).toMatchObject({
      baseline: {
        engineId: 'engine-old',
        queryId: 'baseline',
        durationSeconds: 10,
      },
      comparisons: [
        {
          candidate: {
            engineId: 'engine-new',
            queryId: 'candidate',
            durationSeconds: 12,
          },
        },
      ],
    });
    const rows = result.comparisons[0]!.rows;
    expect(
      rows.find(
        row =>
          row.scope === 'logical' && row.operatorType === 'Scan' && row.metric === 'output_rows'
      )
    ).toEqual({
      scope: 'logical',
      operatorType: 'Scan',
      metric: 'output_rows',
      quantity: 'rows',
      baseline: '100',
      candidate: '150',
      delta: '50',
      deltaPercent: 50,
    });
    expect(
      rows.find(
        row =>
          row.scope === 'physical' &&
          row.operatorType === 'HashJoin' &&
          row.metric === 'active_span_s'
      )
    ).toMatchObject({
      baseline: 5,
      candidate: 6,
      delta: 1,
      deltaPercent: 20,
    });
    expect(
      rows.find(
        row =>
          row.scope === 'physical' &&
          row.operatorType === 'HashJoin' &&
          row.metric === 'output_rows'
      )
    ).toMatchObject({
      baseline: '120',
      candidate: '140',
      delta: '20',
    });
    expect(
      rows.find(
        row =>
          row.scope === 'logical' && row.operatorType === 'Join' && row.metric === 'output_rows'
      )
    ).toMatchObject({
      baseline: '120',
      candidate: '140',
      delta: '20',
    });
    expect(
      rows.find(
        row =>
          row.scope === 'physical' &&
          row.operatorType === 'HashJoin' &&
          row.metric === 'spill_bytes'
      )
    ).toMatchObject({
      baseline: 0,
      candidate: 10,
      delta: 10,
      deltaPercent: null,
    });
    expect(rows.some(row => row.metric === 'label')).toBe(false);

    const summary = result.comparisons[0]!.summary;
    expect(
      summary.find(row => row.scope === 'logical' && row.metric === 'output_rows')
    ).toMatchObject({
      baseline: '220',
      candidate: '315',
      delta: '95',
    });
    expect(
      summary.find(row => row.scope === 'physical' && row.metric === 'output_rows')
    ).toMatchObject({
      baseline: '120',
      candidate: '140',
      delta: '20',
    });
  });

  it('shows unavailable values without manufacturing a delta', () => {
    const result = diffQueryBundles({ bundle: bundle('baseline', 10, []) }, [
      {
        bundle: bundle('candidate', 10, [
          operator('aggregate', 'logical', 'Aggregate', 2, {
            output_rows: { value: 8n, quantity: 'rows' },
          }),
        ]),
      },
    ]);
    const outputRows = result.comparisons[0]!.rows.find(row => row.metric === 'output_rows');

    expect(outputRows).toMatchObject({
      baseline: null,
      candidate: '8',
      delta: null,
      deltaPercent: null,
    });
  });

  it('produces stable floating-point sums when API map order differs', () => {
    const first = operator('first', 'physical', 'Filter', 0.1, {
      selectivity: { value: 0.1, quantity: null },
    });
    const second = operator('second', 'physical', 'Filter', 0.2, {
      selectivity: { value: 0.2, quantity: null },
    });
    const third = operator('third', 'physical', 'Filter', 0.3, {
      selectivity: { value: 0.3, quantity: null },
    });

    const result = diffQueryBundles({ bundle: bundle('baseline', 1, [first, second, third]) }, [
      { bundle: bundle('candidate', 1, [third, first, second]) },
    ]);

    expect(result.comparisons[0]!.rows.every(row => row.delta === 0 || row.delta === '0')).toBe(
      true
    );
  });

  it('compares any number of candidates against one baseline', () => {
    const result = diffQueryBundles(
      { engineId: 'baseline-engine', bundle: bundle('baseline', 1, []) },
      [
        { engineId: 'engine-1', bundle: bundle('candidate-1', 2, []) },
        { engineId: 'engine-2', bundle: bundle('candidate-2', 3, []) },
      ]
    );

    expect(result.comparisons.map(comparison => comparison.candidate)).toEqual([
      { engineId: 'engine-1', queryId: 'candidate-1', durationSeconds: 2 },
      { engineId: 'engine-2', queryId: 'candidate-2', durationSeconds: 3 },
    ]);
  });

  it('preserves optional source provenance', () => {
    const result = diffQueryBundles(
      {
        source: 'local',
        engineId: 'engine',
        bundle: bundle('baseline', 1, []),
      },
      [
        {
          source: 'db 6647',
          engineId: 'engine',
          bundle: bundle('candidate', 1, []),
        },
      ]
    );

    expect(result.baseline.source).toBe('local');
    expect(result.comparisons[0]!.candidate.source).toBe('db 6647');
  });

  it('intersects available metrics and filters comparisons', () => {
    const baseline = bundle('baseline', 1, [
      operator('baseline-scan', 'logical', 'Scan', null, {
        output_rows: { value: 100n, quantity: 'rows' },
        bytes_read: { value: 1000n, quantity: 'bytes' },
      }),
    ]);
    const candidate = bundle('candidate', 1, [
      operator('candidate-scan', 'logical', 'Scan', null, {
        output_rows: { value: 80n, quantity: 'rows' },
        selectivity: { value: 0.8, quantity: null },
      }),
    ]);

    expect(commonQueryMetrics([baseline, candidate])).toEqual(['output_rows']);
    const result = diffQueryBundles({ bundle: baseline }, [{ bundle: candidate }], {
      metrics: ['output_rows'],
    });
    expect(result.metrics).toEqual(['output_rows']);
    expect(result.comparisons[0]!.rows.every(row => row.metric === 'output_rows')).toBe(true);
    expect(result.comparisons[0]!.summary.every(row => row.metric === 'output_rows')).toBe(true);
  });

  it('rejects operators whose plan is unavailable', () => {
    const invalid = bundle('invalid', 10, [operator('orphan', 'missing-plan', 'Scan', 1, {})]);

    expect(() =>
      diffQueryBundles({ bundle: invalid }, [{ bundle: bundle('candidate', 10, []) }])
    ).toThrow('does not reference an available plan');
  });
});
