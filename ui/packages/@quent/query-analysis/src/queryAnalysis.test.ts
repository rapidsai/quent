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

function bundleWithAmbiguousEdgeTypes(reverseEdges: boolean): QueryBundle<EntityRef> {
  const queryBundle = bundle();
  const operatorTemplate = queryBundle.entities.operators['physical-join']!;
  const portTemplate = queryBundle.entities.ports.input!;
  const pairs = [
    ['source-ab', 'ab', 'target-c', 'c'],
    ['source-a', 'a', 'target-bc', 'bc'],
  ] as const;

  for (const [sourceId, sourceType, targetId, targetType] of pairs) {
    queryBundle.entities.operators[sourceId] = {
      ...operatorTemplate,
      id: sourceId,
      instance_name: sourceId,
      operator_type_name: sourceType,
      parent_operator_ids: [],
      statistics: null,
    };
    queryBundle.entities.operators[targetId] = {
      ...operatorTemplate,
      id: targetId,
      instance_name: targetId,
      operator_type_name: targetType,
      parent_operator_ids: [],
      statistics: null,
    };
    queryBundle.entities.ports[`${sourceId}-port`] = {
      ...portTemplate,
      id: `${sourceId}-port`,
      operator_id: sourceId,
    };
    queryBundle.entities.ports[`${targetId}-port`] = {
      ...portTemplate,
      id: `${targetId}-port`,
      operator_id: targetId,
    };
  }

  const edges = pairs.map(([sourceId, , targetId]) => ({
    source: `${sourceId}-port`,
    target: `${targetId}-port`,
  }));
  queryBundle.entities.plans.physical!.edges = reverseEdges ? edges.reverse() : edges;
  return queryBundle;
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

  it('fingerprints ambiguous edge types independently of edge order', async () => {
    const first = await analyzeQueryStructure(bundleWithAmbiguousEdgeTypes(false));
    const reordered = await analyzeQueryStructure(bundleWithAmbiguousEdgeTypes(true));

    expect(first.fingerprint).toEqual(reordered.fingerprint);
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

  it('keeps direct statistics for logical parents without physical children', () => {
    const queryBundle = bundle();
    const logicalParent = queryBundle.entities.operators['logical-join']!;
    logicalParent.statistics = {
      custom_statistics: [{ value: { key: 'rows', value: 10n }, quantity: 'rows' }],
    };
    queryBundle.entities.operators['logical-filter'] = {
      ...logicalParent,
      id: 'logical-filter',
      instance_name: 'Logical filter',
      operator_type_name: 'Filter',
      parent_operator_ids: ['logical-join'],
      statistics: {
        custom_statistics: [{ value: { key: 'rows', value: 5n }, quantity: 'rows' }],
      },
    };
    delete queryBundle.entities.operators['physical-join'];

    const analysis = summarizeOperatorStatistics(queryBundle);

    expect(
      analysis.aggregates
        .filter(aggregate => aggregate.scope === 'logical')
        .map(aggregate => [aggregate.operatorType, aggregate.sum])
    ).toEqual([
      ['Filter', '5'],
      ['Join', '10'],
    ]);
  });

  it('reports missing plans without discarding valid operator statistics', () => {
    const queryBundle = bundle();
    queryBundle.entities.operators['logical-join']!.plan_id = 'missing-plan';

    const analysis = summarizeOperatorStatistics(queryBundle);

    expect(analysis.groupingIssues).toEqual([
      'Operator "logical-join" references missing plan "missing-plan".',
    ]);
    expect(analysis.aggregates.map(aggregate => aggregate.sum)).toEqual(['10', '20']);
  });

  it('reports empty statistic arrays as skipped lists', () => {
    const queryBundle = bundle();
    queryBundle.entities.operators['physical-join']!.statistics = {
      custom_statistics: [{ value: { key: 'samples', value: [] }, quantity: null }],
    };

    const analysis = summarizeOperatorStatistics(queryBundle);

    expect(analysis.aggregates).toEqual([]);
    expect(analysis.skippedValues).toHaveLength(2);
    expect(analysis.skippedValues.every(value => value.reason === 'list')).toBe(true);
  });
});
