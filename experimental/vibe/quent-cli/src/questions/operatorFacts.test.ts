// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { gunzipSync } from 'node:zlib';
import type { EntityRef, QueryBundle } from '@quent/utils';
import { describe, expect, it, vi } from 'vitest';
import type { QueryEvidenceDeepLinkState } from '../deepLink';
import type { OperatorFactsResult } from './operatorFacts';
import { getQuestion } from './questionRegistry';

function bundle(): QueryBundle<EntityRef> {
  return {
    query_id: 'query-1',
    start_time_unix_ns: 0n,
    duration_s: 10,
    unique_operator_names: ['LogicalScan', 'PhysicalScan'],
    quantity_specs: {},
    entities: {
      plans: {
        logical: {
          id: 'logical',
          instance_name: 'Logical plan',
          parent: null,
          worker_id: null,
          edges: [],
        },
        physical: {
          id: 'physical',
          instance_name: 'Physical plan',
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
      operators: {
        'logical-scan': {
          id: 'logical-scan',
          plan_id: 'logical',
          parent_operator_ids: [],
          instance_name: 'Logical scan',
          operator_type_name: 'LogicalScan',
          custom_attributes: [],
          statistics: null,
          active_span: null,
        },
        'physical-scan': {
          id: 'physical-scan',
          plan_id: 'physical',
          parent_operator_ids: ['logical-scan'],
          instance_name: 'Physical scan',
          operator_type_name: 'PhysicalScan',
          custom_attributes: [
            { key: 'label', value: { String: 'orders' } },
            { key: 'parallelism', value: { U64: 4 } },
          ],
          statistics: {
            custom_statistics: [
              {
                value: { key: 'label', value: { String: 'scan' } },
                quantity: null,
              },
              {
                value: { key: 'rows', value: { U64: 100 } },
                quantity: 'rows',
              },
            ],
          },
          active_span: { start: 1.5, end: 8 },
        },
      },
      ports: {
        'port-1': {
          id: 'port-1',
          operator_id: 'physical-scan',
          instance_name: 'output',
          statistics: {
            custom_statistics: [{ key: 'bytes', value: { U64: 1024 } }],
          },
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

function context(queryBundle = bundle()) {
  return {
    engineId: 'engine-1',
    queryId: 'query-1',
    appBaseUrl: 'https://quent.example.test',
    queryBundle,
    api: { fetchEntityList: vi.fn() },
  };
}

function decodeDeepLink(url: string): QueryEvidenceDeepLinkState {
  const encoded = new URL(url).searchParams.get('s');
  const payload = encoded?.split('.', 2)[1];
  if (!payload) {
    throw new Error('Missing deep-link payload.');
  }
  return JSON.parse(
    gunzipSync(Buffer.from(payload, 'base64url')).toString('utf8')
  ) as QueryEvidenceDeepLinkState;
}

describe('operator-facts', () => {
  it('returns stable per-instance facts without coercing dynamic values', async () => {
    const question = getQuestion('operator-facts');
    const result = (await question.run(context(), {})) as OperatorFactsResult;

    expect(result.finding.operatorCount).toBe(2);
    expect(result.finding.operators[0]).toMatchObject({
      id: 'logical-scan',
      plan: { id: 'logical', scope: 'logical', worker: null },
      childOperatorIds: ['physical-scan'],
    });
    expect(result.finding.operators[1]).toEqual({
      id: 'physical-scan',
      instanceName: 'Physical scan',
      typeName: 'PhysicalScan',
      plan: {
        id: 'physical',
        instanceName: 'Physical plan',
        parentPlanId: 'logical',
        scope: 'physical',
        worker: { id: 'worker-1', instanceName: 'Worker 1' },
      },
      parentOperatorIds: ['logical-scan'],
      childOperatorIds: [],
      activeSpan: { startSeconds: 1.5, endSeconds: 8, durationSeconds: 6.5 },
      customAttributes: [
        { path: ['label'], value: { String: 'orders' } },
        { path: ['parallelism'], value: { U64: 4 } },
      ],
      statistics: [
        { path: ['label'], value: { String: 'scan' }, quantity: null },
        { path: ['rows'], value: { U64: 100 }, quantity: 'rows' },
      ],
      ports: [
        {
          id: 'port-1',
          instanceName: 'output',
          statistics: [{ path: ['bytes'], value: { U64: 1024 } }],
        },
      ],
    });
    expect(result.finding.integrityIssues).toEqual([]);
  });

  it('filters and links explicitly requested operators', async () => {
    const question = getQuestion('operator-facts');
    const result = (await question.run(context(), {
      operator: 'physical-scan,physical-scan',
    })) as OperatorFactsResult;

    expect(result.scope.requestedOperatorIds).toEqual(['physical-scan']);
    expect(result.finding.operators.map(operator => operator.id)).toEqual(['physical-scan']);
    expect(decodeDeepLink(result.deepLink)).toMatchObject({
      route: { tab: 'operators' },
      selection: { operatorNodeIds: ['physical-scan'] },
    });
  });

  it('rejects unknown operators and reports invalid spans', async () => {
    const question = getQuestion('operator-facts');
    await expect(question.run(context(), { operator: 'missing-operator' })).rejects.toThrow(
      'not present in the query bundle'
    );

    const invalid = bundle();
    invalid.entities.operators['physical-scan']!.active_span = { start: 5, end: 2 };
    const result = (await question.run(context(invalid), {
      operator: 'physical-scan',
    })) as OperatorFactsResult;
    expect(result.finding.operators[0]?.activeSpan?.durationSeconds).toBeNull();
    expect(result.finding.integrityIssues).toEqual([
      'Operator "physical-scan" has an invalid active span.',
    ]);
  });
});
