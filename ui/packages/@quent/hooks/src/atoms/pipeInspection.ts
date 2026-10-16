// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { atom } from 'jotai';
import { isSamePipe, type DAGEdge, type DAGNode, type PipeRef } from '@quent/utils';

/** Inspected pipes in selection order. Independent of the canonical operator filter. */
export const inspectedPipeRefsAtom = atom<readonly PipeRef[]>([]);
export const hoveredPipeIdAtom = atom<string | null>(null);
export const displayedDagNodesAtom = atom<readonly DAGNode[]>([]);
export const displayedPipesAtom = atom<readonly DAGEdge[]>([]);
/**
 * An inspected pipe and its endpoint operators as last seen on the plan that displayed it.
 * Lets pipes stay inspected, like operator selections, after the user switches plans.
 */
export interface InspectedPipeSnapshot {
  edge: DAGEdge;
  sourceNode?: DAGNode;
  targetNode?: DAGNode;
}
export const inspectedPipeSnapshotsAtom = atom<readonly InspectedPipeSnapshot[]>([]);
/** Adds the pipe to the inspected pipes. Does nothing when it is already inspected. */
export const addInspectedPipeAtom = atom(null, (get, set, ref: PipeRef) => {
  const refs = get(inspectedPipeRefsAtom);
  if (!refs.some(existing => isSamePipe(existing, ref))) {
    set(inspectedPipeRefsAtom, [...refs, ref]);
  }
});
/** Adds the pipe, or removes it when it is already inspected. */
export const toggleInspectedPipeAtom = atom(null, (get, set, ref: PipeRef) => {
  const refs = get(inspectedPipeRefsAtom);
  set(
    inspectedPipeRefsAtom,
    refs.some(existing => isSamePipe(existing, ref))
      ? refs.filter(existing => !isSamePipe(existing, ref))
      : [...refs, ref]
  );
});
/** Removes the pipe from the inspected pipes. Does nothing when it is not inspected. */
export const removeInspectedPipeAtom = atom(null, (get, set, ref: PipeRef) => {
  set(
    inspectedPipeRefsAtom,
    get(inspectedPipeRefsAtom).filter(existing => !isSamePipe(existing, ref))
  );
});
/** Deselects every inspected pipe. Operator selection is unaffected. */
export const clearInspectedPipesAtom = atom(null, (_get, set) => {
  set(inspectedPipeRefsAtom, []);
});
/**
 * Resolves each inspected ref, keeping selection order. A pipe on the displayed plan is read
 * live; one from another plan falls back to its snapshot.
 */
export const inspectedPipesAtom = atom(get => {
  const edges = get(displayedPipesAtom);
  const snapshots = get(inspectedPipeSnapshotsAtom);
  return get(inspectedPipeRefsAtom).flatMap(ref => {
    const edge =
      edges.find(candidate => isSamePipe(candidate, ref)) ??
      snapshots.find(snapshot => isSamePipe(snapshot.edge, ref))?.edge;
    return edge ? [edge] : [];
  });
});
/** Operators by id for labelling inspected pipes, including those on other plans. */
export const inspectedPipeNodesAtom = atom(get => {
  const nodes = new Map<string, DAGNode>();
  for (const { sourceNode, targetNode } of get(inspectedPipeSnapshotsAtom)) {
    for (const node of [sourceNode, targetNode]) {
      if (node) {
        nodes.set(node.id, node);
      }
    }
  }
  for (const node of get(displayedDagNodesAtom)) {
    nodes.set(node.id, node);
  }
  return nodes;
});
