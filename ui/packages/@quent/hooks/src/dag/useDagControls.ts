// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useMemo, useEffect } from 'react';
import { useAtomValue, useSetAtom } from 'jotai';
import type {
  DAGNode,
  DAGEdge,
  NodeColoring,
  EdgeWidthConfig,
  EdgeColoring,
  PaletteTheme,
} from '@quent/utils';
import {
  selectedColorField,
  nodeColoringAtom,
  selectedEdgeWidthFieldAtom,
  edgeWidthConfigAtom,
  selectedEdgeColorFieldAtom,
  edgeColoringAtom,
} from '../atoms/dagControls';

// Computation functions injected to avoid circular dep with @quent/components
type ComputeNodeColoringFn = (
  nodes: DAGNode[],
  field: string | null,
  theme: PaletteTheme
) => NodeColoring;
type ComputeEdgeWidthConfigFn = (edges: DAGEdge[], field: string | null) => EdgeWidthConfig;
type ComputeEdgeColoringFn = (
  edges: DAGEdge[],
  field: string | null,
  theme: PaletteTheme
) => EdgeColoring;
type ParseCustomStatisticsFn = (rawNode: unknown) => Array<{ key: string; value?: unknown }>;

export function useDagNodeColoring(
  nodes: DAGNode[],
  computeNodeColoring: ComputeNodeColoringFn,
  isDark: boolean
) {
  const selectedField = useAtomValue(selectedColorField);
  const setNodeColoring = useSetAtom(nodeColoringAtom);
  const paletteTheme: PaletteTheme = isDark ? 'dark' : 'light';
  const coloring = useMemo(
    () => computeNodeColoring(nodes, selectedField, paletteTheme),
    [nodes, selectedField, paletteTheme, computeNodeColoring]
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

export function useDagEdgeColoring(
  edges: DAGEdge[],
  computeEdgeColoring: ComputeEdgeColoringFn,
  isDark: boolean
) {
  const selectedField = useAtomValue(selectedEdgeColorFieldAtom);
  const setEdgeColoring = useSetAtom(edgeColoringAtom);
  const paletteTheme: PaletteTheme = isDark ? 'dark' : 'light';
  const coloring = useMemo(
    () => computeEdgeColoring(edges, selectedField, paletteTheme),
    [edges, selectedField, paletteTheme, computeEdgeColoring]
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
      ...new Set(
        nodes.flatMap(n => {
          const own = parseCustomStatistics(n.metadata?.rawNode).map(s => s.key);
          // Nodes that group operators (logical-plan nodes) get a value by
          // aggregating their related operators, which only works for numbers,
          // so only numeric related stats are offered.
          const related = Array.isArray(n.metadata?.relatedOperators)
            ? (n.metadata.relatedOperators as unknown[])
            : [];
          const aggregatable = related.flatMap(raw =>
            parseCustomStatistics(raw)
              .filter(s => typeof s.value === 'number' || typeof s.value === 'bigint')
              .map(s => s.key)
          );
          return [...own, ...aggregatable];
        })
      ),
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
