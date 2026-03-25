// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { TreeDataItem } from '@/components/ui/tree-view';

export interface QueryPlanDataItem extends TreeDataItem {
  queryId?: string;
  workerId?: string;
  planType?: string;
}

export interface DAGNode {
  id: string;
  label: string;
  type: string;
  metadata?: {
    details?: string;
    estimates?: unknown[];
    identifier?: string;
    rawNode?: unknown;
    stageId?: string;
    [key: string]: unknown;
  };
}

export interface DAGEdge {
  id: string;
  source: string;
  target: string;
  type?: 'smoothstep' | 'default' | 'straight';
}

export interface DAGData {
  nodes: DAGNode[];
  edges: DAGEdge[];
  queryData: QueryPlanDataItem[];
}

export interface QueryPlanNodeData extends Record<string, unknown> {
  nodeId: string;
  label: string;
  operationType: string;
  metadata?: Record<string, unknown>;
  hasIncoming?: boolean;
  hasOutgoing?: boolean;
}

export type StatValue = string | number | boolean | null | string[];
