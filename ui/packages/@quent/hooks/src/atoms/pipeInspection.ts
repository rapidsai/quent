// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { atom } from 'jotai';
import { isSamePipe, type DAGEdge, type DAGNode, type PipeRef } from '@quent/utils';

/** Inspection identity is independent of the canonical operator filter. */
export const inspectedPipeRefAtom = atom<PipeRef | null>(null);
export const hoveredPipeIdAtom = atom<string | null>(null);
export const displayedDagNodesAtom = atom<readonly DAGNode[]>([]);
export const displayedPipesAtom = atom<readonly DAGEdge[]>([]);
/** Inspects the pipe, or closes the inspection when that pipe is already inspected. */
export const toggleInspectedPipeAtom = atom(null, (get, set, ref: PipeRef) => {
  set(inspectedPipeRefAtom, isSamePipe(get(inspectedPipeRefAtom), ref) ? null : ref);
});
export const inspectedPipeAtom = atom(get => {
  const ref = get(inspectedPipeRefAtom);
  return ref ? (get(displayedPipesAtom).find(edge => isSamePipe(edge, ref)) ?? null) : null;
});
