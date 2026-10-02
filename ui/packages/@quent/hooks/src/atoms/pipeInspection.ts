// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { atom } from 'jotai';
import type { DAGEdge, DAGNode, PipeRef } from '@quent/utils';

/** Inspection identity is independent of the canonical operator filter. */
export const inspectedPipeRefAtom = atom<PipeRef | null>(null);
export const hoveredPipeIdAtom = atom<string | null>(null);
export const displayedDagNodesAtom = atom<readonly DAGNode[]>([]);
export const displayedPipesAtom = atom<readonly DAGEdge[]>([]);
export const inspectedPipeAtom = atom(get => {
  const ref = get(inspectedPipeRefAtom);
  return ref
    ? (get(displayedPipesAtom).find(
        edge => edge.sourcePortId === ref.sourcePortId && edge.targetPortId === ref.targetPortId
      ) ?? null)
    : null;
});
