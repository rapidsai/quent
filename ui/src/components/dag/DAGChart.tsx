// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import ELK from 'elkjs';
import { useCallback, useEffect, useLayoutEffect, useRef, MouseEvent, type RefObject } from 'react';
import {
  Background,
  EdgeLabelRenderer,
  MiniMap,
  ReactFlow,
  ReactFlowProvider,
  useNodesState,
  useEdgesState,
  useReactFlow,
  getSmoothStepPath,
  type Node,
  type Edge,
  type EdgeProps,
  type OnMoveStart,
} from '@xyflow/react';
import '@xyflow/react/dist/style.css';
import { useSelectedNodeIds, useSetSelectedNodeIds, useSetSelectedOperatorLabel } from '@quent/hooks';
import type { DAGData } from '@/services/query-plan/types';
import {
  OPERATION_TYPE_COLORS,
  DEFAULT_OPERATION_COLOR,
} from '@/services/query-plan/operationTypes';
import { QueryPlanNode, type QueryPlanNodeData } from '../query-plan/QueryPlanNode';
import { DAGLegend } from './DAGLegend';
import {
  edgeWidthConfigAtom,
  edgeColoringAtom,
  edgeColorPaletteAtom,
  selectedEdgeWidthFieldAtom,
  selectedEdgeColorFieldAtom,
} from '@/atoms/dagControls';
import { useAtomValue } from 'jotai';
import { continuousColor } from '@/services/colors';
import { useTheme, THEME_DARK } from '@/contexts/ThemeContext';
import { inferFieldFormatter } from '@/services/query-plan/dagFieldProcessing';

const elk = new ELK();

// Edge geometry constants
const EDGE_STROKE_WIDTH_DEFAULT = 1.5;
const EDGE_STROKE_WIDTH_MIN = 2;
const EDGE_STROKE_WIDTH_RANGE = 10; // stroke = MIN + t * RANGE → [2, 12] px
const EDGE_DIMMED_OPACITY = 0.15;
const EDGE_TRANSITION_MS = 150;
const ARROW_WIDTH_MULTIPLIER = 1.5;
const ARROW_WIDTH_BASE = 8;
const ARROW_DEPTH_RATIO = 0.6;
const FALLBACK_NORMALIZED_T = 0.5; // used when min === max

// Layout constants
const NODE_LAYOUT_WIDTH = 200;
const NODE_LAYOUT_HEIGHT = 60;
const FIT_VIEW_PADDING = 0.1;
const FLOW_MIN_ZOOM = 0.1;
const FLOW_MAX_ZOOM = 2;

// MiniMap constants
const MINIMAP_SIZE = 125;
const MINIMAP_NODE_STROKE_WIDTH = 3;

const VariableWidthEdge = ({
  id,
  source,
  target,
  sourceX,
  sourceY,
  targetX,
  targetY,
  sourcePosition,
  targetPosition,
}: EdgeProps) => {
  const edgeWidthConfig = useAtomValue(edgeWidthConfigAtom);
  const edgeColoring = useAtomValue(edgeColoringAtom);
  const edgePalette = useAtomValue(edgeColorPaletteAtom);
  const selectedNodeIds = useSelectedNodeIds();
  const edgeWidthField = useAtomValue(selectedEdgeWidthFieldAtom);
  const edgeColorField = useAtomValue(selectedEdgeColorFieldAtom);
  const { theme } = useTheme();
  const isDarkMode = theme === THEME_DARK;

  let strokeWidth = EDGE_STROKE_WIDTH_DEFAULT;
  if (edgeWidthConfig) {
    const v = edgeWidthConfig.values.get(id);
    if (v !== undefined) {
      const t =
        edgeWidthConfig.max > edgeWidthConfig.min
          ? (v - edgeWidthConfig.min) / (edgeWidthConfig.max - edgeWidthConfig.min)
          : FALLBACK_NORMALIZED_T;
      strokeWidth = EDGE_STROKE_WIDTH_MIN + t * EDGE_STROKE_WIDTH_RANGE;
    }
  }

  let edgeColor: string | undefined;
  let edgeDimmed = false;
  if (edgeColoring) {
    if (edgeColoring.type === 'continuous') {
      const v = edgeColoring.values.get(id);
      if (v === undefined) {
        edgeDimmed = true;
      } else {
        const t =
          edgeColoring.max > edgeColoring.min
            ? (v - edgeColoring.min) / (edgeColoring.max - edgeColoring.min)
            : FALLBACK_NORMALIZED_T;
        edgeColor = continuousColor(t, edgePalette, isDarkMode);
      }
    } else {
      const color = edgeColoring.colorMap.get(id);
      if (!color) edgeDimmed = true;
      else edgeColor = color;
    }
  }

  const hasSelection = selectedNodeIds.size > 0;
  const isEdgeDimmed =
    edgeDimmed || (hasSelection && !selectedNodeIds.has(source) && !selectedNodeIds.has(target));

  let edgeLabelValue: string | undefined;
  if (edgeColoring) {
    if (edgeColoring.type === 'continuous') {
      const v = edgeColoring.values.get(id);
      if (v !== undefined) {
        edgeLabelValue = inferFieldFormatter(edgeColorField ?? '')(v);
      }
    } else {
      const v = edgeColoring.labelMap.get(id);
      if (v !== undefined) edgeLabelValue = v;
    }
  } else if (edgeWidthConfig) {
    const v = edgeWidthConfig.values.get(id);
    if (v !== undefined) {
      edgeLabelValue = inferFieldFormatter(edgeWidthField ?? '')(v);
    }
  }

  const arrowWidth = strokeWidth * ARROW_WIDTH_MULTIPLIER + ARROW_WIDTH_BASE;
  const arrowDepth = arrowWidth * ARROW_DEPTH_RATIO;
  const markerId = `arrow-${id}`;
  const [edgePath, labelX, labelY] = getSmoothStepPath({
    sourceX,
    sourceY,
    targetX,
    targetY: targetY - arrowDepth,
    sourcePosition,
    targetPosition,
  });

  return (
    <>
      <defs>
        <marker
          id={markerId}
          markerWidth={arrowDepth}
          markerHeight={arrowWidth}
          refX={0}
          refY={arrowWidth / 2}
          orient="auto"
          markerUnits="userSpaceOnUse"
        >
          <path
            d={`M0,0 L0,${arrowWidth} L${arrowDepth},${arrowWidth / 2} z`}
            fill={edgeColor ?? 'currentColor'}
            opacity={isEdgeDimmed ? EDGE_DIMMED_OPACITY : 1}
          />
        </marker>
      </defs>
      <path
        id={id}
        className="react-flow__edge-path"
        d={edgePath}
        markerEnd={`url(#${markerId})`}
        style={{
          stroke: edgeColor ?? 'currentColor',
          strokeWidth,
          fill: 'none',
          opacity: isEdgeDimmed ? EDGE_DIMMED_OPACITY : 1,
          transition: `opacity ${EDGE_TRANSITION_MS}ms, stroke ${EDGE_TRANSITION_MS}ms`,
        }}
      />
      {edgeLabelValue && (
        <EdgeLabelRenderer>
          <div
            style={{
              position: 'absolute',
              transform: `translate(-50%, -50%) translate(${labelX}px,${labelY}px)`,
              pointerEvents: 'none',
              opacity: isEdgeDimmed ? EDGE_DIMMED_OPACITY : 1,
              transition: `opacity ${EDGE_TRANSITION_MS}ms`,
            }}
            className="text-[10px] font-medium px-1 py-0.5 rounded bg-background/80 text-muted-foreground border border-border/50"
          >
            {edgeLabelValue}
          </div>
        </EdgeLabelRenderer>
      )}
    </>
  );
};

const ELK_LAYER_SPACING = '100';
const ELK_NODE_SPACING = '50';

const elkOptions = {
  'elk.algorithm': 'layered',
  'elk.direction': 'DOWN',
  'elk.layered.spacing.nodeNodeBetweenLayers': ELK_LAYER_SPACING,
  'elk.spacing.nodeNode': ELK_NODE_SPACING,
};

const edgeTypes = {
  smoothstep: VariableWidthEdge,
  default: VariableWidthEdge,
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
      width: NODE_LAYOUT_WIDTH,
      height: NODE_LAYOUT_HEIGHT,
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
  const setSelectedNodeIds = useSetSelectedNodeIds();
  const setSelectedOperatorLabel = useSetSelectedOperatorLabel();
  const selectedNodeIds = useSelectedNodeIds();
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
          minWidth: NODE_LAYOUT_WIDTH,
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
        fitView({ padding: FIT_VIEW_PADDING, minZoom: FLOW_MIN_ZOOM });
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
      setTimeout(() => fitView({ padding: FIT_VIEW_PADDING, minZoom: FLOW_MIN_ZOOM }), 0);
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
      edgeTypes={edgeTypes}
      fitView
      minZoom={FLOW_MIN_ZOOM}
      maxZoom={FLOW_MAX_ZOOM}
      defaultEdgeOptions={{ type: 'smoothstep' }}
    >
      <Background />
      <DAGLegend />
      <MiniMap
        pannable
        zoomable
        nodeStrokeWidth={MINIMAP_NODE_STROKE_WIDTH}
        style={{ width: MINIMAP_SIZE, height: MINIMAP_SIZE, background: 'hsl(var(--card))' }}
        maskColor="hsl(var(--muted) / 0.7)"
        nodeColor={(node: Node<QueryPlanNodeData>) =>
          OPERATION_TYPE_COLORS[(node.data as QueryPlanNodeData).operationType] ??
          DEFAULT_OPERATION_COLOR
        }
      />
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
