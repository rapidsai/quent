// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { InspectedNodeData } from '@quent/hooks';
import type { DAGNode } from '@quent/utils';
import { parseCustomStatistics } from '../lib/queryBundle.utils';
import type { QueryPlanNodeData } from '../query-plan/QueryPlanNode';

export function resolveInspectedNodeData(
  nodes: readonly DAGNode[],
  selectedNodeIds: ReadonlySet<string>
): InspectedNodeData | null {
  const selected = nodes.find(node => {
    const metadata = node.metadata as QueryPlanNodeData['metadata'];
    const ids = new Set([node.id, ...(metadata?.relatedOperatorIds ?? [])]);
    return (
      ids.size === selectedNodeIds.size && [...ids].every(nodeId => selectedNodeIds.has(nodeId))
    );
  });
  if (!selected) {
    return null;
  }

  const metadata = selected.metadata as QueryPlanNodeData['metadata'];
  return {
    nodeId: selected.id,
    label: selected.label,
    operationType: selected.type,
    statistics: parseCustomStatistics(metadata?.rawNode),
    relatedOperators: metadata?.relatedOperators?.map(operator => ({
      nodeId: operator.id,
      label: operator.instance_name ?? operator.operator_type_name ?? 'Operator',
      operationType: operator.operator_type_name?.toLowerCase() ?? 'operator',
      statistics: parseCustomStatistics(operator),
    })),
  };
}
