import ELK from 'elkjs';
import { useCallback, useLayoutEffect, MouseEvent } from 'react';
import {
  Background,
  ReactFlow,
  ReactFlowProvider,
  useNodesState,
  useEdgesState,
  useReactFlow,
  MarkerType,
  type Node,
  type Edge,
} from '@xyflow/react';
import '@xyflow/react/dist/style.css';
import type { DAGData } from '@/services/query-plan/types';
import { QueryPlanNode, type QueryPlanNodeData } from '../query-plan/QueryPlanNode';
import { useNavigate } from '@tanstack/react-router';

const elk = new ELK();

const elkOptions = {
  'elk.algorithm': 'layered',
  'elk.direction': 'DOWN',
  'elk.layered.spacing.nodeNodeBetweenLayers': '50',
  'elk.spacing.nodeNode': '50',
};

// Custom node types for different operations
const nodeTypes = {
  source: QueryPlanNode,
  scan: QueryPlanNode,
  join: QueryPlanNode,
  joinlocal: QueryPlanNode,
  joinpartition: QueryPlanNode,
  filesystemscan: QueryPlanNode,
  aggregate: QueryPlanNode,
  exchange: QueryPlanNode,
  output: QueryPlanNode,
  stage: QueryPlanNode,
  local: QueryPlanNode,
  project: QueryPlanNode,
  filter: QueryPlanNode,
  sort: QueryPlanNode,
  limit: QueryPlanNode,
  union: QueryPlanNode,
  other: QueryPlanNode,
  default: QueryPlanNode,
};

interface DAGProps {
  data: DAGData;
  queryId: string;
  engineId: string;
  height?: string;
}

async function calculateLayout(
  nodes: Node<QueryPlanNodeData>[],
  edges: Edge[]
): Promise<{ nodes: Node<QueryPlanNodeData>[]; edges: Edge[] }> {
  const graph = {
    id: 'root',
    layoutOptions: elkOptions,
    children: nodes.map(node => ({
      id: node.id,
      width: 200,
      height: 60,
    })),
    edges: edges.map(edge => ({
      id: edge.id,
      sources: [edge.source],
      targets: [edge.target],
    })),
  };

  const layout = await elk.layout(graph);

  return {
    nodes:
      layout.children?.map((child, i) => ({
        ...nodes[i],
        position: { x: child.x ?? 0, y: child.y ?? 0 },
      })) ?? [],
    edges: edges,
  };
}

const FlowLayout = ({
  data,
  queryId,
  engineId,
}: {
  data: DAGData;
  queryId: string;
  engineId: string;
}) => {
  const [nodes, setNodes, onNodesChange] = useNodesState<Node<QueryPlanNodeData>>([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState<Edge>([]);
  const { fitView } = useReactFlow();
  const navigate = useNavigate();

  // Convert DAGData to ReactFlow format
  const convertToReactFlow = useCallback(() => {
    // Determine which nodes have incoming/outgoing edges
    const nodesWithIncoming = new Set(data.edges.map(e => e.target));
    const nodesWithOutgoing = new Set(data.edges.map(e => e.source));

    const flowNodes: Node<QueryPlanNodeData>[] = data.nodes.map(node => {
      return {
        id: node.id,
        type: node.type,
        data: {
          label: node.label,
          operationType: node.type,
          metadata: node.metadata,
          hasIncoming: nodesWithIncoming.has(node.id),
          hasOutgoing: nodesWithOutgoing.has(node.id),
        },
        position: { x: 0, y: 0 }, // Will be set by layout
      };
    });

    const flowEdges: Edge[] = data.edges.map(edge => ({
      id: edge.id,
      source: edge.source,
      target: edge.target,
      type: 'smoothstep',
      markerEnd: {
        type: MarkerType.ArrowClosed,
        width: 20,
        height: 20,
      },
    }));

    return { flowNodes, flowEdges };
  }, [data]);

  const handleNodeClick = useCallback(
    (_event: MouseEvent, node: Node<QueryPlanNodeData>): void => {
      navigate({
        to: '/profile/engine/$engineId/query/$queryId/node/$nodeId',
        params: { engineId, queryId, nodeId: node.id },
      });
    },
    [navigate, engineId, queryId]
  );

  // Calculate and apply layout
  useLayoutEffect(() => {
    const applyLayout = async () => {
      const { flowNodes, flowEdges } = convertToReactFlow();
      const layoutResult = await calculateLayout(flowNodes, flowEdges);

      setNodes(layoutResult.nodes);
      setEdges(layoutResult.edges);

      // Fit view after layout
      setTimeout(() => fitView({ padding: 0.1, minZoom: 0.1 }), 0);
    };

    applyLayout();
  }, [data, convertToReactFlow, fitView, setNodes, setEdges]);

  return (
    <ReactFlow
      nodes={nodes}
      edges={edges}
      onNodesChange={onNodesChange}
      onEdgesChange={onEdgesChange}
      onNodeClick={handleNodeClick}
      nodeTypes={nodeTypes}
      fitView
      minZoom={0.1}
      maxZoom={2}
      defaultEdgeOptions={{
        type: 'smoothstep',
        markerEnd: { type: MarkerType.ArrowClosed, width: 20, height: 20 },
      }}
    >
      <Background />
    </ReactFlow>
  );
};

export const DAGChart = ({ data, queryId, engineId, height = '100%' }: DAGProps) => {
  return (
    <div style={{ width: '100%', height }}>
      <ReactFlowProvider>
        <FlowLayout data={data} queryId={queryId} engineId={engineId} />
      </ReactFlowProvider>
    </div>
  );
};
