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
} from '@/atoms/dag';
import {
  computeNodeColoring,
  computeEdgeWidthConfig,
  computeEdgeColoring,
} from '@/services/query-plan/dagFieldProcessing';
import { parseCustomStatistics } from '@/lib/queryBundle.utils';
import { THEME_DARK, useTheme } from '@/contexts/ThemeContext';

export function useDagNodeColoring(nodes: DAGNode[]) {
  const selectedField = useAtomValue(selectedColorField);
  const setNodeColoring = useSetAtom(nodeColoringAtom);
  const { theme } = useTheme();
  const paletteTheme = theme === THEME_DARK ? 'dark' : 'light';
  const coloring = useMemo(
    () => computeNodeColoring(nodes, selectedField, paletteTheme),
    [nodes, selectedField, paletteTheme]
  );
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
  const { theme } = useTheme();
  const paletteTheme = theme === THEME_DARK ? 'dark' : 'light';
  const coloring = useMemo(
    () => computeEdgeColoring(edges, selectedField, paletteTheme),
    [edges, selectedField, paletteTheme]
  );
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
