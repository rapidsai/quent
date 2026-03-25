// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import ELK from 'elkjs';
import { useCallback, useEffect, useLayoutEffect, useRef, MouseEvent, type RefObject } from 'react';
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
  type OnMoveStart,
} from '@xyflow/react';
import '@xyflow/react/dist/style.css';
import { useAtomValue, useSetAtom } from 'jotai';
import type { DAGData } from '@/services/query-plan/types';
import { QueryPlanNode, type QueryPlanNodeData } from '../query-plan/QueryPlanNode';
import { selectedNodeIdsAtom, selectedOperatorLabelAtom } from '@/atoms/dag';

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
  containerRef,
}: {
  data: DAGData;
  containerRef: RefObject<HTMLDivElement | null>;
}) => {
  const [nodes, setNodes, onNodesChange] = useNodesState<Node<QueryPlanNodeData>>([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState<Edge>([]);
  const { fitView } = useReactFlow();
  const setSelectedNodeIds = useSetAtom(selectedNodeIdsAtom);
  const setSelectedOperatorLabel = useSetAtom(selectedOperatorLabelAtom);
  const selectedNodeIds = useAtomValue(selectedNodeIdsAtom);
  const hasUserInteracted = useRef(false);

  const handleMoveStart = useCallback<OnMoveStart>(event => {
    if (event !== null) {
      hasUserInteracted.current = true;
    }
  }, []);

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
          nodeId: node.id,
          label: node.label,
          operationType: node.type,
          metadata: node.metadata as QueryPlanNodeData['metadata'],
          hasIncoming: nodesWithIncoming.has(node.id),
          hasOutgoing: nodesWithOutgoing.has(node.id),
        },
        style: {
          width: 'auto',
          minWidth: 200,
          background: 'transparent',
          boxShadow: 'none',
          border: 0,
          padding: 0,
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
      if (selectedNodeIds.has(node.id)) {
        setSelectedNodeIds(new Set());
        setSelectedOperatorLabel(null);
      } else {
        setSelectedNodeIds(new Set([node.id]));
        setSelectedOperatorLabel(node.data.label);
      }
    },
    [selectedNodeIds, setSelectedNodeIds, setSelectedOperatorLabel]
  );

  // Re-fit view when the react-flow container is resized, but only if the user
  // hasn't interacted with the chart (to maintain any focus states applied)
  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;
    const observer = new ResizeObserver(() => {
      if (nodes.length > 0 && !hasUserInteracted.current) {
        fitView({ padding: 0.1, minZoom: 0.1 });
      }
    });
    observer.observe(container);
    return () => observer.disconnect();
  }, [containerRef, fitView, nodes.length]);

  // Calculate and apply layout
  useLayoutEffect(() => {
    hasUserInteracted.current = false;

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
      onMoveStart={handleMoveStart}
      proOptions={{ hideAttribution: true }}
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

export const DAGChart = ({ data, height = '100%' }: DAGProps) => {
  const containerRef = useRef<HTMLDivElement>(null);
  return (
    <div ref={containerRef} style={{ width: '100%', height }}>
      <ReactFlowProvider>
        <FlowLayout data={data} containerRef={containerRef} />
      </ReactFlowProvider>
    </div>
  );
};
