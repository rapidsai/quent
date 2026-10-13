// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

const NO_EDGES: ReadonlySet<string> = new Set();

export function shouldDimEdgeFromInteraction({
  hoveredEdgeId,
  inspectedEdgeIds = NO_EDGES,
  edgeId,
  sourceId,
  targetId,
  selectedNodeIds,
  highlightedNodeIds,
}: {
  /** A transient hover narrows emphasis to this edge alone. */
  hoveredEdgeId?: string;
  /** Inspected pipes are emphasized alongside the selected operators. */
  inspectedEdgeIds?: ReadonlySet<string>;
  edgeId?: string;
  sourceId: string;
  targetId: string;
  selectedNodeIds: ReadonlySet<string>;
  highlightedNodeIds: ReadonlySet<string> | null;
}): boolean {
  if (hoveredEdgeId) {
    return edgeId !== hoveredEdgeId;
  }
  if (edgeId !== undefined && inspectedEdgeIds.has(edgeId)) {
    return false;
  }
  const hasSelection = selectedNodeIds.size > 0 || inspectedEdgeIds.size > 0;
  const hasActiveHighlight = highlightedNodeIds !== null;
  const touchesSelection = selectedNodeIds.has(sourceId) || selectedNodeIds.has(targetId);
  const touchesHighlight =
    highlightedNodeIds?.has(sourceId) === true || highlightedNodeIds?.has(targetId) === true;

  return (hasActiveHighlight || hasSelection) && !touchesHighlight && !touchesSelection;
}
