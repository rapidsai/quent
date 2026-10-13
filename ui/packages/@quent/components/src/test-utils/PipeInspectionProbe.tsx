// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { useEffect, type ReactNode } from 'react';
import {
  useHighlightedNodeIds,
  useHoveredPipeId,
  useInspectedPipe,
  useOperatorSelectionActions,
  useSelectedOperatorIds,
  useSelectedOperatorsData,
  useSetInspectedPipe,
  useSyncDisplayedPipes,
} from '@quent/hooks';
import type { DAGEdge, DAGNode } from '@quent/utils';

type Actions = {
  setInspectedPipe: ReturnType<typeof useSetInspectedPipe>;
  updateSelection: ReturnType<typeof useOperatorSelectionActions>;
};

/** The latest pipe-inspection state, read through the public hooks. */
export type PipeInspectionSnapshot = Actions & {
  inspectedPipe: ReturnType<typeof useInspectedPipe>;
  hoveredPipeId: ReturnType<typeof useHoveredPipeId>;
  highlight: ReturnType<typeof useHighlightedNodeIds>[0];
  selectedOperatorIds: ReturnType<typeof useSelectedOperatorIds>;
  selectedOperatorsData: ReturnType<typeof useSelectedOperatorsData>;
};

/** Holds the latest snapshot the probe reported. */
export interface PipeInspectionRecorder {
  latest: () => PipeInspectionSnapshot;
  record: (snapshot: PipeInspectionSnapshot) => void;
}

export function createPipeInspectionRecorder(): PipeInspectionRecorder {
  let snapshot: PipeInspectionSnapshot | null = null;
  return {
    latest: () => {
      if (!snapshot) {
        throw new Error('PipeInspectionProbe has not rendered yet');
      }
      return snapshot;
    },
    record: next => {
      snapshot = next;
    },
  };
}

// Stable defaults: a fresh array each render would re-run the sync effect and reset hover state.
const NO_EDGES: readonly DAGEdge[] = [];
const NO_NODES: readonly DAGNode[] = [];

/**
 * Test helper: seeds the displayed pipes and nodes the way `DAGChart` does and
 * records the resulting state, so tests need no atom imports.
 */
export function PipeInspectionProbe({
  edges = NO_EDGES,
  nodes = NO_NODES,
  recorder,
  children,
}: {
  edges?: readonly DAGEdge[];
  nodes?: readonly DAGNode[];
  recorder: PipeInspectionRecorder;
  /** Rendered inside the probe so consumers subscribe before the pipes are seeded, as under `DAGChart`. */
  children?: ReactNode;
}) {
  useSyncDisplayedPipes(edges, nodes);
  const [highlight] = useHighlightedNodeIds();
  const snapshot: PipeInspectionSnapshot = {
    inspectedPipe: useInspectedPipe(),
    hoveredPipeId: useHoveredPipeId(),
    highlight,
    selectedOperatorIds: useSelectedOperatorIds(),
    selectedOperatorsData: useSelectedOperatorsData(),
    setInspectedPipe: useSetInspectedPipe(),
    updateSelection: useOperatorSelectionActions(),
  };
  useEffect(() => recorder.record(snapshot));
  return <>{children}</>;
}
