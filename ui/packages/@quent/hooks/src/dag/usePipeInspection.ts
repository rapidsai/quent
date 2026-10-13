// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useEffect } from 'react';
import { useAtomValue, useSetAtom, useStore } from 'jotai';
import { isSamePipe, type DAGEdge, type DAGNode } from '@quent/utils';
import {
  displayedPipesAtom,
  displayedDagNodesAtom,
  hoveredPipeIdAtom,
  addInspectedPipeAtom,
  inspectedPipeRefsAtom,
  inspectedPipesAtom,
  toggleInspectedPipeAtom,
} from '../atoms/pipeInspection';

export const useInspectedPipes = () => useAtomValue(inspectedPipesAtom);
export const useAddInspectedPipe = () => useSetAtom(addInspectedPipeAtom);
export const useToggleInspectedPipe = () => useSetAtom(toggleInspectedPipeAtom);

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
    const refs = store.get(inspectedPipeRefsAtom);
    const kept = refs.filter(ref => edges.some(edge => isSamePipe(edge, ref)));
    if (kept.length !== refs.length) {
      store.set(inspectedPipeRefsAtom, kept);
    }
    return () => {
      store.set(displayedPipesAtom, []);
      store.set(displayedDagNodesAtom, []);
      store.set(hoveredPipeIdAtom, null);
    };
  }, [edges, nodes, store]);
}
