// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { AggMode } from '@quent/utils';
import type { HoveredStatInfo } from '../atoms/dagControls';

function aggregate(values: number[], mode: AggMode): number {
  const sum = values.reduce((a, b) => a + b, 0);
  switch (mode) {
    case 'mean':
      return sum / values.length;
    case 'min':
      return Math.min(...values);
    case 'max':
      return Math.max(...values);
    case 'stdev': {
      if (values.length < 2) {
        return 0;
      }
      const mean = sum / values.length;
      const variance = values.reduce((acc, v) => acc + (v - mean) ** 2, 0) / (values.length - 1);
      return Math.sqrt(variance);
    }
    case 'sum':
    case 'value':
    default:
      return sum;
  }
}

export interface ResolvedHoveredStatValue {
  value: number;
  /**
   * 'direct' when the operator itself had an entry in the hovered stat
   * (e.g. a physical operator that's also a pivot-table row); 'aggregated'
   * when it was derived from related operators (e.g. a logical-plan node).
   * Aggregated values live on a different scale than raw item values (a sum
   * across several operators routinely exceeds any single item's max), so
   * callers must not compare them against `hoveredStat.min`/`max` directly —
   * see `dagHeatmapRangeAtom`.
   */
  source: 'direct' | 'aggregated';
}

/**
 * Resolves the hovered-stat value for a DAG node. Physical operators that
 * are themselves pivot-table rows get a direct lookup. Nodes that group
 * other operators (e.g. a logical-plan operator whose descendants are the
 * physical operators doing the work) have no entry of their own, so their
 * value is derived by aggregating their related operators' values with
 * `hoveredStat.aggMode`.
 */
export function resolveHoveredStatValue(
  hoveredStat: HoveredStatInfo,
  operatorId: string,
  relatedOperatorIds: readonly string[] = []
): ResolvedHoveredStatValue | undefined {
  const direct = hoveredStat.values.get(operatorId);
  if (direct !== undefined) {
    return { value: direct, source: 'direct' };
  }
  if (relatedOperatorIds.length === 0) {
    return undefined;
  }
  const values: number[] = [];
  for (const id of relatedOperatorIds) {
    const v = hoveredStat.values.get(id);
    if (v !== undefined) {
      values.push(v);
    }
  }
  if (values.length === 0) {
    return undefined;
  }
  return { value: aggregate(values, hoveredStat.aggMode), source: 'aggregated' };
}
