// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { analyzeQueryStructure, type QueryStructureAnalysis } from '@quent/query-analysis';
import type {
  QuestionContext,
  QuestionDefinition,
  QuestionMetadata,
  QuestionResult,
} from './question.types';
import {
  buildQueryBundleDeepLink,
  queryBundleResolution,
  validateQueryBundleContext,
} from './queryBundleQuestion.utils';

export interface QueryStructureResult extends QuestionResult {
  scope: {
    engineId: string;
    queryId: string;
  };
  finding: QueryStructureAnalysis;
}

export const queryStructureMetadata: QuestionMetadata = {
  id: 'query-structure',
  version: 1,
  title: 'Query structure',
  explanation:
    'Projects plans, workers, operators, ports, and edges into a stable query topology with a structural fingerprint.',
  requirements: {
    apis: ['query-bundle'],
    resourceTypes: [],
    capacities: [],
    fsmStates: [],
    topologyFields: [],
  },
  parameters: [],
  limitations: [
    'The fingerprint captures operator types and plan topology, not SQL text or semantic equivalence.',
    'Operator statistics, attributes, and runtime activity are intentionally omitted from this structural view.',
    'Map ordering and entity IDs do not affect the structural fingerprint.',
  ],
};

async function run(context: QuestionContext): Promise<QueryStructureResult> {
  validateQueryBundleContext(context);
  const finding = await analyzeQueryStructure(context.queryBundle);

  return {
    question: {
      id: queryStructureMetadata.id,
      version: queryStructureMetadata.version,
      title: queryStructureMetadata.title,
    },
    evidence: {
      class: 'derived',
      explanation:
        'Entity identities and edges are observed; counts, stable ordering, integrity checks, and the structural fingerprint are deterministic projections.',
    },
    resolution: queryBundleResolution(),
    scope: {
      engineId: context.engineId,
      queryId: context.queryId,
    },
    finding,
    deepLink: buildQueryBundleDeepLink(context, { tab: 'operators' }),
    limitations: [
      ...queryStructureMetadata.limitations,
      ...(finding.integrityIssues.length > 0
        ? [
            `The query bundle contains ${finding.integrityIssues.length} topology integrity issue(s).`,
          ]
        : []),
    ],
  };
}

function formatHuman(result: QueryStructureResult): string {
  const { counts, fingerprint, observedOperatorTypes, integrityIssues } = result.finding;
  return [
    `${result.question.title} (v${result.question.version})`,
    `Query: ${result.scope.queryId}`,
    `Workers: ${counts.workers}`,
    `Plans: ${counts.plans}`,
    `Operators: ${counts.operators} across ${counts.operatorTypes} observed types`,
    `Ports / edges: ${counts.ports} / ${counts.edges}`,
    `Operator types: ${observedOperatorTypes.join(', ') || '(none)'}`,
    `Structural fingerprint: ${fingerprint.value}`,
    `Integrity issues: ${integrityIssues.length}`,
    `Evidence: ${result.evidence.class} — ${result.evidence.explanation}`,
    `Open evidence: ${result.deepLink}`,
    '',
    'Limitations:',
    ...result.limitations.map(limitation => `- ${limitation}`),
  ].join('\n');
}

export const queryStructureQuestion: QuestionDefinition<
  Record<string, never>,
  QueryStructureResult
> = {
  metadata: queryStructureMetadata,
  parseInput: () => ({}),
  run,
  formatHuman,
};
