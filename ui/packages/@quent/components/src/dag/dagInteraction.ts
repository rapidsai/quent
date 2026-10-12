// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { computeScaleRange, type DAGNode, type ScaleRange } from '@quent/utils';
import type { HoveredStatInfo } from '@quent/hooks';
import { resolveHoveredStatValue } from '@quent/hooks';

export interface DagHeatmap {
  values: ReadonlyMap<string, number>;
  range: ScaleRange;
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
  for (const node of nodes) {
    const resolved = resolveHoveredStatValue(hoveredStat, node.id, getRelatedOperatorIds(node));
    if (resolved === undefined) {
      continue;
    }
    values.set(node.id, resolved);
  }

  return values.size > 0 ? { values, range: computeScaleRange([...values.values()]) } : null;
}

export function resolveDagHighlightedNodeIds(
  highlightedNodeIds: ReadonlySet<string> | null,
  nodes: readonly DAGNode[]
): ReadonlySet<string> | null {
  if (!highlightedNodeIds || highlightedNodeIds.size === 0) {
    return null;
  }
  const resolved = new Set(highlightedNodeIds);
  let intersectsDag = false;
  for (const node of nodes) {
    if (highlightedNodeIds.has(node.id)) {
      intersectsDag = true;
    }
    for (const relatedOperatorId of getRelatedOperatorIds(node)) {
      if (highlightedNodeIds.has(relatedOperatorId)) {
        resolved.add(node.id);
        intersectsDag = true;
        break;
      }
    }
  }
  if (!intersectsDag) {
    return null;
  }
  return resolved.size > highlightedNodeIds.size ? resolved : highlightedNodeIds;
}
