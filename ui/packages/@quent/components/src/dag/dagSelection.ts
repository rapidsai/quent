// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import {
  resolveOperatorSelectionCandidates,
  type DAGNode,
  type SelectedOperatorGroupData,
} from '@quent/utils';
import { parseCustomStatistics } from '../lib/queryBundle.utils';
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

function getOperatorIds(node: DAGNode): Set<string> {
  const metadata = node.metadata as QueryPlanNodeData['metadata'];
  return new Set([node.id, ...(metadata?.relatedOperatorIds ?? [])]);
}

function getSelectedOperatorData(node: DAGNode): SelectedOperatorGroupData {
  const metadata = node.metadata as QueryPlanNodeData['metadata'];
  return {
    nodeId: node.id,
    label: node.label,
    operationType: node.type,
    statistics: parseCustomStatistics(metadata?.rawNode),
    workerLabel: metadata?.operatorWorkerLabels?.[node.id],
    relatedOperators: metadata?.relatedOperators?.map(operator => ({
      nodeId: operator.id,
      label: operator.instance_name ?? operator.operator_type_name ?? 'Operator',
      operationType: operator.operator_type_name?.toLowerCase() ?? 'operator',
      statistics: parseCustomStatistics(operator),
      workerLabel: metadata?.operatorWorkerLabels?.[operator.id],
    })),
  };
}

export function resolveSelectedOperatorsFromNodes(
  nodes: readonly DAGNode[],
  selectedOperatorIds: ReadonlySet<string>
): ResolvedOperatorSelections {
  const candidates = nodes.map(node => ({
    selectionId: node.id,
    label: node.label,
    operatorIds: getOperatorIds(node),
    selectedData: getSelectedOperatorData(node),
  }));

  return resolveOperatorSelectionCandidates(candidates, selectedOperatorIds);
}
