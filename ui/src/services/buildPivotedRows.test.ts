// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, it, expect } from 'vitest';
import {
  buildPivotedRows,
  computeRowSpans,
  type GroupIndexDef,
  type StatGroupExpandedRow,
} from '@quent/components';

/**
 * These tests live under `ui/src/` rather than inside `@quent/components/` so
 * the root vitest runner (which matches `src/**\/*.test.ts`) picks them up
 * without extra configuration. The function under test is a pure reducer
 * with no React dependencies, so cross-boundary testing is fine here —
 * same pattern as `api.test.ts` testing `@quent/utils`.
 */

function expanded(
  groups: Record<string, { id: string; label?: string }>,
  itemId: string,
  statisticName: string,
  value: number | string | null
): StatGroupExpandedRow {
  const normalized: Record<string, { id: string; label: string }> = {};
  for (const [k, v] of Object.entries(groups)) {
    normalized[k] = { id: v.id, label: v.label ?? v.id };
  }
  return {
    groups: normalized,
    itemType: groups.item_type?.id ?? '-',
    itemId,
    scopeId: groups.partition?.id ?? '-',
    statisticName,
    value,
  };
}

const brandIdx: GroupIndexDef = {
  key: 'brand',
  getId: r => r.groups.brand.id,
  getLabel: r => r.groups.brand.label,
};
const fuelIdx: GroupIndexDef = {
  key: 'fuel',
  getId: r => r.groups.fuel.id,
  getLabel: r => r.groups.fuel.label,
};

describe('buildPivotedRows row clustering', () => {
  it('clusters interleaved groups so same-brand rows become contiguous', () => {
    // Mimics the cars-dataset pathology: Ford, Hyundai, BMW, Hyundai, Honda,
    // BMW — brands repeat but are not adjacent in the input.
    const rows: StatGroupExpandedRow[] = [
      expanded({ brand: { id: 'Ford' }, fuel: { id: 'Hybrid' } }, 'car-1', 'price', 34000),
      expanded({ brand: { id: 'Hyundai' }, fuel: { id: 'Electric' } }, 'car-2', 'price', 55000),
      expanded({ brand: { id: 'BMW' }, fuel: { id: 'Diesel' } }, 'car-3', 'price', 41000),
      expanded({ brand: { id: 'Hyundai' }, fuel: { id: 'Petrol' } }, 'car-4', 'price', 54000),
      expanded({ brand: { id: 'Honda' }, fuel: { id: 'Petrol' } }, 'car-5', 'price', 54000),
      expanded({ brand: { id: 'BMW' }, fuel: { id: 'Petrol' } }, 'car-6', 'price', 52000),
      expanded({ brand: { id: 'Ford' }, fuel: { id: 'Petrol' } }, 'car-7', 'price', 56000),
    ];

    const out = buildPivotedRows(rows, [brandIdx, fuelIdx], false);
    const brandOrder = out.map(r => r.groupKeys[0].id);

    // Same brand should appear consecutively.
    const seen = new Set<string>();
    let prev = '';
    for (const b of brandOrder) {
      if (b !== prev) {
        expect(seen.has(b)).toBe(false);
        seen.add(b);
        prev = b;
      }
    }

    // First-appearance order of brands is preserved.
    const firstAppearance: string[] = [];
    for (const b of brandOrder) {
      if (!firstAppearance.includes(b)) firstAppearance.push(b);
    }
    expect(firstAppearance).toEqual(['Ford', 'Hyundai', 'BMW', 'Honda']);
  });

  it('enables computeRowSpans to collapse the outer group column', () => {
    // Same rows as above — verify the clustering is actually usable by the
    // rowspan merger (that's the whole point of the reordering).
    const rows: StatGroupExpandedRow[] = [
      expanded({ brand: { id: 'Ford' }, fuel: { id: 'Hybrid' } }, 'car-1', 'price', 1),
      expanded({ brand: { id: 'Hyundai' }, fuel: { id: 'Electric' } }, 'car-2', 'price', 1),
      expanded({ brand: { id: 'Ford' }, fuel: { id: 'Petrol' } }, 'car-3', 'price', 1),
      expanded({ brand: { id: 'Hyundai' }, fuel: { id: 'Petrol' } }, 'car-4', 'price', 1),
    ];

    const out = buildPivotedRows(rows, [brandIdx, fuelIdx], false);
    const spans = computeRowSpans(out);

    const fordSpan = spans
      .map((colSpans, i) => ({ span: colSpans[0], brand: out[i].groupKeys[0].id }))
      .find(e => e.brand === 'Ford');
    expect(fordSpan?.span).toBe(2);
  });

  it('is a no-op when rows are already contiguous by group key', () => {
    // Operator-panel shape: partition rows already clustered by plan, with
    // item_type following pipeline order within each partition. Must not be
    // reshuffled.
    const partitionIdx: GroupIndexDef = {
      key: 'partition',
      getId: r => r.groups.partition.id,
      getLabel: r => r.groups.partition.label,
    };
    const typeIdx: GroupIndexDef = {
      key: 'item_type',
      getId: r => r.groups.item_type.id,
      getLabel: r => r.groups.item_type.label,
    };

    const pipelineOrder: Array<[string, string]> = [
      ['plan-a', 'Scan'],
      ['plan-a', 'Filter'],
      ['plan-a', 'Project'],
      ['plan-a', 'Aggregate'],
      ['plan-a', 'Sort'],
      ['plan-b', 'Scan'],
      ['plan-b', 'Join'],
      ['plan-b', 'Sort'],
      ['plan-b', 'Project'],
    ];
    const rows: StatGroupExpandedRow[] = pipelineOrder.map(([p, t], i) =>
      expanded(
        { partition: { id: p }, item_type: { id: t } },
        `op-${i}`,
        'duration_s',
        0.1 + i * 0.01
      )
    );

    const out = buildPivotedRows(rows, [partitionIdx, typeIdx], false);
    const observed = out.map(r => [r.groupKeys[0].id, r.groupKeys[1].id] as const);
    expect(observed).toEqual(pipelineOrder);
  });

  it('still clusters when aggregating', () => {
    // Aggregation dedupes by group key (multiple input rows → one output row)
    // so the ordering contract is especially important here — computeRowSpans
    // is the only way a user can tell two partitions apart visually.
    const rows: StatGroupExpandedRow[] = [
      expanded({ brand: { id: 'Ford' }, fuel: { id: 'Hybrid' } }, 'car-1', 'price', 30000),
      expanded({ brand: { id: 'Hyundai' }, fuel: { id: 'Electric' } }, 'car-2', 'price', 55000),
      expanded({ brand: { id: 'Ford' }, fuel: { id: 'Hybrid' } }, 'car-3', 'price', 50000),
      expanded({ brand: { id: 'Ford' }, fuel: { id: 'Petrol' } }, 'car-4', 'price', 56000),
    ];
    const out = buildPivotedRows(rows, [brandIdx, fuelIdx], true);
    const brandOrder = out.map(r => r.groupKeys[0].id);
    expect(brandOrder).toEqual(['Ford', 'Ford', 'Hyundai']);
    const fordHybrid = out.find(
      r => r.groupKeys[0].id === 'Ford' && r.groupKeys[1].id === 'Hybrid'
    );
    expect(fordHybrid?.aggs.get('price')?.sum).toBe(80000);
    expect(fordHybrid?.aggs.get('price')?.count).toBe(2);
  });

  it('preserves first-appearance order across both columns of the hierarchy', () => {
    const rows: StatGroupExpandedRow[] = [
      expanded({ brand: { id: 'Ford' }, fuel: { id: 'Hybrid' } }, 'c1', 'x', 1),
      expanded({ brand: { id: 'Ford' }, fuel: { id: 'Petrol' } }, 'c2', 'x', 1),
      expanded({ brand: { id: 'Ford' }, fuel: { id: 'Hybrid' } }, 'c3', 'x', 1),
      expanded({ brand: { id: 'BMW' }, fuel: { id: 'Diesel' } }, 'c4', 'x', 1),
      expanded({ brand: { id: 'BMW' }, fuel: { id: 'Petrol' } }, 'c5', 'x', 1),
      expanded({ brand: { id: 'BMW' }, fuel: { id: 'Diesel' } }, 'c6', 'x', 1),
    ];
    const out = buildPivotedRows(rows, [brandIdx, fuelIdx], false);
    const pairs = out.map(r => [r.groupKeys[0].id, r.groupKeys[1].id] as const);
    // Within Ford: Hybrid appeared before Petrol. Within BMW: Diesel before Petrol.
    expect(pairs).toEqual([
      ['Ford', 'Hybrid'],
      ['Ford', 'Hybrid'],
      ['Ford', 'Petrol'],
      ['BMW', 'Diesel'],
      ['BMW', 'Diesel'],
      ['BMW', 'Petrol'],
    ]);
  });

  it('handles the single-column case', () => {
    const rows: StatGroupExpandedRow[] = [
      expanded({ brand: { id: 'Ford' } }, 'c1', 'x', 1),
      expanded({ brand: { id: 'BMW' } }, 'c2', 'x', 1),
      expanded({ brand: { id: 'Ford' } }, 'c3', 'x', 1),
      expanded({ brand: { id: 'BMW' } }, 'c4', 'x', 1),
    ];
    const out = buildPivotedRows(rows, [brandIdx], false);
    expect(out.map(r => r.groupKeys[0].id)).toEqual(['Ford', 'Ford', 'BMW', 'BMW']);
  });

  it('tolerates zero active indices', () => {
    const rows: StatGroupExpandedRow[] = [
      expanded({ partition: { id: '-' } }, 'c1', 'x', 1),
      expanded({ partition: { id: '-' } }, 'c2', 'x', 2),
    ];
    const out = buildPivotedRows(rows, [], false);
    // Nothing to cluster on — should not crash and should produce one row per
    // distinct flat-row identity (here: every row collapses into the empty
    // rowKey, so exactly one result row).
    expect(out).toHaveLength(1);
  });
});
