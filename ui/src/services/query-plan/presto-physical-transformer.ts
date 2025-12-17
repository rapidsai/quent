import type {
  PrestoPhysicalPlan,
  PrestoPhysicalPlanNode,
  DAGData,
  DAGNode,
  DAGEdge,
  QueryPlanTransformer,
} from './types';

export class PrestoPhysicalTransformer implements QueryPlanTransformer<PrestoPhysicalPlan> {
  engineName = 'presto-physical';

  validate(plan: unknown): plan is PrestoPhysicalPlan {
    if (typeof plan !== 'object' || plan === null) return false;

    // Check if it has numeric stage keys with plan objects
    const keys = Object.keys(plan);
    if (keys.length === 0) return false;

    // Check first stage has expected structure
    const firstStage = (plan as Record<string, unknown>)[keys[0]];
    return typeof firstStage === 'object' && firstStage !== null && 'plan' in firstStage;
  }

  transform(plan: PrestoPhysicalPlan): DAGData {
    const nodes: DAGNode[] = [];
    const edges: DAGEdge[] = [];
    const processedNodes = new Set<string>();

    // Process each stage
    Object.entries(plan).forEach(([stageId, stage]) => {
      // Create stage node
      const stageNodeId = `stage-${stageId}`;
      nodes.push({
        id: stageNodeId,
        label: `Stage ${stageId}`,
        type: 'stage',
        metadata: {
          stageId,
          rawNode: stage.plan,
        },
      });

      // Traverse the plan tree within this stage
      this.traverseStageTree(stage.plan, stageNodeId, nodes, edges, processedNodes);

      // Connect to remote sources (other stages)
      if (stage.plan.remoteSources && stage.plan.remoteSources.length > 0) {
        stage.plan.remoteSources.forEach(remoteStageId => {
          const remoteStageNodeId = `stage-${remoteStageId}`;
          edges.push({
            id: `e-${remoteStageNodeId}-${stageNodeId}`,
            source: remoteStageNodeId,
            target: stageNodeId,
            type: 'smoothstep',
          });
        });
      }
    });

    return { nodes, edges };
  }

  private traverseStageTree(
    node: PrestoPhysicalPlanNode,
    parentId: string,
    nodes: DAGNode[],
    edges: DAGEdge[],
    processedNodes: Set<string>
  ): void {
    // Avoid duplicate nodes
    if (processedNodes.has(node.id)) {
      // Still create edge if needed
      edges.push({
        id: `e-${node.id}-${parentId}`,
        source: node.id,
        target: parentId,
        type: 'smoothstep',
      });
      return;
    }

    processedNodes.add(node.id);

    const label = node.name;
    const nodeType = this.categorizeOperation(node.name);

    nodes.push({
      id: node.id,
      label,
      type: nodeType,
      metadata: {
        details: node.details,
        estimates: node.estimates,
        identifier: node.identifier,
        rawNode: node,
      },
    });

    // Create edge from this node to parent
    edges.push({
      id: `e-${node.id}-${parentId}`,
      source: node.id,
      target: parentId,
      type: 'smoothstep',
    });

    node.children.forEach(child => {
      this.traverseStageTree(child, node.id, nodes, edges, processedNodes);
    });

    // Handle remote sources within the node
    if (node.remoteSources && node.remoteSources.length > 0) {
      node.remoteSources.forEach(remoteStageId => {
        const remoteStageNodeId = `stage-${remoteStageId}`;
        edges.push({
          id: `e-${remoteStageNodeId}-${node.id}`,
          source: remoteStageNodeId,
          target: node.id,
          type: 'smoothstep',
        });
      });
    }
  }

  private categorizeOperation(name: string): string {
    // Categorize stage type for visual styling
    if (name.startsWith('Scan')) return 'source';
    if (name.includes('Join')) return 'join';
    if (name.includes('Aggregate')) return 'aggregate';
    if (name.includes('Exchange') || name.includes('Merge')) return 'exchange';
    if (name.includes('RemoteSource')) return 'exchange';
    if (name === 'Output') return 'output';
    if (name.startsWith('Local')) return 'local';
    if (name === 'Project') return 'project';
    return 'other';
  }
}

export const prestoPhysicalTransformer = new PrestoPhysicalTransformer();
