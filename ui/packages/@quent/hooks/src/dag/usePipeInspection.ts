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
  clearInspectedPipesAtom,
  inspectedPipeNodesAtom,
  inspectedPipeRefsAtom,
  inspectedPipesAtom,
  inspectedPipeSnapshotsAtom,
  type InspectedPipeSnapshot,
  toggleInspectedPipeAtom,
} from '../atoms/pipeInspection';

export const useInspectedPipes = () => useAtomValue(inspectedPipesAtom);
export const useAddInspectedPipe = () => useSetAtom(addInspectedPipeAtom);
export const useToggleInspectedPipe = () => useSetAtom(toggleInspectedPipeAtom);
export const useClearInspectedPipes = () => useSetAtom(clearInspectedPipesAtom);

export const useInspectedPipeNodes = () => useAtomValue(inspectedPipeNodesAtom);
export const useDisplayedPipes = () => useAtomValue(displayedPipesAtom);
export const useDisplayedDagNodes = () => useAtomValue(displayedDagNodesAtom);
export const useHoveredPipeId = () => useAtomValue(hoveredPipeIdAtom);
export const useSetHoveredPipeId = () => useSetAtom(hoveredPipeIdAtom);
const EMPTY_NODES: readonly DAGNode[] = [];

function sameSnapshots(a: readonly InspectedPipeSnapshot[], b: readonly InspectedPipeSnapshot[]) {
  return a.length === b.length && a.every((snapshot, index) => snapshot === b[index]);
}

/**
 * Publishes the displayed plan's pipes and nodes. Inspected pipes outlive a plan switch through
 * their snapshots; a ref that is neither on the plan nor snapshotted can never resolve and is dropped.
 */
export function useSyncDisplayedPipes(
  edges: readonly DAGEdge[],
  nodes: readonly DAGNode[] = EMPTY_NODES
) {
  const store = useStore();
  const refs = useAtomValue(inspectedPipeRefsAtom);
  useEffect(() => {
    store.set(displayedPipesAtom, edges);
    store.set(displayedDagNodesAtom, nodes);
    store.set(hoveredPipeIdAtom, null);
    const current = store.get(inspectedPipeRefsAtom);
    const snapshots = store.get(inspectedPipeSnapshotsAtom);
    const kept = current.filter(
      ref =>
        edges.some(edge => isSamePipe(edge, ref)) ||
        snapshots.some(snapshot => isSamePipe(snapshot.edge, ref))
    );
    if (kept.length !== current.length) {
      store.set(inspectedPipeRefsAtom, kept);
    }
    return () => {
      store.set(displayedPipesAtom, []);
      store.set(displayedDagNodesAtom, []);
      store.set(hoveredPipeIdAtom, null);
    };
  }, [edges, nodes, store]);
  // Keep one snapshot per inspected ref, refreshed while its plan is displayed.
  useEffect(() => {
    const previous = store.get(inspectedPipeSnapshotsAtom);
    const next = refs.flatMap((ref): InspectedPipeSnapshot[] => {
      const existing = previous.find(snapshot => isSamePipe(snapshot.edge, ref));
      const edge = edges.find(candidate => isSamePipe(candidate, ref));
      if (!edge) {
        return existing ? [existing] : [];
      }
      const sourceNode = nodes.find(node => node.id === edge.source);
      const targetNode = nodes.find(node => node.id === edge.target);
      return existing &&
        existing.edge === edge &&
        existing.sourceNode === sourceNode &&
        existing.targetNode === targetNode
        ? [existing]
        : [{ edge, sourceNode, targetNode }];
    });
    if (!sameSnapshots(previous, next)) {
      store.set(inspectedPipeSnapshotsAtom, next);
    }
  }, [refs, edges, nodes, store]);
}
