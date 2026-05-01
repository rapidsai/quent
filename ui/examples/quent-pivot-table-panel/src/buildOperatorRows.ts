// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { parseCustomStatistics } from '@quent/components';
import type { StatValue, QueryEntities } from '@quent/utils';

/**
 * One row per operator across the included plans. Mirrors the shape consumed
 * by `PivotedStatTable` in the main Quent UI's OperatorTable, but lives here
 * so the example is self-contained and can be ported to other hosts.
 */
export interface OperatorRow {
  partitionId: string;
  partitionLabel: string;
  scopeId: string;
  scopeLabel: string;
  itemType: string;
  itemName: string;
  itemId: string;
  stats: Record<string, StatValue>;
}

/**
 * Walk the `QueryEntities` graph and produce `OperatorRow`s for every
 * operator under the given plan ids. Plans are sorted by worker → plan id;
 * operators by type → instance name.
 *
 * `includedPlanIds` defaults to "all plans" so the panel is useful out of the
 * box without forcing the user to pick a plan.
 */
export function buildOperatorRows(
  entities: QueryEntities,
  includedPlanIds?: Set<string>
): OperatorRow[] {
  const rows: OperatorRow[] = [];
  const plans = Object.values(entities.plans)
    .filter(
      (p): p is NonNullable<typeof p> =>
        p != null && (includedPlanIds == null || includedPlanIds.has(p.id))
    )
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
        itemType,
        itemName,
        itemId: op.id,
        stats,
      });
    }
  }
  return rows;
}
