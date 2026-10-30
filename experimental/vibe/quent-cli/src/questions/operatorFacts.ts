// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { analyzeOperatorFacts, type OperatorFactsAnalysis } from '@quent/query-analysis';
import type {
  QuestionCliValues,
  QuestionContext,
  QuestionDefinition,
  QuestionMetadata,
  QuestionResult,
} from './question.types';
import {
  buildQueryBundleDeepLink,
  formatSeconds,
  queryBundleResolution,
  renderTable,
  validateQueryBundleContext,
} from './queryBundleQuestion.utils';

const MAX_LINKED_OPERATORS = 50;

interface OperatorFactsInput {
  operatorIds: string[];
}

export interface OperatorFactsResult extends QuestionResult {
  scope: {
    engineId: string;
    queryId: string;
    requestedOperatorIds: string[];
  };
  finding: OperatorFactsAnalysis;
}

export const operatorFactsMetadata: QuestionMetadata = {
  id: 'operator-facts',
  version: 1,
  title: 'Operator facts',
  explanation:
    'Returns declared identity, plan, lineage, activity, attributes, statistics, and ports for operator instances.',
  requirements: {
    apis: ['query-bundle'],
    resourceTypes: [],
    capacities: [],
    fsmStates: [],
    topologyFields: [],
  },
  parameters: [
    {
      name: 'operator',
      required: false,
      description: 'Comma-separated operator IDs; all operators when omitted.',
    },
  ],
  limitations: [
    'Active-span duration includes idle gaps and is not CPU or GPU execution time.',
    'Dynamic attribute and statistic values are preserved as declared rather than coerced or aggregated.',
    'Missing optional names, plans, workers, spans, attributes, statistics, and ports remain explicit.',
  ],
};

function parseOperatorIds(value: string | boolean | undefined): string[] {
  if (value === undefined) {
    return [];
  }
  if (typeof value !== 'string') {
    throw new Error('--operator must be a comma-separated list of operator IDs.');
  }
  const ids = [
    ...new Set(
      value
        .split(',')
        .map(id => id.trim())
        .filter(Boolean)
    ),
  ].sort();
  if (ids.length === 0) {
    throw new Error('--operator must include at least one operator ID.');
  }
  if (ids.length > MAX_LINKED_OPERATORS) {
    throw new Error(`--operator supports at most ${MAX_LINKED_OPERATORS} IDs.`);
  }
  return ids;
}

function parseInput(values: QuestionCliValues): OperatorFactsInput {
  return { operatorIds: parseOperatorIds(values.operator) };
}

async function run(
  context: QuestionContext,
  input: OperatorFactsInput
): Promise<OperatorFactsResult> {
  validateQueryBundleContext(context);
  const finding = analyzeOperatorFacts(context.queryBundle, input.operatorIds);

  return {
    question: {
      id: operatorFactsMetadata.id,
      version: operatorFactsMetadata.version,
      title: operatorFactsMetadata.title,
    },
    evidence: {
      class: 'derived',
      explanation:
        'Declared operator fields are observed; child relationships, active-span durations, ordering, and integrity checks are deterministic projections.',
    },
    resolution: queryBundleResolution(),
    scope: {
      engineId: context.engineId,
      queryId: context.queryId,
      requestedOperatorIds: input.operatorIds,
    },
    finding,
    deepLink: buildQueryBundleDeepLink(context, {
      tab: 'operators',
      ...(input.operatorIds.length > 0 ? { operatorIds: input.operatorIds } : {}),
    }),
    limitations: [
      ...operatorFactsMetadata.limitations,
      ...(finding.integrityIssues.length > 0
        ? [`The selected operators contain ${finding.integrityIssues.length} integrity issue(s).`]
        : []),
    ],
  };
}

function formatHuman(result: OperatorFactsResult): string {
  const table =
    result.finding.operators.length === 0
      ? 'No operators found.'
      : renderTable(
          [
            'Operator',
            'Type',
            'Plan',
            'Scope',
            'Worker',
            'Active span',
            'Parents / children',
            'A / S / P',
          ],
          result.finding.operators.map(operator => [
            `${operator.instanceName ?? '(unnamed)'} (${operator.id})`,
            operator.typeName ?? '—',
            operator.plan?.instanceName ?? operator.plan?.id ?? '—',
            operator.plan?.scope ?? '—',
            operator.plan?.worker?.instanceName ?? operator.plan?.worker?.id ?? '—',
            operator.activeSpan?.durationSeconds == null
              ? '—'
              : formatSeconds(operator.activeSpan.durationSeconds),
            `${operator.parentOperatorIds.length} / ${operator.childOperatorIds.length}`,
            `${operator.customAttributes.length} / ${operator.statistics.length} / ${operator.ports.length}`,
          ]),
          new Set([5, 6, 7])
        );
  const lines = [
    `${result.question.title} (v${result.question.version})`,
    `Query: ${result.scope.queryId}`,
    `Operators: ${result.finding.operatorCount}`,
    `Integrity issues: ${result.finding.integrityIssues.length}`,
    '',
    table,
  ];
  lines.push(
    '',
    `Evidence: ${result.evidence.class} — ${result.evidence.explanation}`,
    `Open evidence: ${result.deepLink}`,
    '',
    'Limitations:',
    ...result.limitations.map(limitation => `- ${limitation}`)
  );
  return lines.join('\n');
}

export const operatorFactsQuestion: QuestionDefinition<OperatorFactsInput, OperatorFactsResult> = {
  metadata: operatorFactsMetadata,
  parseInput,
  run,
  formatHuman,
};
