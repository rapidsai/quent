// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import {
  resolveOperatorSelectionCandidates,
  type DAGNode,
  type SelectedOperatorGroupData,
} from '@quent/utils';
import { parseOperatorAttributes } from '../lib/queryBundle.utils';
import type { QueryPlanNodeData } from '../query-plan/QueryPlanNode';

export interface ResolvedOperatorSelection {
  selectionId: string;
  label: string;
  operatorIds: ReadonlySet<string>;
  selectedData: SelectedOperatorGroupData;
}

export interface ResolvedOperatorSelections {
  selections: ResolvedOperatorSelection[];
  unresolvedOperatorIds: ReadonlySet<string>;
}

export function getRelatedOperatorIds(node: DAGNode): readonly string[] {
  const metadata = node.metadata as QueryPlanNodeData['metadata'];
  return metadata?.relatedOperatorIds ?? [];
}

export function getOperatorIds(node: DAGNode): Set<string> {
  return new Set([node.id, ...getRelatedOperatorIds(node)]);
}

export function getSelectedOperatorData(node: DAGNode): SelectedOperatorGroupData {
  const metadata = node.metadata as QueryPlanNodeData['metadata'];
  return {
    nodeId: node.id,
    label: node.label,
    operationType: node.type,
    attributes: parseOperatorAttributes(metadata?.rawNode),
    statistics: metadata?.aggregatedStatistics?.length
      ? metadata.aggregatedStatistics
      : (metadata?.operatorStatistics?.statistics ?? []),
    workerLabel: metadata?.operatorWorkerLabels?.[node.id],
    locationLabel: metadata?.operatorLocationLabels?.[node.id],
    relatedOperators: metadata?.relatedOperators?.map((operator, index) => ({
      nodeId: operator.id,
      label: operator.instance_name ?? operator.operator_type_name ?? 'Operator',
      operationType: operator.operator_type_name?.toLowerCase() ?? 'operator',
      attributes: parseOperatorAttributes(operator),
      statistics: metadata.relatedOperatorStatistics?.[index]?.statistics ?? [],
      workerLabel: metadata?.operatorWorkerLabels?.[operator.id],
      locationLabel: metadata?.operatorLocationLabels?.[operator.id],
    })),
  };
}

export function operatorSelectionFromNode(node: DAGNode): ResolvedOperatorSelection {
  return {
    selectionId: node.id,
    label: node.label,
    operatorIds: getOperatorIds(node),
    selectedData: getSelectedOperatorData(node),
  };
}

export function resolveSelectedOperatorsFromNodes(
  nodes: readonly DAGNode[],
  selectedOperatorIds: ReadonlySet<string>
): ResolvedOperatorSelections {
  const candidates = nodes.map(operatorSelectionFromNode);

  return resolveOperatorSelectionCandidates(candidates, selectedOperatorIds);
}
