// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { analyzeQueryLifecycle, type QueryLifecycleAnalysis } from '@quent/query-analysis';
import type {
  QuestionContext,
  QuestionDefinition,
  QuestionMetadata,
  QuestionResult,
} from './question.types';
import {
  buildQueryBundleDeepLink,
  formatSeconds,
  queryBundleResolution,
  validateQueryBundleContext,
} from './queryBundleQuestion.utils';

export interface QueryLifecycleResult extends QuestionResult {
  scope: {
    engineId: string;
    queryId: string;
    queryName: string | null;
  };
  finding: QueryLifecycleAnalysis;
}

export const queryLifecycleMetadata: QuestionMetadata = {
  id: 'query-lifecycle',
  version: 1,
  title: 'Query lifecycle',
  explanation:
    'Reports declared query-relative lifecycle offsets and deterministic intervals from one query bundle.',
  requirements: {
    apis: ['query-bundle'],
    resourceTypes: [],
    capacities: [],
    fsmStates: [],
    topologyFields: [],
  },
  parameters: [],
  limitations: [
    'Lifecycle offsets are query-relative declarations; they do not measure client or frontend wall time.',
    'The bundle duration is reported independently from the optional completed offset.',
    'A missing or invalid offset remains unavailable and is not inferred from neighboring phases.',
  ],
};

async function run(context: QuestionContext): Promise<QueryLifecycleResult> {
  validateQueryBundleContext(context);
  const query = context.queryBundle.entities.query;
  const finding = analyzeQueryLifecycle(context.queryBundle);
  const dynamicLimitations = [
    ...(finding.missingFields.length > 0
      ? [`Missing lifecycle fields: ${finding.missingFields.join(', ')}.`]
      : []),
    ...(finding.invalidFields.length > 0
      ? [`Invalid lifecycle fields: ${finding.invalidFields.join(', ')}.`]
      : []),
    ...(!finding.ordered ? ['Declared lifecycle offsets are not monotonically ordered.'] : []),
    ...(query.start_unix_ns !== null &&
    query.start_unix_ns !== context.queryBundle.start_time_unix_ns
      ? ['The query entity and query bundle declare different absolute start timestamps.']
      : []),
  ];

  return {
    question: {
      id: queryLifecycleMetadata.id,
      version: queryLifecycleMetadata.version,
      title: queryLifecycleMetadata.title,
    },
    evidence: {
      class: 'derived',
      explanation:
        'Lifecycle timestamps are observed query-bundle fields; phase intervals are deterministic differences between valid ordered offsets.',
    },
    resolution: queryBundleResolution(),
    scope: {
      engineId: context.engineId,
      queryId: context.queryId,
      queryName: query.instance_name,
    },
    finding,
    deepLink: buildQueryBundleDeepLink(context, { tab: 'timeline' }),
    limitations: [...queryLifecycleMetadata.limitations, ...dynamicLimitations],
  };
}

function formatOffset(label: string, value: number | null): string {
  return `${label}: ${value === null ? 'unavailable' : `+${formatSeconds(value)}`}`;
}

function formatHuman(result: QueryLifecycleResult): string {
  const { finding } = result;
  return [
    `${result.question.title} (v${result.question.version})`,
    `Query: ${result.scope.queryName ?? '(unnamed)'} (${result.scope.queryId})`,
    `Start: ${finding.bundleStartTimeUnixNs.toString()}ns since Unix epoch`,
    `Duration: ${formatSeconds(finding.durationSeconds)}`,
    formatOffset('Planning', finding.offsetsSeconds.planning),
    formatOffset('Executing', finding.offsetsSeconds.executing),
    formatOffset('Completed', finding.offsetsSeconds.completed),
    `Planning interval: ${
      finding.intervalsSeconds.planning === null
        ? 'unavailable'
        : formatSeconds(finding.intervalsSeconds.planning)
    }`,
    `Execution interval: ${
      finding.intervalsSeconds.execution === null
        ? 'unavailable'
        : formatSeconds(finding.intervalsSeconds.execution)
    }`,
    `Complete: ${finding.complete ? 'yes' : 'no'}`,
    `Evidence: ${result.evidence.class} — ${result.evidence.explanation}`,
    `Open evidence: ${result.deepLink}`,
    '',
    'Limitations:',
    ...result.limitations.map(limitation => `- ${limitation}`),
  ].join('\n');
}

export const queryLifecycleQuestion: QuestionDefinition<
  Record<string, never>,
  QueryLifecycleResult
> = {
  metadata: queryLifecycleMetadata,
  parseInput: () => ({}),
  run,
  formatHuman,
};
