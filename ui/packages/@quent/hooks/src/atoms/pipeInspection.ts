// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { atom } from 'jotai';
import type { DAGEdge, DAGNode, PipeRef } from '@quent/utils';

/** Inspection identity is independent of the canonical operator filter. */
export const inspectedPipeRefAtom = atom<PipeRef | null>(null);
export const hoveredPipeIdAtom = atom<string | null>(null);
export const displayedDagNodesAtom = atom<readonly DAGNode[]>([]);
export const displayedPipesAtom = atom<readonly DAGEdge[]>([]);
/** Inspects the pipe, or closes the inspection when that pipe is already inspected. */
export const toggleInspectedPipeAtom = atom(null, (get, set, ref: PipeRef) => {
  const current = get(inspectedPipeRefAtom);
  const isSamePipe =
    current?.sourcePortId === ref.sourcePortId && current.targetPortId === ref.targetPortId;
  set(inspectedPipeRefAtom, isSamePipe ? null : ref);
});
export const inspectedPipeAtom = atom(get => {
  const ref = get(inspectedPipeRefAtom);
  return ref
    ? (get(displayedPipesAtom).find(
        edge => edge.sourcePortId === ref.sourcePortId && edge.targetPortId === ref.targetPortId
      ) ?? null)
    : null;
});
