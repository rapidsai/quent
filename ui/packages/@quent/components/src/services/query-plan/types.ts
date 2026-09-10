// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { TreeDataItem } from '../../ui/tree-view';
import type { DAGNode, DAGEdge, QuantitySpec } from '@quent/utils';

export interface QueryPlanDataItem extends TreeDataItem {
  queryId?: string;
  workerId?: string;
  workerName?: string;
  planType?: string;
}

// Re-export DAG node/edge/stat types from @quent/utils (canonical location)
export type { DAGNode, DAGEdge, StatValue } from '@quent/utils';

// Re-export DAG coloring types from @quent/utils (canonical location)
export type {
  ContinuousNodeColoring,
  CategoricalNodeColoring,
  NodeColoring,
  EdgeWidthConfig,
  ContinuousEdgeColoring,
  CategoricalEdgeColoring,
  EdgeColoring,
} from '@quent/utils';

export interface DAGData {
  nodes: DAGNode[];
  edges: DAGEdge[];
  queryData: QueryPlanDataItem[];
  quantitySpecs?: { [key in string]?: QuantitySpec };
}

export interface QueryPlanNodeData extends Record<string, unknown> {
  nodeId: string;
  label: string;
  operationType: string;
  metadata?: Record<string, unknown>;
  hasIncoming?: boolean;
  hasOutgoing?: boolean;
}
