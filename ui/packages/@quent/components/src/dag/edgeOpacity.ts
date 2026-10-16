// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

const NO_EDGES: ReadonlySet<string> = new Set();

/**
 * Edges are individual: only an inspected or hovered pipe is emphasized, never an edge because
 * of the operators it touches. Like nodes, every other edge dims while anything is selected,
 * inspected, hovered or highlighted.
 */
export function shouldDimEdgeFromInteraction({
  hoveredEdgeId,
  inspectedEdgeIds = NO_EDGES,
  edgeId,
  selectedNodeIds,
  highlightedNodeIds,
}: {
  /** A transient hover emphasizes this edge in addition to the inspected pipes. */
  hoveredEdgeId?: string;
  inspectedEdgeIds?: ReadonlySet<string>;
  edgeId?: string;
  selectedNodeIds: ReadonlySet<string>;
  highlightedNodeIds: ReadonlySet<string> | null;
}): boolean {
  if (edgeId !== undefined && (edgeId === hoveredEdgeId || inspectedEdgeIds.has(edgeId))) {
    return false;
  }
  return (
    hoveredEdgeId !== undefined ||
    selectedNodeIds.size > 0 ||
    inspectedEdgeIds.size > 0 ||
    highlightedNodeIds !== null
  );
}
