// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useEffect } from 'react';
import { useAtomValue, useSetAtom, useStore } from 'jotai';
import type { DAGEdge, DAGNode } from '@quent/utils';
import {
  displayedPipesAtom,
  displayedDagNodesAtom,
  hoveredPipeIdAtom,
  inspectedPipeAtom,
  inspectedPipeRefAtom,
} from '../atoms/pipeInspection';

export const useInspectedPipe = () => useAtomValue(inspectedPipeAtom);
export const useSetInspectedPipe = () => useSetAtom(inspectedPipeRefAtom);

export const useDisplayedPipes = () => useAtomValue(displayedPipesAtom);
export const useDisplayedDagNodes = () => useAtomValue(displayedDagNodesAtom);
export const useHoveredPipeId = () => useAtomValue(hoveredPipeIdAtom);
export const useSetHoveredPipeId = () => useSetAtom(hoveredPipeIdAtom);
const EMPTY_NODES: readonly DAGNode[] = [];
export function useSyncDisplayedPipes(
  edges: readonly DAGEdge[],
  nodes: readonly DAGNode[] = EMPTY_NODES
) {
  const store = useStore();
  useEffect(() => {
    store.set(displayedPipesAtom, edges);
    store.set(displayedDagNodesAtom, nodes);
    store.set(hoveredPipeIdAtom, null);
    const ref = store.get(inspectedPipeRefAtom);
    if (
      ref &&
      !edges.some(
        edge => edge.sourcePortId === ref.sourcePortId && edge.targetPortId === ref.targetPortId
      )
    ) {
      store.set(inspectedPipeRefAtom, null);
    }
    return () => {
      store.set(displayedPipesAtom, []);
      store.set(displayedDagNodesAtom, []);
      store.set(hoveredPipeIdAtom, null);
    };
  }, [edges, nodes, store]);
}
