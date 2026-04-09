// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useMemo, useEffect } from 'react';
import { useAtomValue, useSetAtom } from 'jotai';
import type { DAGNode, DAGEdge } from '@/services/query-plan/types';
import {
  selectedColorField,
  nodeColoringAtom,
  selectedEdgeWidthFieldAtom,
  edgeWidthConfigAtom,
  selectedEdgeColorFieldAtom,
  edgeColoringAtom,
} from '@/atoms/dagControls';
import {
  computeNodeColoring,
  computeEdgeWidthConfig,
  computeEdgeColoring,
} from '@/services/query-plan/dagFieldProcessing';
import { parseCustomStatistics } from '@/lib/queryBundle.utils';

export function useDagNodeColoring(nodes: DAGNode[]) {
  const selectedField = useAtomValue(selectedColorField);
  const setNodeColoring = useSetAtom(nodeColoringAtom);
  const coloring = useMemo(() => computeNodeColoring(nodes, selectedField), [nodes, selectedField]);
  useEffect(() => {
    setNodeColoring(coloring);
  }, [coloring, setNodeColoring]);
}

export function useDagEdgeWidthConfig(edges: DAGEdge[]) {
  const selectedEdgeWidthField = useAtomValue(selectedEdgeWidthFieldAtom);
  const setEdgeWidthConfig = useSetAtom(edgeWidthConfigAtom);
  const config = useMemo(
    () => computeEdgeWidthConfig(edges, selectedEdgeWidthField),
    [edges, selectedEdgeWidthField]
  );
  useEffect(() => {
    setEdgeWidthConfig(config);
  }, [config, setEdgeWidthConfig]);
}

export function useDagEdgeColoring(edges: DAGEdge[]) {
  const selectedField = useAtomValue(selectedEdgeColorFieldAtom);
  const setEdgeColoring = useSetAtom(edgeColoringAtom);
  const coloring = useMemo(() => computeEdgeColoring(edges, selectedField), [edges, selectedField]);
  useEffect(() => {
    setEdgeColoring(coloring);
  }, [coloring, setEdgeColoring]);
}

export function useOperatorStatFields(nodes: DAGNode[]): string[] {
  return useMemo(
    () => [
      ...new Set(nodes.flatMap(n => parseCustomStatistics(n.metadata?.rawNode).map(s => s.key))),
    ],
    [nodes]
  );
}

export function usePortStatFields(edges: DAGEdge[]): string[] {
  return useMemo(
    () => [...new Set(edges.flatMap(e => (e.portStats ?? []).map(s => s.key)))],
    [edges]
  );
}
