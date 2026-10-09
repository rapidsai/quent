// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { parseCustomStatistics, parsePortStatistics } from '../../lib/queryBundle.utils';
import type { DAGNode, DAGEdge, QueryPlanDataItem } from './types';
import type { QueryBundle, EntityRef } from '@quent/utils';
import {
  aggregateNumericValues,
  buildRelatedOperatorIdsById,
  flattenStatistics,
  isNumericValue,
  operatorWorkerLabel,
  statisticFieldLabel,
  workerDisplayName,
  Operator,
  Port,
  Plan,
  PlanTree,
  Worker,
  type DAGStatisticSet,
  type Statistic,
  type StatisticField,
} from '@quent/utils';

interface PlanTreeNode extends PlanTree {
  query?: string | null;
}

function processStatistics(statistics: DAGStatisticSet['statistics']): DAGStatisticSet {
  return { statistics, fields: flattenStatistics(statistics) };
}

function sumStatisticSets(statisticSets: readonly DAGStatisticSet[]): Statistic[] {
  const buckets = new Map<
    string,
    { field: StatisticField; values: Array<number | bigint>; fields: StatisticField[] }
  >();
  for (const statisticSet of statisticSets) {
    for (const field of statisticSet.fields) {
      if (!isNumericValue(field.value)) {
        continue;
      }
      const bucket = buckets.get(field.key) ?? { field, values: [], fields: [] };
      bucket.values.push(field.value);
      bucket.fields.push(field);
      buckets.set(field.key, bucket);
    }
  }
  return [...buckets.values()].flatMap(({ field, values, fields }) => {
    const sum = aggregateNumericValues(values)?.sum;
    if (sum == null) {
      return [];
    }
    const quantity = fields[0].quantity;
    const hasConsistentQuantity = fields.every(candidate => candidate.quantity === quantity);
    return [
      {
        key: statisticFieldLabel(field),
        value: sum,
        ...(hasConsistentQuantity && quantity !== undefined ? { quantity } : {}),
      },
    ];
  });
}

/**
 * Validate that a query bundle has the required structure
 */
export const validateQueryBundle = (
  bundle: QueryBundle<EntityRef>
): bundle is QueryBundle<EntityRef> =>
  typeof bundle === 'object' && bundle !== null && Object.keys(bundle?.entities?.plans).length > 0;

/**
 * Retrieve the operator node entity from a port id
 */
const getNodeEntity = (
  bundle: QueryBundle<EntityRef>,
  id: string,
  relatedOperatorIdsById: Map<string, string[]>,
  operatorStatisticsById: ReadonlyMap<string, DAGStatisticSet>,
  nodeMap: Map<string, DAGNode>
): DAGNode | undefined => {
  // Find associated port
  if (bundle?.entities?.ports?.[id]) {
    const port: Port = bundle?.entities?.ports?.[id];
    const operator: Operator | undefined = port.operator_id
      ? bundle?.entities?.operators?.[port.operator_id]
      : undefined;
    if (operator) {
      const cached = nodeMap.get(operator.id);
      if (cached) {
        return cached;
      }
      const relatedOperatorIds = relatedOperatorIdsById.get(operator.id) ?? [];
      const relatedOperators = relatedOperatorIds.flatMap(id => {
        const relatedOperator = bundle.entities.operators[id];
        return relatedOperator ? [relatedOperator] : [];
      });
      const operatorStatistics = operatorStatisticsById.get(operator.id);
      const relatedOperatorStatistics: DAGStatisticSet[] = [];
      for (const relatedOperator of relatedOperators) {
        const statistics = operatorStatisticsById.get(relatedOperator.id);
        if (!statistics) {
          return undefined;
        }
        relatedOperatorStatistics.push(statistics);
      }
      if (!operatorStatistics) {
        return undefined;
      }
      const operatorWorkerLabels: Record<string, string | undefined> = {
        [operator.id]: operatorWorkerLabel(
          operator,
          bundle.entities.plans,
          bundle.entities.workers
        ),
      };
      for (const relatedOperator of relatedOperators) {
        operatorWorkerLabels[relatedOperator.id] = operatorWorkerLabel(
          relatedOperator,
          bundle.entities.plans,
          bundle.entities.workers
        );
      }
      const node: DAGNode = {
        id: operator.id,
        label: operator.instance_name ?? operator.operator_type_name ?? 'Node',
        type: operator.operator_type_name?.toLowerCase() ?? 'operator',
        metadata: {
          rawNode: operator,
          aggregatedStatistics:
            operatorStatistics.statistics.length === 0
              ? sumStatisticSets(relatedOperatorStatistics)
              : [],
          operatorStatistics,
          relatedOperatorIds,
          relatedOperators,
          relatedOperatorStatistics,
          operatorWorkerLabels,
        },
      };
      return node;
    }
  }

  return undefined;
};

/**
 * Recursively transform a plan node into TreeView format and provide display data
 */
const transformNodeForTreeView = (
  node: PlanTreeNode,
  plans: Plan[],
  workers: { [id: string]: Worker | undefined }
): QueryPlanDataItem => {
  const plan = plans.find(plan => plan.id === node.id);
  const worker = node.worker ? workers[node.worker] : undefined;

  return {
    id: node.id,
    name: `Query Plan: ${node.id}`,
    queryId: node.id ?? undefined,
    workerId: node.worker ?? undefined,
    workerName: worker ? workerDisplayName(worker) : undefined,
    planType: plan?.instance_name ?? undefined,
    className: 'rounded-none',
    children: node.children?.length
      ? node.children?.map(child => transformNodeForTreeView(child, plans, workers))
      : undefined,
  };
};

/**
 * Transform the plan_tree into TreeView format for query plan explorer
 */
export const getTreeData = (bundle: QueryBundle<EntityRef>): QueryPlanDataItem[] => {
  if (!validateQueryBundle(bundle)) {
    throw new Error('Invalid QueryBundle format');
  }

  const plans = Object.values(bundle.entities.plans).filter(
    (plan): plan is Plan => plan !== undefined
  );
  return [bundle.plan_tree].map(node =>
    transformNodeForTreeView(node, plans, bundle.entities.workers ?? {})
  );
};

export const getSelectedOperatorCountsByPlan = (
  bundle: QueryBundle<EntityRef>,
  selectedOperatorIds: ReadonlySet<string>
): Map<string, number> => {
  const counts = new Map<string, number>();
  for (const operatorId of selectedOperatorIds) {
    const planId = bundle.entities.operators[operatorId]?.plan_id;
    if (planId) {
      counts.set(planId, (counts.get(planId) ?? 0) + 1);
    }
  }
  return counts;
};

/**
 * Transform specified query plan into DAG visualization data
 */
export const getPlanDAG = (
  bundle: QueryBundle<EntityRef>,
  planId: string
): { nodes: DAGNode[]; edges: DAGEdge[] } => {
  if (!validateQueryBundle(bundle)) {
    throw new Error('Invalid QueryBundle format');
  }

  const nodeMap = new Map<string, DAGNode>();
  const edges: DAGEdge[] = [];
  const plans = Object.values(bundle.entities.plans).filter(
    (plan): plan is Plan => plan !== undefined
  );
  const planTree = plans.find(plan => plan.id === planId) || plans[0];

  if (!planTree) {
    throw new Error(`No plan found for planId: ${planId}`);
  }

  const selectedOperatorIds = new Set(
    planTree.edges.flatMap(edge =>
      [edge.source, edge.target].flatMap(portId => {
        const operatorId = bundle.entities.ports[portId]?.operator_id;
        return operatorId ? [operatorId] : [];
      })
    )
  );
  const operators = Object.values(bundle.entities.operators).filter(
    (operator): operator is Operator => operator !== undefined
  );
  const relatedOperatorIdsById = buildRelatedOperatorIdsById(operators, selectedOperatorIds);
  const relevantOperatorIds = new Set(selectedOperatorIds);
  for (const operatorId of selectedOperatorIds) {
    for (const relatedOperatorId of relatedOperatorIdsById.get(operatorId) ?? []) {
      relevantOperatorIds.add(relatedOperatorId);
    }
  }
  const operatorStatisticsById = new Map(
    [...relevantOperatorIds].flatMap(operatorId => {
      const operator = bundle.entities.operators[operatorId];
      return operator
        ? [[operatorId, processStatistics(parseCustomStatistics(operator))] as const]
        : [];
    })
  );
  const portStatisticsById = new Map(
    [...new Set(planTree.edges.flatMap(edge => [edge.source, edge.target]))].map(
      portId =>
        [portId, processStatistics(parsePortStatistics(bundle.entities.ports[portId]))] as const
    )
  );

  // Build the DAG from the plan's edges
  planTree.edges.forEach(edge => {
    const sourceNode = getNodeEntity(
      bundle,
      edge.source,
      relatedOperatorIdsById,
      operatorStatisticsById,
      nodeMap
    );
    const targetNode = getNodeEntity(
      bundle,
      edge.target,
      relatedOperatorIdsById,
      operatorStatisticsById,
      nodeMap
    );
    const sourceStatistics = portStatisticsById.get(edge.source);
    const targetStatistics = portStatisticsById.get(edge.target);

    if (sourceNode && targetNode && sourceStatistics && targetStatistics) {
      nodeMap.set(sourceNode.id, sourceNode);
      nodeMap.set(targetNode.id, targetNode);
      edges.push({
        id: `${edge.source}-${edge.target}`,
        source: sourceNode.id,
        target: targetNode.id,
        type: 'smoothstep',
        sourcePortId: edge.source,
        targetPortId: edge.target,
        sourcePortName: bundle.entities.ports[edge.source]?.instance_name ?? undefined,
        targetPortName: bundle.entities.ports[edge.target]?.instance_name ?? undefined,
        portStats: sourceStatistics.statistics,
        statisticFields: sourceStatistics.fields,
        targetPortStats: targetStatistics.statistics,
        targetStatisticFields: targetStatistics.fields,
      });
    }
  });

  return {
    nodes: Array.from(nodeMap.values()),
    edges,
  };
};
