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
  portStats?: Array<{ key: string; value: StatValue }>; // from source port
}

export type ContinuousNodeColoring = {
  type: 'continuous';
  values: Map<string, number>; // operatorId → numeric value
  min: number;
  max: number;
};

export type CategoricalNodeColoring = {
  type: 'categorical';
  colorMap: Map<string, string>; // operatorId → hex color
};

export type NodeColoring = ContinuousNodeColoring | CategoricalNodeColoring | null;

export type EdgeWidthConfig = {
  values: Map<string, number>; // edgeId → numeric value
  min: number;
  max: number;
} | null;

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
