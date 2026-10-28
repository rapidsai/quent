// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { gunzipSync } from 'node:zlib';
import type { EntityRef, QueryBundle } from '@quent/utils';
import { describe, expect, it, vi } from 'vitest';
import type { QueryEvidenceDeepLinkState } from '../deepLink';
import type { QueryLifecycleResult } from './queryLifecycle';
import { getQuestion } from './questionRegistry';

function bundle(
  query: Partial<QueryBundle<EntityRef>['entities']['query']> = {}
): QueryBundle<EntityRef> {
  return {
    query_id: 'query-1',
    start_time_unix_ns: 1_000_000_000n,
    duration_s: 12,
    unique_operator_names: [],
    quantity_specs: {},
    entities: {
      query: {
        id: 'query-1',
        query_group_id: 'group-1',
        instance_name: 'Query 1',
        start_unix_ns: 1_000_000_000n,
        planning_s: 0.5,
        executing_s: 1.25,
        completed_s: 11,
        ...query,
      },
    },
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

describe('query-lifecycle', () => {
  it('reports declared offsets and deterministic phase intervals', async () => {
    const question = getQuestion('query-lifecycle');
    const result = (await question.run(context(), {})) as QueryLifecycleResult;

    expect(result).toMatchObject({
      question: { id: 'query-lifecycle', version: 1 },
      evidence: { class: 'derived' },
      resolution: { source: 'query-bundle', binCount: null },
      scope: {
        engineId: 'engine-1',
        queryId: 'query-1',
        queryName: 'Query 1',
      },
      finding: {
        bundleStartTimeUnixNs: 1_000_000_000n,
        durationSeconds: 12,
        offsetsSeconds: {
          planning: 0.5,
          executing: 1.25,
          completed: 11,
        },
        intervalsSeconds: {
          beforePlanning: 0.5,
          planning: 0.75,
          execution: 9.75,
          afterCompletion: 1,
        },
        complete: true,
        missingFields: [],
        invalidFields: [],
        ordered: true,
      },
    });
    expect(decodeDeepLink(result.deepLink)).toEqual({
      route: { engineId: 'engine-1', queryId: 'query-1', tab: 'timeline' },
      timeline: { zoomRange: { start: 0, end: 12 } },
    });
    expect(question.formatHuman(result)).toContain('Execution interval: 9.75s');
  });

  it('keeps missing and unordered lifecycle fields explicit', async () => {
    const question = getQuestion('query-lifecycle');
    const result = (await question.run(
      context(bundle({ planning_s: null, executing_s: 8, completed_s: 7 })),
      {}
    )) as QueryLifecycleResult;

    expect(result.finding).toMatchObject({
      complete: false,
      missingFields: ['planning_s'],
      invalidFields: [],
      ordered: false,
      intervalsSeconds: {
        beforePlanning: null,
        planning: null,
        execution: null,
        afterCompletion: 5,
      },
    });
    expect(result.limitations).toContain('Missing lifecycle fields: planning_s.');
    expect(result.limitations).toContain(
      'Declared lifecycle offsets are not monotonically ordered.'
    );
  });

  it('rejects a mismatched query bundle', async () => {
    const question = getQuestion('query-lifecycle');
    await expect(question.run({ ...context(), queryId: 'query-other' }, {})).rejects.toThrow(
      'does not match requested query'
    );
  });
});
