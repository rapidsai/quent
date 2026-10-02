// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { resolveGroupedValue } from '@quent/utils';
import type { HoveredStatInfo } from '../atoms/dagControls';

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
  const related = relatedOperatorIds.flatMap(id => {
    const v = hoveredStat.values.get(id);
    return v === undefined ? [] : [v];
  });
  return resolveGroupedValue(hoveredStat.values.get(operatorId), related, hoveredStat.aggMode);
}
