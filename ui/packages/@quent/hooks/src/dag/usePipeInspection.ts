// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useEffect } from 'react';
import { useAtomValue, useSetAtom, useStore } from 'jotai';
import type { DAGEdge } from '@quent/utils';
import {
  displayedPipesAtom,
  inspectedPipeAtom,
  inspectedPipeRefAtom,
} from '../atoms/pipeInspection';

export const useInspectedPipe = () => useAtomValue(inspectedPipeAtom);
export const useSetInspectedPipe = () => useSetAtom(inspectedPipeRefAtom);

export function useSyncDisplayedPipes(edges: readonly DAGEdge[]) {
  const store = useStore();
  useEffect(() => {
    store.set(displayedPipesAtom, edges);
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
    };
  }, [edges, store]);
}
