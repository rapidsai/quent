// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

export function getNodeOpacityClass({
  isHoveredStatActive,
  hasHoveredValue,
  highlightedNodeIds,
  operatorId,
  isDimmed,
  isFiltered,
}: {
  /** Whether a stat column is currently being hovered in the pivot table (or DAG). */
  isHoveredStatActive: boolean;
  /**
   * Whether this node resolved to a value for the hovered stat — either a
   * direct value or one aggregated from related operators (see
   * `resolveHoveredStatValue`).
   */
  hasHoveredValue: boolean;
  highlightedNodeIds: ReadonlySet<string> | null;
  operatorId: string;
  isDimmed: boolean;
  isFiltered: boolean;
}): string {
  if (isHoveredStatActive) {
    return hasHoveredValue || isFiltered ? 'opacity-100' : 'opacity-20';
  }
  if (highlightedNodeIds !== null && highlightedNodeIds.size > 0) {
    return highlightedNodeIds.has(operatorId) || isFiltered ? 'opacity-100' : 'opacity-35';
  }
  if (isDimmed) {
    return 'opacity-35';
  }
  return 'opacity-100';
}
