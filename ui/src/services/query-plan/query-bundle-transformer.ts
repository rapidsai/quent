import type { DAGData, DAGNode, DAGEdge, QueryPlanTransformer } from './types';
import type { QueryBundle } from '~quent/types/QueryBundle';
import { Operator } from '~quent/types/Operator';
import { Port } from '~quent/types/Port';

export class QueryBundleTransformer implements QueryPlanTransformer<QueryBundle> {
  engineName = 'quent';

  validate(bundle: QueryBundle): bundle is QueryBundle {
    return (
      typeof bundle === 'object' &&
      bundle !== null &&
      Object.keys(bundle?.entities?.plans).length > 0
    );
  }

  getNodeEntity(bundle: QueryBundle, id: string): DAGNode | undefined {
    // Find associated port
    if (bundle?.entities?.ports?.[id]) {
      const port: Port = bundle?.entities?.ports?.[id];
      const operator: Operator | undefined = bundle?.entities?.operators?.[port.parent_operator_id];
      if (operator) {
        return {
          id: operator.id,
          label: operator.name ?? 'Node',
          type: operator.name?.toLowerCase() ?? 'operator',
          metadata: {
            rawNode: operator,
          },
        };
      }
    }

    return undefined;
  }

  transform(bundle: QueryBundle): DAGData {
    const nodeMap = new Map<string, DAGNode>();
    const edges: DAGEdge[] = [];

    // TODO: Once we have a way to display plan trees, we'll take a plan ID to render
    // for now just render the phyiscal plan
    const plans = Object.values(bundle?.entities?.plans);
    const planTree = plans?.find(plan => plan?.name === 'physical') || plans?.[0];

    if (!planTree) {
      throw new Error('No physical plan found');
    }

    planTree.edges.forEach(edge => {
      const sourceNode = this.getNodeEntity(bundle, edge.source);
      const targetNode = this.getNodeEntity(bundle, edge.target);
      if (sourceNode && targetNode) {
        // Only add each node once (deduplicate by ID)
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

    return { nodes: Array.from(nodeMap.values()), edges };
  }
}

export const queryBundleTransformer = new QueryBundleTransformer();
