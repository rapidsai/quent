// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { EntityRef, QueryBundle } from '@quent/utils';
import { describe, expect, it } from 'vitest';
import {
  analyzeOperatorFacts,
  analyzeQueryLifecycle,
  analyzeQueryStructure,
  summarizeOperatorStatistics,
} from '.';

function bundle(): QueryBundle<EntityRef> {
  return {
    query_id: 'query-1',
    start_time_unix_ns: 100n,
    duration_s: 10,
    unique_operator_names: ['Join', 'HashJoin'],
    quantity_specs: {
      rows: {
        symbol: 'rows',
        singular: 'row',
        plural: 'rows',
        occupancy_prefix: 'Si',
        rate_prefix: 'Si',
      },
    },
    entities: {
      query: {
        id: 'query-1',
        query_group_id: 'group-1',
        instance_name: 'Query 1',
        start_unix_ns: 100n,
        planning_s: 1,
        executing_s: 3,
        completed_s: 9,
      },
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
          edges: [{ source: 'input', target: 'output' }],
        },
      },
      workers: {
        'worker-1': {
          id: 'worker-1',
          parent_engine_id: 'engine-1',
          instance_name: 'Worker 1',
          start_unix_ns: 100n,
          end_unix_ns: 10_000_000_100n,
        },
      },
      operators: {
        'logical-join': {
          id: 'logical-join',
          plan_id: 'logical',
          parent_operator_ids: [],
          instance_name: 'Logical join',
          operator_type_name: 'Join',
          custom_attributes: [],
          statistics: null,
          active_span: null,
        },
        'physical-join': {
          id: 'physical-join',
          plan_id: 'physical',
          parent_operator_ids: ['logical-join'],
          instance_name: 'Physical join',
          operator_type_name: 'HashJoin',
          custom_attributes: [],
          statistics: {
            custom_statistics: [
              {
                value: { key: 'rows', value: 10n },
                quantity: 'rows',
              },
              {
                value: { key: 'rows', value: 20n },
                quantity: 'rows',
              },
            ],
          },
          active_span: { start: 3, end: 8 },
        },
      },
      ports: {
        input: {
          id: 'input',
          operator_id: 'physical-join',
          instance_name: 'input',
          statistics: null,
        },
        output: {
          id: 'output',
          operator_id: 'physical-join',
          instance_name: 'output',
          statistics: null,
        },
      },
    },
    plan_tree: {
      id: 'logical',
      worker: null,
      children: [{ id: 'physical', worker: 'worker-1', children: [] }],
    },
    resource_tree: { Resource: { Resource: 'unused' } },
  } as unknown as QueryBundle<EntityRef>;
}

describe('@quent/query-analysis', () => {
  it('calculates lifecycle intervals from a query bundle', () => {
    expect(analyzeQueryLifecycle(bundle())).toMatchObject({
      offsetsSeconds: { planning: 1, executing: 3, completed: 9 },
      intervalsSeconds: {
        beforePlanning: 1,
        planning: 2,
        execution: 6,
        afterCompletion: 1,
      },
      complete: true,
    });
  });

  it('projects operator facts without CLI context', () => {
    const analysis = analyzeOperatorFacts(bundle(), ['physical-join']);

    expect(analysis).toMatchObject({
      operatorCount: 1,
      operators: [
        {
          id: 'physical-join',
          plan: { id: 'physical', scope: 'physical', worker: { id: 'worker-1' } },
          activeSpan: { durationSeconds: 5 },
          parentOperatorIds: ['logical-join'],
        },
      ],
      integrityIssues: [],
    });
  });

  it('creates a browser-safe structural fingerprint', async () => {
    const analysis = await analyzeQueryStructure(bundle());

    expect(analysis.counts).toMatchObject({
      workers: 1,
      plans: 2,
      operators: 2,
      ports: 2,
      edges: 1,
    });
    expect(analysis.fingerprint.value).toMatch(/^[a-f0-9]{64}$/u);
    expect(analysis.integrityIssues).toEqual([]);
  });

  it('keeps repeated root statistics in separate path occurrences', () => {
    const analysis = summarizeOperatorStatistics(bundle());
    const paths = analysis.aggregates
      .filter(aggregate => aggregate.scope === 'physical')
      .map(aggregate => aggregate.statisticPath);

    expect(paths).toEqual([[['rows', 0]], [['rows', 1]]]);
    expect(
      analysis.aggregates
        .filter(aggregate => aggregate.scope === 'physical')
        .map(aggregate => aggregate.sum)
    ).toEqual(['10', '20']);
  });
});
