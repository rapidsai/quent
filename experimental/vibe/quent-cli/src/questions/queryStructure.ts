// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { createHash } from 'node:crypto';
import type { PlanTree, QueryBundle, EntityRef } from '@quent/utils';
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

interface StructureEdge {
  sourcePortId: string;
  sourceOperatorId: string | null;
  targetPortId: string;
  targetOperatorId: string | null;
}

interface StructurePlanTree {
  id: string;
  workerId: string | null;
  children: StructurePlanTree[];
}

export interface QueryStructureResult extends QuestionResult {
  scope: {
    engineId: string;
    queryId: string;
  };
  finding: {
    fingerprint: {
      algorithm: 'sha256';
      value: string;
      basis: 'operator-types-and-plan-topology';
    };
    counts: {
      workers: number;
      plans: number;
      operators: number;
      operatorTypes: number;
      ports: number;
      edges: number;
    };
    declaredOperatorTypes: string[];
    observedOperatorTypes: string[];
    workers: Array<{
      id: string;
      instanceName: string | null;
      parentEngineId: string | null;
    }>;
    plans: Array<{
      id: string;
      instanceName: string | null;
      parentPlanId: string | null;
      workerId: string | null;
      operatorIds: string[];
      edges: StructureEdge[];
    }>;
    operators: Array<{
      id: string;
      instanceName: string | null;
      typeName: string | null;
      planId: string | null;
      parentOperatorIds: string[];
      portIds: string[];
    }>;
    ports: Array<{
      id: string;
      instanceName: string | null;
      operatorId: string | null;
    }>;
    planTree: StructurePlanTree;
    integrityIssues: string[];
  };
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

function compareId<T extends { id: string }>(left: T, right: T): number {
  return left.id.localeCompare(right.id);
}

function structureTree(node: PlanTree): StructurePlanTree {
  return {
    id: node.id,
    workerId: node.worker,
    children: node.children.map(structureTree),
  };
}

function structuralFingerprint(bundle: QueryBundle<EntityRef>): string {
  const { plans, operators, ports } = bundle.entities;
  const signature = (node: PlanTree): unknown => {
    const plan = plans[node.id];
    const planOperators = Object.values(operators)
      .filter(operator => operator.plan_id === node.id)
      .map(operator => operator.operator_type_name ?? '(unknown)')
      .sort();
    const edgeTypes = (plan?.edges ?? [])
      .map(edge => {
        const sourceOperatorId = ports[edge.source]?.operator_id;
        const targetOperatorId = ports[edge.target]?.operator_id;
        return [
          sourceOperatorId
            ? (operators[sourceOperatorId]?.operator_type_name ?? '(unknown)')
            : '(missing)',
          targetOperatorId
            ? (operators[targetOperatorId]?.operator_type_name ?? '(unknown)')
            : '(missing)',
        ];
      })
      .sort(([leftSource, leftTarget], [rightSource, rightTarget]) =>
        `${leftSource}\0${leftTarget}`.localeCompare(`${rightSource}\0${rightTarget}`)
      );
    const children = node.children
      .map(child => signature(child))
      .sort((left, right) => JSON.stringify(left).localeCompare(JSON.stringify(right)));
    return {
      scope: node.worker === null ? 'logical' : 'physical',
      operators: planOperators,
      edges: edgeTypes,
      children,
    };
  };
  return createHash('sha256')
    .update(JSON.stringify(signature(bundle.plan_tree)))
    .digest('hex');
}

function collectIntegrityIssues(bundle: QueryBundle<EntityRef>): string[] {
  const { plans, workers, operators, ports } = bundle.entities;
  const issues: string[] = [];
  const treePlanIds = new Set<string>();
  const visit = (node: PlanTree) => {
    treePlanIds.add(node.id);
    if (!plans[node.id]) {
      issues.push(`Plan tree references missing plan "${node.id}".`);
    }
    if (node.worker && !workers[node.worker]) {
      issues.push(`Plan tree references missing worker "${node.worker}".`);
    }
    node.children.forEach(visit);
  };
  visit(bundle.plan_tree);
  for (const plan of Object.values(plans)) {
    if (!treePlanIds.has(plan.id)) {
      issues.push(`Plan "${plan.id}" is absent from the plan tree.`);
    }
    if (plan.parent && !plans[plan.parent]) {
      issues.push(`Plan "${plan.id}" references missing parent plan "${plan.parent}".`);
    }
    if (plan.worker_id && !workers[plan.worker_id]) {
      issues.push(`Plan "${plan.id}" references missing worker "${plan.worker_id}".`);
    }
    for (const edge of plan.edges) {
      if (!ports[edge.source]) {
        issues.push(`Plan "${plan.id}" edge references missing source port "${edge.source}".`);
      }
      if (!ports[edge.target]) {
        issues.push(`Plan "${plan.id}" edge references missing target port "${edge.target}".`);
      }
    }
  }
  for (const operator of Object.values(operators)) {
    if (operator.plan_id && !plans[operator.plan_id]) {
      issues.push(`Operator "${operator.id}" references missing plan "${operator.plan_id}".`);
    }
    for (const parentId of operator.parent_operator_ids) {
      if (!operators[parentId]) {
        issues.push(`Operator "${operator.id}" references missing parent operator "${parentId}".`);
      }
    }
  }
  for (const port of Object.values(ports)) {
    if (port.operator_id && !operators[port.operator_id]) {
      issues.push(`Port "${port.id}" references missing operator "${port.operator_id}".`);
    }
  }
  return [...new Set(issues)].sort();
}

async function run(context: QuestionContext): Promise<QueryStructureResult> {
  validateQueryBundleContext(context);
  const { workers, plans, operators, ports } = context.queryBundle.entities;
  const sortedWorkers = Object.values(workers).sort(compareId);
  const sortedPlans = Object.values(plans).sort(compareId);
  const sortedOperators = Object.values(operators).sort(compareId);
  const sortedPorts = Object.values(ports).sort(compareId);
  const portsByOperator = new Map<string, string[]>();
  for (const port of sortedPorts) {
    if (port.operator_id) {
      const ids = portsByOperator.get(port.operator_id) ?? [];
      ids.push(port.id);
      portsByOperator.set(port.operator_id, ids);
    }
  }
  const operatorIdsByPlan = new Map<string, string[]>();
  for (const operator of sortedOperators) {
    if (operator.plan_id) {
      const ids = operatorIdsByPlan.get(operator.plan_id) ?? [];
      ids.push(operator.id);
      operatorIdsByPlan.set(operator.plan_id, ids);
    }
  }
  const integrityIssues = collectIntegrityIssues(context.queryBundle);
  const observedOperatorTypes = [
    ...new Set(
      sortedOperators.flatMap(operator =>
        operator.operator_type_name === null ? [] : [operator.operator_type_name]
      )
    ),
  ].sort();

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
    finding: {
      fingerprint: {
        algorithm: 'sha256',
        value: structuralFingerprint(context.queryBundle),
        basis: 'operator-types-and-plan-topology',
      },
      counts: {
        workers: sortedWorkers.length,
        plans: sortedPlans.length,
        operators: sortedOperators.length,
        operatorTypes: observedOperatorTypes.length,
        ports: sortedPorts.length,
        edges: sortedPlans.reduce((count, plan) => count + plan.edges.length, 0),
      },
      declaredOperatorTypes: [...context.queryBundle.unique_operator_names].sort(),
      observedOperatorTypes,
      workers: sortedWorkers.map(worker => ({
        id: worker.id,
        instanceName: worker.instance_name,
        parentEngineId: worker.parent_engine_id,
      })),
      plans: sortedPlans.map(plan => ({
        id: plan.id,
        instanceName: plan.instance_name,
        parentPlanId: plan.parent,
        workerId: plan.worker_id,
        operatorIds: operatorIdsByPlan.get(plan.id) ?? [],
        edges: plan.edges
          .map(edge => ({
            sourcePortId: edge.source,
            sourceOperatorId: ports[edge.source]?.operator_id ?? null,
            targetPortId: edge.target,
            targetOperatorId: ports[edge.target]?.operator_id ?? null,
          }))
          .sort(
            (left, right) =>
              left.sourcePortId.localeCompare(right.sourcePortId) ||
              left.targetPortId.localeCompare(right.targetPortId)
          ),
      })),
      operators: sortedOperators.map(operator => ({
        id: operator.id,
        instanceName: operator.instance_name,
        typeName: operator.operator_type_name,
        planId: operator.plan_id,
        parentOperatorIds: [...operator.parent_operator_ids].sort(),
        portIds: portsByOperator.get(operator.id) ?? [],
      })),
      ports: sortedPorts.map(port => ({
        id: port.id,
        instanceName: port.instance_name,
        operatorId: port.operator_id,
      })),
      planTree: structureTree(context.queryBundle.plan_tree),
      integrityIssues,
    },
    deepLink: buildQueryBundleDeepLink(context, { tab: 'operators' }),
    limitations: [
      ...queryStructureMetadata.limitations,
      ...(integrityIssues.length > 0
        ? [`The query bundle contains ${integrityIssues.length} topology integrity issue(s).`]
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
