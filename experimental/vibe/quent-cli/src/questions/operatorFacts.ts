// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type {
  DynamicAttribute,
  DynamicValue,
  OperatorStatistic,
  PortStatistics,
} from '@quent/utils';
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
  validateQueryBundleContext,
} from './queryBundleQuestion.utils';

const MAX_LINKED_OPERATORS = 50;

interface OperatorFactsInput {
  operatorIds: string[];
}

interface NamedDynamicValue {
  path: string[];
  value: DynamicValue | null;
}

interface NamedOperatorStatistic extends NamedDynamicValue {
  quantity: string | null;
}

export interface OperatorFactsResult extends QuestionResult {
  scope: {
    engineId: string;
    queryId: string;
    requestedOperatorIds: string[];
  };
  finding: {
    operatorCount: number;
    operators: Array<{
      id: string;
      instanceName: string | null;
      typeName: string | null;
      plan: {
        id: string;
        instanceName: string | null;
        parentPlanId: string | null;
        scope: 'logical' | 'physical';
        worker: {
          id: string;
          instanceName: string | null;
        } | null;
      } | null;
      parentOperatorIds: string[];
      childOperatorIds: string[];
      activeSpan: {
        startSeconds: number;
        endSeconds: number;
        durationSeconds: number | null;
      } | null;
      customAttributes: NamedDynamicValue[];
      statistics: NamedOperatorStatistic[];
      ports: Array<{
        id: string;
        instanceName: string | null;
        statistics: NamedDynamicValue[];
      }>;
    }>;
    integrityIssues: string[];
  };
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

function dynamicValues(values: DynamicAttribute[]): NamedDynamicValue[] {
  return [...values]
    .sort((left, right) => left.key.localeCompare(right.key))
    .map(attribute => ({ path: [attribute.key], value: attribute.value }));
}

function operatorStatistics(values: OperatorStatistic[]): NamedOperatorStatistic[] {
  return [...values]
    .sort((left, right) => left.value.key.localeCompare(right.value.key))
    .map(statistic => ({
      path: [statistic.value.key],
      value: statistic.value.value,
      quantity: statistic.quantity,
    }));
}

function portStatistics(statistics: PortStatistics | null): NamedDynamicValue[] {
  return statistics ? dynamicValues(statistics.custom_statistics) : [];
}

async function run(
  context: QuestionContext,
  input: OperatorFactsInput
): Promise<OperatorFactsResult> {
  validateQueryBundleContext(context);
  const { operators, plans, workers, ports } = context.queryBundle.entities;
  for (const id of input.operatorIds) {
    if (!operators[id]) {
      throw new Error(`Operator "${id}" is not present in the query bundle.`);
    }
  }
  const selected = (
    input.operatorIds.length > 0
      ? input.operatorIds.map(id => operators[id]!)
      : Object.values(operators)
  ).sort((left, right) => left.id.localeCompare(right.id));
  const childIdsByParent = new Map<string, string[]>();
  for (const operator of Object.values(operators)) {
    for (const parentId of operator.parent_operator_ids) {
      const childIds = childIdsByParent.get(parentId) ?? [];
      childIds.push(operator.id);
      childIdsByParent.set(parentId, childIds);
    }
  }
  for (const ids of childIdsByParent.values()) {
    ids.sort();
  }
  const portsByOperator = new Map<string, (typeof ports)[string][]>();
  for (const port of Object.values(ports)) {
    if (port.operator_id) {
      const operatorPorts = portsByOperator.get(port.operator_id) ?? [];
      operatorPorts.push(port);
      portsByOperator.set(port.operator_id, operatorPorts);
    }
  }
  for (const operatorPorts of portsByOperator.values()) {
    operatorPorts.sort((left, right) => left.id.localeCompare(right.id));
  }
  const issues: string[] = [];
  const projected = selected.map(operator => {
    const plan = operator.plan_id ? plans[operator.plan_id] : undefined;
    if (operator.plan_id && !plan) {
      issues.push(`Operator "${operator.id}" references missing plan "${operator.plan_id}".`);
    }
    for (const parentId of operator.parent_operator_ids) {
      if (!operators[parentId]) {
        issues.push(`Operator "${operator.id}" references missing parent operator "${parentId}".`);
      }
    }
    const worker = plan?.worker_id ? workers[plan.worker_id] : undefined;
    if (plan?.worker_id && !worker) {
      issues.push(`Plan "${plan.id}" references missing worker "${plan.worker_id}".`);
    }
    const activeSpan = operator.active_span;
    const duration =
      activeSpan &&
      Number.isFinite(activeSpan.start) &&
      Number.isFinite(activeSpan.end) &&
      activeSpan.end >= activeSpan.start
        ? activeSpan.end - activeSpan.start
        : null;
    if (activeSpan && duration === null) {
      issues.push(`Operator "${operator.id}" has an invalid active span.`);
    }
    return {
      id: operator.id,
      instanceName: operator.instance_name,
      typeName: operator.operator_type_name,
      plan: plan
        ? {
            id: plan.id,
            instanceName: plan.instance_name,
            parentPlanId: plan.parent,
            scope: plan.worker_id === null ? ('logical' as const) : ('physical' as const),
            worker: worker
              ? {
                  id: worker.id,
                  instanceName: worker.instance_name,
                }
              : null,
          }
        : null,
      parentOperatorIds: [...operator.parent_operator_ids].sort(),
      childOperatorIds: childIdsByParent.get(operator.id) ?? [],
      activeSpan: activeSpan
        ? {
            startSeconds: activeSpan.start,
            endSeconds: activeSpan.end,
            durationSeconds: duration,
          }
        : null,
      customAttributes: dynamicValues(operator.custom_attributes),
      statistics: operator.statistics
        ? operatorStatistics(operator.statistics.custom_statistics)
        : [],
      ports: (portsByOperator.get(operator.id) ?? []).map(port => ({
        id: port.id,
        instanceName: port.instance_name,
        statistics: portStatistics(port.statistics),
      })),
    };
  });
  const integrityIssues = [...new Set(issues)].sort();

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
    finding: {
      operatorCount: projected.length,
      operators: projected,
      integrityIssues,
    },
    deepLink: buildQueryBundleDeepLink(context, {
      tab: 'operators',
      ...(input.operatorIds.length > 0 ? { operatorIds: input.operatorIds } : {}),
    }),
    limitations: [
      ...operatorFactsMetadata.limitations,
      ...(integrityIssues.length > 0
        ? [`The selected operators contain ${integrityIssues.length} integrity issue(s).`]
        : []),
    ],
  };
}

function formatHuman(result: OperatorFactsResult): string {
  const lines = [
    `${result.question.title} (v${result.question.version})`,
    `Query: ${result.scope.queryId}`,
    `Operators: ${result.finding.operatorCount}`,
    `Integrity issues: ${result.finding.integrityIssues.length}`,
    '',
  ];
  for (const operator of result.finding.operators) {
    lines.push(
      `${operator.instanceName ?? '(unnamed)'} (${operator.typeName ?? '(unknown)'}, ${operator.id})`,
      `  Plan: ${operator.plan?.instanceName ?? operator.plan?.id ?? 'unavailable'} [${
        operator.plan?.scope ?? 'unknown'
      }]`,
      `  Active span: ${
        operator.activeSpan?.durationSeconds === null || operator.activeSpan === null
          ? 'unavailable'
          : formatSeconds(operator.activeSpan.durationSeconds)
      }`,
      `  Attributes / statistics / ports: ${operator.customAttributes.length} / ${operator.statistics.length} / ${operator.ports.length}`
    );
  }
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
