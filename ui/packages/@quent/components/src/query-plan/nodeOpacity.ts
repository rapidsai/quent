// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

export function getNodeOpacityClass({
  hoveredStatValues,
  highlightedNodeIds,
  operatorId,
  isDimmed,
  isSelected,
}: {
  hoveredStatValues: ReadonlyMap<string, number> | null | undefined;
  highlightedNodeIds: ReadonlySet<string> | null;
  operatorId: string;
  isDimmed: boolean;
  isSelected: boolean;
}): string {
  if (hoveredStatValues) {
    return hoveredStatValues.has(operatorId) || isSelected ? 'opacity-100' : 'opacity-20';
  }
  if (highlightedNodeIds !== null && highlightedNodeIds.size > 0) {
    return highlightedNodeIds.has(operatorId) || isSelected ? 'opacity-100' : 'opacity-35';
  }
  if (isDimmed) {
    return 'opacity-35';
  }
  return 'opacity-100';
}
