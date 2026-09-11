// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import {
  resolveOperatorSelectionCandidates,
  type DAGNode,
  type InspectedNodeData,
} from '@quent/utils';
import {
  parseCustomStatistics,
  parseOperatorInformation,
  parseOperatorObservations,
  parseOperatorPortRelations,
} from '../lib/queryBundle.utils';
import type { QueryPlanNodeData } from '../query-plan/QueryPlanNode';

export interface ResolvedOperatorSelection {
  selectionId: string;
  label: string;
  operatorIds: ReadonlySet<string>;
  inspectedData: InspectedNodeData;
}

export interface ResolvedOperatorSelections {
  selections: ResolvedOperatorSelection[];
  unresolvedOperatorIds: ReadonlySet<string>;
}

function getOperatorIds(node: DAGNode): Set<string> {
  const metadata = node.metadata as QueryPlanNodeData['metadata'];
  return new Set([node.id, ...(metadata?.relatedOperatorIds ?? [])]);
}

function inspectNode(node: DAGNode): InspectedNodeData {
  const metadata = node.metadata as QueryPlanNodeData['metadata'];
  return {
    nodeId: node.id,
    label: node.label,
    operationType: node.type,
    statistics: parseCustomStatistics(metadata?.rawNode),
    information: parseOperatorInformation(metadata?.rawNode),
    observations: parseOperatorObservations(metadata?.rawNode),
    portRelations: parseOperatorPortRelations(metadata?.rawNode),
    relatedOperators: metadata?.relatedOperators?.map(operator => ({
      nodeId: operator.id,
      label: operator.instance_name ?? operator.operator_type_name ?? 'Operator',
      operationType: operator.operator_type_name?.toLowerCase() ?? 'operator',
      statistics: parseCustomStatistics(operator),
      information: parseOperatorInformation(operator),
      observations: parseOperatorObservations(operator),
      portRelations: parseOperatorPortRelations(operator),
    })),
  };
}

export function resolveInspectedNodeSelections(
  nodes: readonly DAGNode[],
  selectedNodeIds: ReadonlySet<string>
): ResolvedOperatorSelections {
  const candidates = nodes.map(node => ({
    selectionId: node.id,
    label: node.label,
    operatorIds: getOperatorIds(node),
    inspectedData: inspectNode(node),
  }));

  return resolveOperatorSelectionCandidates(candidates, selectedNodeIds);
}

export function resolveInspectedNodeData(
  nodes: readonly DAGNode[],
  selectedNodeIds: ReadonlySet<string>
): InspectedNodeData | null {
  const resolved = resolveInspectedNodeSelections(nodes, selectedNodeIds);
  if (resolved.selections.length !== 1 || resolved.unresolvedOperatorIds.size > 0) {
    return null;
  }
  return resolved.selections[0].inspectedData;
}
