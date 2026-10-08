// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { DAGNode } from '@quent/utils';
import type { HoveredStatInfo } from '@quent/hooks';
import { resolveHoveredStatValue } from '@quent/hooks';

export interface DagHeatmap {
  values: ReadonlyMap<string, number>;
  range: { min: number; max: number };
}

function getRelatedOperatorIds(node: DAGNode): readonly string[] {
  const relatedOperatorIds = node.metadata?.relatedOperatorIds;
  return Array.isArray(relatedOperatorIds)
    ? relatedOperatorIds.filter((id): id is string => typeof id === 'string')
    : [];
}

export function resolveDagHeatmap(
  hoveredStat: HoveredStatInfo | null,
  nodes: readonly DAGNode[]
): DagHeatmap | null {
  if (!hoveredStat) {
    return null;
  }

  const values = new Map<string, number>();
  let min = Infinity;
  let max = -Infinity;
  for (const node of nodes) {
    const resolved = resolveHoveredStatValue(hoveredStat, node.id, getRelatedOperatorIds(node));
    if (!resolved) {
      continue;
    }
    values.set(node.id, resolved.value);
    min = Math.min(min, resolved.value);
    max = Math.max(max, resolved.value);
  }

  return values.size > 0 ? { values, range: { min, max } } : null;
}

export function resolveDagHighlightedNodeIds(
  highlightedNodeIds: ReadonlySet<string> | null,
  nodes: readonly DAGNode[]
): ReadonlySet<string> | null {
  if (!highlightedNodeIds || highlightedNodeIds.size === 0) {
    return null;
  }
  for (const node of nodes) {
    if (highlightedNodeIds.has(node.id)) {
      return highlightedNodeIds;
    }
    for (const relatedOperatorId of getRelatedOperatorIds(node)) {
      if (highlightedNodeIds.has(relatedOperatorId)) {
        return highlightedNodeIds;
      }
    }
  }
  return null;
}
