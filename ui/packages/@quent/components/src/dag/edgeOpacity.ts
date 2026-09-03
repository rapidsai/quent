// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

export function shouldDimEdgeFromInteraction({
  sourceId,
  targetId,
  selectedNodeIds,
  highlightedNodeIds,
}: {
  sourceId: string;
  targetId: string;
  selectedNodeIds: ReadonlySet<string>;
  highlightedNodeIds: ReadonlySet<string> | null;
}): boolean {
  const hasSelection = selectedNodeIds.size > 0;
  const hasActiveHighlight = highlightedNodeIds !== null;
  const touchesSelection = selectedNodeIds.has(sourceId) || selectedNodeIds.has(targetId);
  const touchesHighlight =
    highlightedNodeIds?.has(sourceId) === true || highlightedNodeIds?.has(targetId) === true;

  return (hasActiveHighlight || hasSelection) && !touchesHighlight && !touchesSelection;
}
