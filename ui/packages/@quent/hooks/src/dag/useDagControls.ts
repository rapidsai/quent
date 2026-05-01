// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useMemo, useEffect } from 'react';
import { useAtomValue, useSetAtom } from 'jotai';
import type { DAGNode, DAGEdge, NodeColoring, EdgeWidthConfig, EdgeColoring } from '@quent/utils';
import {
  selectedColorField,
  nodeColoringAtom,
  selectedEdgeWidthFieldAtom,
  edgeWidthConfigAtom,
  selectedEdgeColorFieldAtom,
  edgeColoringAtom,
} from '../atoms/dagControls';

// Computation functions injected to avoid circular dep with @quent/components
type ComputeNodeColoringFn = (nodes: DAGNode[], field: string | null) => NodeColoring;
type ComputeEdgeWidthConfigFn = (edges: DAGEdge[], field: string | null) => EdgeWidthConfig;
type ComputeEdgeColoringFn = (edges: DAGEdge[], field: string | null) => EdgeColoring;
type ParseCustomStatisticsFn = (rawNode: unknown) => Array<{ key: string }>;

export function useDagNodeColoring(nodes: DAGNode[], computeNodeColoring: ComputeNodeColoringFn) {
  const selectedField = useAtomValue(selectedColorField);
  const setNodeColoring = useSetAtom(nodeColoringAtom);
  const coloring = useMemo(
    () => computeNodeColoring(nodes, selectedField),
    [nodes, selectedField, computeNodeColoring]
  );
  useEffect(() => {
    setNodeColoring(coloring);
  }, [coloring, setNodeColoring]);
}

export function useDagEdgeWidthConfig(
  edges: DAGEdge[],
  computeEdgeWidthConfig: ComputeEdgeWidthConfigFn
) {
  const selectedEdgeWidthField = useAtomValue(selectedEdgeWidthFieldAtom);
  const setEdgeWidthConfig = useSetAtom(edgeWidthConfigAtom);
  const config = useMemo(
    () => computeEdgeWidthConfig(edges, selectedEdgeWidthField),
    [edges, selectedEdgeWidthField, computeEdgeWidthConfig]
  );
  useEffect(() => {
    setEdgeWidthConfig(config);
  }, [config, setEdgeWidthConfig]);
}

export function useDagEdgeColoring(edges: DAGEdge[], computeEdgeColoring: ComputeEdgeColoringFn) {
  const selectedField = useAtomValue(selectedEdgeColorFieldAtom);
  const setEdgeColoring = useSetAtom(edgeColoringAtom);
  const coloring = useMemo(
    () => computeEdgeColoring(edges, selectedField),
    [edges, selectedField, computeEdgeColoring]
  );
  useEffect(() => {
    setEdgeColoring(coloring);
  }, [coloring, setEdgeColoring]);
}

export function useOperatorStatFields(
  nodes: DAGNode[],
  parseCustomStatistics: ParseCustomStatisticsFn
): string[] {
  return useMemo(
    () => [
      ...new Set(nodes.flatMap(n => parseCustomStatistics(n.metadata?.rawNode).map(s => s.key))),
    ],
    [nodes, parseCustomStatistics]
  );
}

export function usePortStatFields(edges: DAGEdge[]): string[] {
  return useMemo(
    () => [...new Set(edges.flatMap(e => (e.portStats ?? []).map(s => s.key)))],
    [edges]
  );
}
