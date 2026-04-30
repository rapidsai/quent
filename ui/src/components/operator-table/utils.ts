// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { QueryEntities } from '~quent/types/QueryEntities';
import type { StatValue } from '@quent/utils';
import { parseCustomStatistics } from '@quent/components';
import type { OperatorTableRow } from './types';

/**
 * Flatten a `QueryEntities` graph into one row per operator across the given
 * plans. Plans are sorted by worker id then plan id; operators within each
 * plan are sorted by operator type then instance name. Parent-operator-derived
 * fields collapse duplicate values via Set/join, and missing values are
 * normalized to the placeholder `'-'`.
 */
export function buildOperatorRows(
  entities: QueryEntities,
  includedPlanIds: Set<string>
): OperatorTableRow[] {
  const rows: OperatorTableRow[] = [];
  const plans = Object.values(entities.plans)
    .filter((p): p is NonNullable<typeof p> => p != null && includedPlanIds.has(p.id))
    .sort((a, b) => {
      const wA = a.worker_id ?? '';
      const wB = b.worker_id ?? '';
      if (wA !== wB) return wA.localeCompare(wB);
      return a.id.localeCompare(b.id);
    });

  for (const plan of plans) {
    const worker = plan.worker_id ? entities.workers[plan.worker_id] : undefined;
    const workerPart = worker?.instance_name ?? plan.worker_id ?? '-';
    const planPart = plan.instance_name ?? plan.id;
    const partitionLabel = `${workerPart} / ${planPart}`;
    const partitionId = `${plan.worker_id ?? '-'}:${plan.id}`;

    const ops = Object.values(entities.operators)
      .filter((op): op is NonNullable<typeof op> => op != null && op.plan_id === plan.id)
      .sort((a, b) => {
        const typeA = a.operator_type_name ?? '';
        const typeB = b.operator_type_name ?? '';
        if (typeA !== typeB) return typeA.localeCompare(typeB);
        const nameA = a.instance_name ?? a.id;
        const nameB = b.instance_name ?? b.id;
        return nameA.localeCompare(nameB);
      });

    for (const op of ops) {
      const itemName = op.instance_name ?? op.id;
      const itemType = op.operator_type_name ?? '-';
      const parentOps = (op.parent_operator_ids ?? [])
        .map(id => entities.operators[id])
        .filter((p): p is NonNullable<typeof p> => p != null);
      const parentScopeLabel =
        parentOps.length > 0
          ? [
              ...new Set(
                parentOps.map(p =>
                  p.plan_id ? (entities.plans[p.plan_id]?.instance_name ?? '-') : '-'
                )
              ),
            ].join(', ')
          : '-';
      const parentItemType =
        parentOps.length > 0
          ? [...new Set(parentOps.map(p => p.operator_type_name ?? '-'))].join(', ')
          : '-';
      const parentItemName =
        parentOps.length > 0 ? parentOps.map(p => p.instance_name ?? p.id).join(', ') : '-';
      const duration = op.active_span ? op.active_span.end - op.active_span.start : null;
      const stats: Record<string, StatValue> = {
        duration_s: duration !== null ? Number(duration.toFixed(6)) : null,
      };
      for (const stat of parseCustomStatistics(op)) {
        stats[stat.key] = stat.value;
      }
      rows.push({
        partitionId,
        partitionLabel,
        scopeId: plan.id,
        scopeLabel: planPart,
        parentScopeLabel,
        parentItemType,
        parentItemName,
        itemType,
        itemName,
        itemId: op.id,
        stats,
      });
    }
  }
  return rows;
}

/**
 * Build a `field value -> Set<itemId>` index over `rows`. The selected `field`
 * must be a string-valued column; rows whose value is the placeholder `'-'`
 * are skipped when `skipDash` is true (the default).
 */
export function buildItemIdIndex<T extends keyof OperatorTableRow>(
  rows: OperatorTableRow[],
  field: T,
  skipDash = true
): Map<string, Set<string>> {
  const map = new Map<string, Set<string>>();
  for (const row of rows) {
    const key = row[field];
    if (typeof key !== 'string') continue;
    if (skipDash && key === '-') continue;
    let set = map.get(key);
    if (!set) {
      set = new Set();
      map.set(key, set);
    }
    set.add(row.itemId);
  }
  return map;
}
