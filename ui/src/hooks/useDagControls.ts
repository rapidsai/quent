import { useMemo, useEffect } from 'react';
import { useAtomValue, useSetAtom } from 'jotai';
import type { DAGNode, DAGEdge } from '@/services/query-plan/types';
import {
  selectedColorField,
  nodeColoringAtom,
  selectedEdgeWidthFieldAtom,
  edgeWidthConfigAtom,
} from '@/atoms/dag';
import { computeNodeColoring, computeEdgeWidthConfig } from '@/services/query-plan/dagFieldProcessing';
import { parseCustomStatistics } from '@/lib/queryBundle.utils';

export function useDagNodeColoring(nodes: DAGNode[]) {
  const selectedField = useAtomValue(selectedColorField);
  const setNodeColoring = useSetAtom(nodeColoringAtom);
  const coloring = useMemo(() => computeNodeColoring(nodes, selectedField), [nodes, selectedField]);
  useEffect(() => { setNodeColoring(coloring); }, [coloring, setNodeColoring]);
}

export function useDagEdgeWidthConfig(edges: DAGEdge[]) {
  const selectedEdgeWidthField = useAtomValue(selectedEdgeWidthFieldAtom);
  const setEdgeWidthConfig = useSetAtom(edgeWidthConfigAtom);
  const config = useMemo(
    () => computeEdgeWidthConfig(edges, selectedEdgeWidthField),
    [edges, selectedEdgeWidthField]
  );
  useEffect(() => { setEdgeWidthConfig(config); }, [config, setEdgeWidthConfig]);
}

export function useOperatorStatFields(nodes: DAGNode[]): string[] {
  return useMemo(
    () => [...new Set(nodes.flatMap(n => parseCustomStatistics(n.metadata?.rawNode).map(s => s.key)))],
    [nodes]
  );
}

export function usePortStatFields(edges: DAGEdge[]): string[] {
  return useMemo(
    () => [...new Set(edges.flatMap(e => (e.portStats ?? []).map(s => s.key)))],
    [edges]
  );
}
