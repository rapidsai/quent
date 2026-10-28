// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

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

type LifecycleOffset = 'planning_s' | 'executing_s' | 'completed_s';

export interface QueryLifecycleResult extends QuestionResult {
  scope: {
    engineId: string;
    queryId: string;
    queryName: string | null;
  };
  finding: {
    bundleStartTimeUnixNs: bigint;
    queryStartTimeUnixNs: bigint | null;
    durationSeconds: number;
    offsetsSeconds: {
      planning: number | null;
      executing: number | null;
      completed: number | null;
    };
    intervalsSeconds: {
      planning: number | null;
      execution: number | null;
      beforePlanning: number | null;
      afterCompletion: number | null;
    };
    complete: boolean;
    missingFields: LifecycleOffset[];
    invalidFields: LifecycleOffset[];
    ordered: boolean;
  };
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

function validOffset(value: number | null): value is number {
  return value !== null && Number.isFinite(value) && value >= 0;
}

function interval(start: number | null, end: number | null): number | null {
  return validOffset(start) && validOffset(end) && end >= start ? end - start : null;
}

async function run(context: QuestionContext): Promise<QueryLifecycleResult> {
  validateQueryBundleContext(context);
  const query = context.queryBundle.entities.query;
  const offsets = {
    planning_s: query.planning_s,
    executing_s: query.executing_s,
    completed_s: query.completed_s,
  };
  const missingFields = (Object.entries(offsets) as Array<[LifecycleOffset, number | null]>)
    .filter(([, value]) => value === null)
    .map(([name]) => name);
  const invalidFields = (Object.entries(offsets) as Array<[LifecycleOffset, number | null]>)
    .filter(([, value]) => value !== null && !validOffset(value))
    .map(([name]) => name);
  const planning = offsets.planning_s;
  const executing = offsets.executing_s;
  const completed = offsets.completed_s;
  const ordered =
    (!validOffset(planning) || !validOffset(executing) || planning <= executing) &&
    (!validOffset(executing) || !validOffset(completed) || executing <= completed) &&
    (!validOffset(completed) || completed <= context.queryBundle.duration_s);
  const dynamicLimitations = [
    ...(missingFields.length > 0 ? [`Missing lifecycle fields: ${missingFields.join(', ')}.`] : []),
    ...(invalidFields.length > 0 ? [`Invalid lifecycle fields: ${invalidFields.join(', ')}.`] : []),
    ...(!ordered ? ['Declared lifecycle offsets are not monotonically ordered.'] : []),
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
    finding: {
      bundleStartTimeUnixNs: context.queryBundle.start_time_unix_ns,
      queryStartTimeUnixNs: query.start_unix_ns,
      durationSeconds: context.queryBundle.duration_s,
      offsetsSeconds: {
        planning,
        executing,
        completed,
      },
      intervalsSeconds: {
        beforePlanning: validOffset(planning) ? planning : null,
        planning: interval(planning, executing),
        execution: interval(executing, completed),
        afterCompletion:
          validOffset(completed) && completed <= context.queryBundle.duration_s
            ? context.queryBundle.duration_s - completed
            : null,
      },
      complete: missingFields.length === 0 && invalidFields.length === 0 && ordered,
      missingFields,
      invalidFields,
      ordered,
    },
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
