// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { DAGNode, DAGEdge, QueryPlanDataItem } from './types';
import type { QueryBundle, EntityRef } from '@quent/utils';
import {
  buildRelatedOperatorIdsById,
  Operator,
  Port,
  Plan,
  PlanTree,
  Worker,
} from '@quent/utils';

interface PlanTreeNode extends PlanTree {
  query?: string | null;
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
  relatedOperatorIdsById: Map<string, string[]>
): DAGNode | undefined => {
  // Find associated port
  if (bundle?.entities?.ports?.[id]) {
    const port: Port = bundle?.entities?.ports?.[id];
    const operator: Operator | undefined = port.operator_id
      ? bundle?.entities?.operators?.[port.operator_id]
      : undefined;
    if (operator) {
      const relatedOperatorIds = relatedOperatorIdsById.get(operator.id) ?? [];
      return {
        id: operator.id,
        label: operator.instance_name ?? operator.operator_type_name ?? 'Node',
        type: operator.operator_type_name?.toLowerCase() ?? 'operator',
        metadata: {
          rawNode: operator,
          relatedOperatorIds,
          relatedOperators: relatedOperatorIds.flatMap(id => {
            const relatedOperator = bundle.entities.operators[id];
            return relatedOperator ? [relatedOperator] : [];
          }),
        },
      };
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
    workerName: worker?.instance_name ?? undefined,
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

  // Build the DAG from the plan's edges
  planTree.edges.forEach(edge => {
    const sourceNode = getNodeEntity(bundle, edge.source, relatedOperatorIdsById);
    const targetNode = getNodeEntity(bundle, edge.target, relatedOperatorIdsById);

    if (sourceNode && targetNode) {
      // Deduplicate nodes by ID
      if (!nodeMap.has(sourceNode.id)) {
        nodeMap.set(sourceNode.id, sourceNode);
      }
      if (!nodeMap.has(targetNode.id)) {
        nodeMap.set(targetNode.id, targetNode);
      }

      edges.push({
        id: `${edge.source}-${edge.target}`,
        source: sourceNode.id,
        target: targetNode.id,
        type: 'smoothstep',
      });
    }
  });

  return {
    nodes: Array.from(nodeMap.values()),
    edges,
  };
};
