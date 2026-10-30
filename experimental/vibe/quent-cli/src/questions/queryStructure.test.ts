// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { EntityRef, QueryBundle } from '@quent/utils';
import { describe, expect, it, vi } from 'vitest';
import type { QueryStructureResult } from './queryStructure';
import { getQuestion } from './questionRegistry';

function bundle(suffix = 'a', reverse = false): QueryBundle<EntityRef> {
  const rootPlanId = `logical-${suffix}`;
  const workerPlanId = `physical-${suffix}`;
  const workerId = `worker-${suffix}`;
  const logicalOperatorId = `logical-scan-${suffix}`;
  const physicalOperatorId = `physical-scan-${suffix}`;
  const sourcePortId = `source-${suffix}`;
  const targetPortId = `target-${suffix}`;
  const entries = <T>(values: Array<[string, T]>) =>
    Object.fromEntries(reverse ? [...values].reverse() : values);
  return {
    query_id: 'query-1',
    start_time_unix_ns: 0n,
    duration_s: 10,
    unique_operator_names: ['PhysicalScan', 'Scan'],
    quantity_specs: {},
    entities: {
      query: {
        id: 'query-1',
        query_group_id: 'group-1',
        instance_name: 'Query 1',
        start_unix_ns: 0n,
        planning_s: 0,
        executing_s: 1,
        completed_s: 10,
      },
      workers: {
        [workerId]: {
          id: workerId,
          parent_engine_id: 'engine-1',
          instance_name: 'Worker',
          start_unix_ns: 0n,
          end_unix_ns: 10_000_000_000n,
        },
      },
      plans: entries([
        [
          rootPlanId,
          {
            id: rootPlanId,
            instance_name: 'Logical',
            parent: null,
            worker_id: null,
            edges: [],
          },
        ],
        [
          workerPlanId,
          {
            id: workerPlanId,
            instance_name: 'Physical',
            parent: rootPlanId,
            worker_id: workerId,
            edges: [{ source: sourcePortId, target: targetPortId }],
          },
        ],
      ]),
      operators: entries([
        [
          logicalOperatorId,
          {
            id: logicalOperatorId,
            plan_id: rootPlanId,
            parent_operator_ids: [],
            instance_name: 'Logical scan',
            operator_type_name: 'Scan',
            custom_attributes: {},
            statistics: null,
            active_span: null,
          },
        ],
        [
          physicalOperatorId,
          {
            id: physicalOperatorId,
            plan_id: workerPlanId,
            parent_operator_ids: [logicalOperatorId],
            instance_name: 'Physical scan',
            operator_type_name: 'PhysicalScan',
            custom_attributes: {},
            statistics: null,
            active_span: { start: 1, end: 9 },
          },
        ],
      ]),
      ports: entries([
        [
          sourcePortId,
          {
            id: sourcePortId,
            operator_id: physicalOperatorId,
            instance_name: 'output',
            statistics: null,
          },
        ],
        [
          targetPortId,
          {
            id: targetPortId,
            operator_id: physicalOperatorId,
            instance_name: 'input',
            statistics: null,
          },
        ],
      ]),
      resource_types: {},
      resource_group_types: {},
      resources: {},
      resource_groups: {},
      fsm_types: {},
    },
    plan_tree: {
      id: rootPlanId,
      worker: null,
      children: [{ id: workerPlanId, worker: workerId, children: [] }],
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

describe('query-structure', () => {
  it('returns a stable, ordered topology projection', async () => {
    const question = getQuestion('query-structure');
    const result = (await question.run(context(bundle()), {})) as QueryStructureResult;

    expect(result).toMatchObject({
      question: { id: 'query-structure', version: 1 },
      resolution: { source: 'query-bundle' },
      finding: {
        counts: {
          workers: 1,
          plans: 2,
          operators: 2,
          operatorTypes: 2,
          ports: 2,
          edges: 1,
        },
        declaredOperatorTypes: ['PhysicalScan', 'Scan'],
        observedOperatorTypes: ['PhysicalScan', 'Scan'],
        integrityIssues: [],
      },
    });
    expect(result.finding.plans[1]).toMatchObject({
      id: 'physical-a',
      parentPlanId: 'logical-a',
      workerId: 'worker-a',
      operatorIds: ['physical-scan-a'],
      edges: [
        {
          sourcePortId: 'source-a',
          sourceOperatorId: 'physical-scan-a',
          targetPortId: 'target-a',
          targetOperatorId: 'physical-scan-a',
        },
      ],
    });
    expect(result.finding.operators[1]?.portIds).toEqual(['source-a', 'target-a']);
    expect(result.deepLink).toContain('/operators?s=v3.');
  });

  it('fingerprints structure independently of map order and entity IDs', async () => {
    const question = getQuestion('query-structure');
    const first = (await question.run(context(bundle('a')), {})) as QueryStructureResult;
    const reordered = (await question.run(context(bundle('b', true)), {})) as QueryStructureResult;

    expect(first.finding.fingerprint).toEqual(reordered.finding.fingerprint);
  });

  it('reports broken topology references without hiding the remaining structure', async () => {
    const invalid = bundle();
    invalid.entities.plans['physical-a']!.edges[0]!.target = 'missing-port';
    const question = getQuestion('query-structure');
    const result = (await question.run(context(invalid), {})) as QueryStructureResult;

    expect(result.finding.integrityIssues).toEqual([
      'Plan "physical-a" edge references missing target port "missing-port".',
    ]);
    expect(result.limitations).toContain(
      'The query bundle contains 1 topology integrity issue(s).'
    );
  });
});
