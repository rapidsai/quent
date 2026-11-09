// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import {
  summarizeOperatorStatistics,
  type OperatorStatisticsSummary,
  type StatisticPath,
} from '@quent/query-analysis';
import type {
  QuestionContext,
  QuestionDefinition,
  QuestionMetadata,
  QuestionResult,
} from './question.types';
import {
  buildQueryBundleDeepLink,
  queryBundleResolution,
  renderTable,
  validateQueryBundleContext,
} from './queryBundleQuestion.utils';

export interface OperatorStatisticsSummaryResult extends QuestionResult {
  scope: {
    engineId: string;
    queryId: string;
  };
  finding: OperatorStatisticsSummary;
}

export const operatorStatisticsSummaryMetadata: QuestionMetadata = {
  id: 'operator-statistics-summary',
  version: 1,
  title: 'Operator statistics summary',
  explanation:
    'Aggregates numeric operator-statistic leaves by logical or physical operator type, structured path, and quantity.',
  requirements: {
    apis: ['query-bundle'],
    resourceTypes: [],
    capacities: [],
    fsmStates: [],
    topologyFields: [],
  },
  parameters: [],
  limitations: [
    'Logical and physical aggregates are overlapping views and must not be added together.',
    'Physical values roll up to logical parent types; rolled-up logical operators do not also contribute their own values.',
    'Lists and non-numeric values are reported but never coerced into numeric aggregates.',
    'Variance and standard deviation use the population formula; percentiles use linear interpolation over sorted values.',
    'Means, percentiles, and dispersion are floating-point values; unsafe big integers are marked approximate.',
  ],
};

async function run(context: QuestionContext): Promise<OperatorStatisticsSummaryResult> {
  validateQueryBundleContext(context);
  const finding = summarizeOperatorStatistics(context.queryBundle);
  const approximateCount = finding.aggregates.filter(
    aggregate => aggregate.approximateFloatingPoint
  ).length;

  return {
    question: {
      id: operatorStatisticsSummaryMetadata.id,
      version: operatorStatisticsSummaryMetadata.version,
      title: operatorStatisticsSummaryMetadata.title,
    },
    evidence: {
      class: 'derived',
      explanation:
        'Numeric leaves and operator identities are observed; grouping, extrema, percentiles, and population dispersion are deterministic calculations.',
    },
    resolution: queryBundleResolution(),
    scope: {
      engineId: context.engineId,
      queryId: context.queryId,
    },
    finding,
    deepLink: buildQueryBundleDeepLink(context, { tab: 'operators' }),
    limitations: [
      ...operatorStatisticsSummaryMetadata.limitations,
      ...(finding.skippedValues.length > 0
        ? [`Skipped ${finding.skippedValues.length} missing or non-numeric statistic value(s).`]
        : []),
      ...(finding.undeclaredQuantities.length > 0
        ? [`Undeclared quantities: ${finding.undeclaredQuantities.join(', ')}.`]
        : []),
      ...(finding.groupingIssues.length > 0
        ? [`Operator grouping issues: ${finding.groupingIssues.length}.`]
        : []),
      ...(approximateCount > 0
        ? [
            `${approximateCount} aggregate(s) contain integers outside JavaScript's safe range; floating-point measures are approximate.`,
          ]
        : []),
    ],
  };
}

function pathLabel(path: StatisticPath): string {
  return path.map(([name, index]) => (index === 0 ? name : `${name}[${index}]`)).join('.');
}

function formatNumber(value: number | string): string {
  return typeof value === 'string'
    ? value
    : value.toLocaleString('en-US', { maximumSignificantDigits: 10 });
}

function formatHuman(result: OperatorStatisticsSummaryResult): string {
  const table =
    result.finding.aggregates.length === 0
      ? 'No numeric operator statistics found.'
      : renderTable(
          [
            'Plan',
            'Operator',
            'Statistic',
            'Quantity',
            'n',
            'Missing',
            'Other',
            'Sum',
            'Min',
            'Max',
            'Mean',
            'Median',
            'Stddev',
          ],
          result.finding.aggregates.map(aggregate => [
            aggregate.scope === 'logical' ? 'Logical' : 'Physical',
            aggregate.operatorType,
            pathLabel(aggregate.statisticPath),
            aggregate.quantity ?? '—',
            `${aggregate.numericValueCount}/${aggregate.operatorCount}`,
            String(aggregate.missingValueCount),
            String(aggregate.nonNumericValueCount),
            formatNumber(aggregate.sum),
            formatNumber(aggregate.minimum),
            formatNumber(aggregate.maximum),
            formatNumber(aggregate.mean),
            formatNumber(aggregate.median),
            formatNumber(aggregate.populationStandardDeviation),
          ]),
          new Set([4, 5, 6, 7, 8, 9, 10, 11, 12])
        );
  const lines = [
    `${result.question.title} (v${result.question.version})`,
    `Query: ${result.scope.queryId}`,
    `Aggregates: ${result.finding.aggregates.length}`,
    '',
    table,
  ];
  lines.push(
    '',
    `Skipped values: ${result.finding.skippedValues.length}`,
    `Evidence: ${result.evidence.class} — ${result.evidence.explanation}`,
    `Open evidence: ${result.deepLink}`,
    '',
    'Limitations:',
    ...result.limitations.map(limitation => `- ${limitation}`)
  );
  return lines.join('\n');
}

export const operatorStatisticsSummaryQuestion: QuestionDefinition<
  Record<string, never>,
  OperatorStatisticsSummaryResult
> = {
  metadata: operatorStatisticsSummaryMetadata,
  parseInput: () => ({}),
  run,
  formatHuman,
};
