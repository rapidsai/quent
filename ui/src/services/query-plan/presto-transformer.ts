import type {
  PrestoQueryPlan,
  PrestoQueryPlanNode,
  DAGData,
  DAGNode,
  DAGEdge,
  QueryPlanTransformer,
} from './types';

export class PrestoTransformer implements QueryPlanTransformer<PrestoQueryPlan> {
  engineName = 'presto';

  validate(plan: unknown): plan is PrestoQueryPlan {
    return (
      typeof plan === 'object' &&
      plan !== null &&
      'id' in plan &&
      'name' in plan &&
      'children' in plan
    );
  }

  transform(plan: PrestoQueryPlan): DAGData {
    const nodes: DAGNode[] = [];
    const edges: DAGEdge[] = [];

    // Recursive traversal with parent tracking
    const traverse = (node: PrestoQueryPlanNode, parentId: string | null = null): void => {
      const label = this.createLabel(node.name, node.identifier);
      // const label = node.name;

      nodes.push({
        id: node.id,
        label,
        type: this.categorizeOperation(node.name),
        metadata: {
          details: node.details,
          estimates: node.estimates,
        },
      });

      // Create edge from parent to this node
      if (parentId) {
        edges.push({
          id: `e-${parentId}-${node.id}`,
          source: parentId,
          target: node.id,
          type: 'smoothstep',
        });
      }

      node.children.forEach(child => traverse(child, node.id));
    };

    // Start traversal from root
    traverse(plan);

    return { nodes, edges };
  }

  private createLabel(name: string, identifier: string): string {
    if (identifier && identifier.trim()) {
      return `${name} ${identifier}`;
    }
    return name;
  }

  private categorizeOperation(name: string): string {
    // Categorize for visual styling
    if (name.startsWith('Scan')) return 'source';
    if (name.includes('Join')) return 'join';
    if (name.includes('Aggregate')) return 'aggregate';
    if (name.includes('Exchange')) return 'exchange';
    if (name === 'Output') return 'output';
    return 'default';
  }
}

export const prestoTransformer = new PrestoTransformer();
