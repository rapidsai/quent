import type { DAGData, DAGNode, DAGEdge, QueryPlanTransformer } from './types';

// Types for the "unknown" query plan format from the API
interface OperatorPort {
  id: string;
  is_input: boolean;
  name: string;
}

interface StateEntry {
  [stateName: string]: unknown;
}

interface Operator {
  id: string;
  plan_id: string;
  name: string;
  ports: OperatorPort[];
  state_sequence: StateEntry[];
}

interface PortEdge {
  source: string; // source port ID
  target: string; // target port ID
}

interface Plan {
  id: string;
  query_id: string;
  timestamps: Record<string, number>;
  parent_id: string | null;
  worker_id: string | null;
  operators: Operator[];
  edges: PortEdge[];
}

interface QueryPlan {
  id: string;
  coordinator_id: string;
  timestamps: Record<string, number>;
  name: string | null;
  plans: Plan[];
}

export class QueryEngineTransformer implements QueryPlanTransformer<QueryPlan> {
  engineName = 'queryEngine';

  validate(plan: unknown): plan is QueryPlan {
    if (typeof plan !== 'object' || plan === null) return false;

    const p = plan as Record<string, unknown>;

    // Check for the specific structure of this query plan format
    return (
      'id' in p &&
      'plans' in p &&
      Array.isArray(p.plans) &&
      p.plans.length > 0 &&
      'operators' in (p.plans[0] as Record<string, unknown>) &&
      'edges' in (p.plans[0] as Record<string, unknown>)
    );
  }

  transform(plan: QueryPlan): DAGData {
    const nodes: DAGNode[] = [];
    const edges: DAGEdge[] = [];

    // For now, we'll handle the first plan
    // TODO: handle multiple plans if needed
    const firstPlan = plan.plans[0];
    if (!firstPlan) {
      return { nodes, edges };
    }

    // Create a map from port ID to operator ID for edge resolution
    const portToOperator = new Map<string, string>();

    firstPlan.operators.forEach(operator => {
      operator.ports.forEach(port => {
        portToOperator.set(port.id, operator.id);
      });
    });

    // Create nodes from operators
    firstPlan.operators.forEach(operator => {
      const nodeType = this.categorizeOperation(operator.name);

      nodes.push({
        id: operator.id,
        label: operator.name,
        type: nodeType,
        metadata: {
          planId: operator.plan_id,
          ports: operator.ports,
          stateSequence: operator.state_sequence,
          rawNode: operator,
        },
      });
    });

    // Create edges from the port-based edges
    firstPlan.edges.forEach((edge, index) => {
      const sourceOperatorId = portToOperator.get(edge.source);
      const targetOperatorId = portToOperator.get(edge.target);

      if (sourceOperatorId && targetOperatorId) {
        edges.push({
          id: `e-${index}-${sourceOperatorId}-${targetOperatorId}`,
          source: sourceOperatorId,
          target: targetOperatorId,
          type: 'smoothstep',
        });
      }
    });

    return { nodes, edges };
  }

  private categorizeOperation(name: string): string {
    const nameLower = name.toLowerCase();

    // Map operator names to visual categories
    if (nameLower === 'scan') return 'source';
    if (nameLower === 'output') return 'output';
    if (nameLower.includes('join')) return 'join';
    if (nameLower.includes('aggregate') || nameLower.includes('agg')) return 'aggregate';
    if (nameLower === 'project') return 'project';
    if (nameLower === 'filter') return 'filter';
    if (nameLower === 'sort' || nameLower === 'order') return 'sort';
    if (nameLower === 'limit') return 'limit';
    if (nameLower.includes('exchange') || nameLower.includes('shuffle')) return 'exchange';
    if (nameLower.includes('union')) return 'union';

    return 'other';
  }
}

export const queryEngineTransformer = new QueryEngineTransformer();
