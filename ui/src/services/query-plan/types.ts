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
}

export interface PrestoQueryPlanNode {
  id: string;
  name: string;
  identifier: string;
  details: string;
  children: PrestoQueryPlanNode[];
  remoteSources: unknown[];
  estimates: unknown[];
}

// Root node structure
export interface PrestoQueryPlan {
  id: string;
  name: string;
  identifier: string;
  details: string;
  children: PrestoQueryPlanNode[];
  remoteSources: unknown[];
  estimates: unknown[];
}

// Presto Physical Plan types (distributed/stage-based)
export interface PrestoPhysicalPlanNode {
  id: string;
  name: string;
  identifier: string;
  details: string;
  children: PrestoPhysicalPlanNode[];
  remoteSources: string[]; // Stage IDs
  estimates: unknown[];
}

export interface PrestoPhysicalStage {
  plan: PrestoPhysicalPlanNode;
}

export interface PrestoPhysicalPlan {
  [stageId: string]: PrestoPhysicalStage;
}

// TODO: development only, remove once API configured
export type QueryPlanSource =
  | { type: 'local'; path: string }
  | { type: 'api'; engineId: string; queryId: string };

export interface QueryPlanTransformer<T = unknown> {
  engineName: string;
  transform(plan: T): DAGData;
  validate(plan: unknown): plan is T;
}
