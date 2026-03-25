// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { DAGNode, DAGEdge, QueryPlanDataItem } from './types';
import type { QueryBundle } from '~quent/types/QueryBundle';
import type { EntityRef } from '~quent/types/EntityRef';
import { Operator } from '~quent/types/Operator';
import { Port } from '~quent/types/Port';
import { Plan } from '~quent/types/Plan';
import { PlanTree } from '~quent/types/PlanTree';

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
const getNodeEntity = (bundle: QueryBundle<EntityRef>, id: string): DAGNode | undefined => {
  // Find associated port
  if (bundle?.entities?.ports?.[id]) {
    const port: Port = bundle?.entities?.ports?.[id];
    const operator: Operator | undefined = port.operator_id
      ? bundle?.entities?.operators?.[port.operator_id]
      : undefined;
    if (operator) {
      return {
        id: operator.id,
        label: operator.instance_name ?? operator.operator_type_name ?? 'Node',
        type: operator.operator_type_name?.toLowerCase() ?? 'operator',
        metadata: {
          rawNode: operator,
        },
      };
    }
  }

  return undefined;
};

/**
 * Recursively transform a plan node into TreeView format and provide display data
 */
const transformNodeForTreeView = (node: PlanTreeNode, plans: Plan[]): QueryPlanDataItem => {
  const plan = plans.find(plan => plan.id === node.id);

  return {
    id: node.id,
    name: `Query Plan: ${node.id}`,
    queryId: node.id ?? undefined,
    workerId: node.worker ?? undefined,
    planType: plan?.instance_name ?? undefined,
    className: 'rounded-none',
    children: node.children?.length
      ? node.children?.map(child => transformNodeForTreeView(child, plans))
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
  return [bundle.plan_tree].map(node => transformNodeForTreeView(node, plans));
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

  // Build the DAG from the plan's edges
  planTree.edges.forEach(edge => {
    const sourceNode = getNodeEntity(bundle, edge.source);
    const targetNode = getNodeEntity(bundle, edge.target);

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
