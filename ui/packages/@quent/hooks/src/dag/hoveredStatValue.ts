// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { aggregateToNumber } from '@quent/utils';
import type { HoveredStatInfo } from '../atoms/dagControls';

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
): number | undefined {
  const direct = hoveredStat.values.get(operatorId);
  if (direct !== undefined) {
    return direct;
  }
  const related = relatedOperatorIds.flatMap(id => {
    const v = hoveredStat.values.get(id);
    return v === undefined ? [] : [v];
  });
  return aggregateToNumber(related, hoveredStat.aggMode);
}
