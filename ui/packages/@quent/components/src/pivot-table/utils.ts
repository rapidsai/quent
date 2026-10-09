// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import {
  aggregateNumericValues,
  getAggregateValue,
  inferFieldFormatter,
  isNumericValue,
  isStatStruct,
  formatStatWithQuantity,
} from '@quent/utils';
import type { StatValue, Statistic, QuantitySpec, ContinuousPaletteName } from '@quent/utils';
import { continuousColor } from '@quent/utils';
import type { GroupedDataTableSortFn } from './GroupedDataTable';

// Re-exported for consumers that still import it from here; defined in `@quent/utils`
export { isNumericValue };

import type {
  StatGroupExpandedRow,
  GroupKeyEntry,
  PivotedRow,
  PivotedRowAgg,
  AggMode,
  PivotedStatTableSchema,
} from './types';

/**
 * Defines how a single grouping dimension maps a StatGroupExpandedRow
 * to its key/label values.
 */
export interface GroupIndexDef {
  key: string;
  getId: (row: StatGroupExpandedRow) => string;
  getLabel: (row: StatGroupExpandedRow) => string;
}

export function formatNumericStat(n: number | bigint | null, statName: string): string {
  if (n === null) {
    return '-';
  }
  return inferFieldFormatter(statName)(n);
}

export const numericSortingFn: GroupedDataTableSortFn<PivotedRow> = (rowA, rowB, columnId) => {
  const a = rowA.getValue<number | bigint | undefined>(columnId);
  const b = rowB.getValue<number | bigint | undefined>(columnId);
  if (a === b) {
    return 0;
  }
  if (a == null) {
    return 1;
  }
  if (b == null) {
    return -1;
  }
  if (typeof a === typeof b) {
    return a < b ? -1 : 1;
  }
  return (a as number) < (b as number) ? -1 : 1;
};

/**
 * Returns true when any id in `items` is present in `target`. Equivalent to
 * `[...items].some(id => target.has(id))` but without allocating an
 * intermediate array — important for hot render paths that compare per-row
 * item-id sets against highlight/selection sets.
 */
export function itemHasId(items: Iterable<string>, target: ReadonlySet<string>): boolean {
  for (const id of items) {
    if (target.has(id)) {
      return true;
    }
  }
  return false;
}

export function formatStatValue(
  value: StatValue,
  statName: string,
  quantitySpecs?: Record<string, QuantitySpec | undefined>,
  quantity?: string
): string {
  if (value === null || value === undefined) {
    return '-';
  }
  if (isNumericValue(value)) {
    return formatStatWithQuantity(
      value,
      statName,
      quantity !== undefined ? quantitySpecs?.[quantity] : undefined
    );
  }
  if (typeof value === 'boolean') {
    return value ? 'true' : 'false';
  }
  if (isStatStruct(value)) {
    return value.fields
      .map(
        field =>
          `${field.key}: ${formatStatValue(field.value, field.key, quantitySpecs, field.quantity)}`
      )
      .join(', ');
  }
  if (Array.isArray(value)) {
    return value.map(item => formatStatValue(item, statName, quantitySpecs, quantity)).join(', ');
  }
  return String(value);
}

/** Only a single scalar statistic defines a numeric cell. */
export function getNumericCellValue(
  statistics: readonly Statistic[] | undefined
): number | bigint | null {
  if (statistics?.length !== 1) {
    return null;
  }
  return isNumericValue(statistics[0].value) ? statistics[0].value : null;
}

// --- color gradient ---

export function gradientBg(
  value: number | bigint,
  min: number | bigint,
  max: number | bigint,
  palette: ContinuousPaletteName = 'blue',
  darkMode = false
): string | undefined {
  const vn = Number(value);
  const mn = Number(min);
  const mx = Number(max);
  if (mn === mx) {
    return undefined;
  }
  const t = (vn - mn) / (mx - mn);
  return continuousColor(t, palette, darkMode);
}

export function rowGroupKey(row: StatGroupExpandedRow, indices: GroupIndexDef[]): string {
  return indices.map(def => def.getId(row)).join('\0');
}

export function getGroupKeys(row: StatGroupExpandedRow, indices: GroupIndexDef[]): GroupKeyEntry[] {
  return indices.map(def => ({ key: def.key, id: def.getId(row), label: def.getLabel(row) }));
}

export function getUniqueStatNames(rows: StatGroupExpandedRow[]): string[] {
  const seen = new Set<string>();
  const names: string[] = [];
  for (const row of rows) {
    if (seen.has(row.statisticName)) {
      continue;
    }
    seen.add(row.statisticName);
    names.push(row.statisticName);
  }
  return names;
}

export function getSchemaStatNames<TRow>(
  rows: TRow[],
  schema: PivotedStatTableSchema<TRow>
): string[] {
  const seen = new Set<string>();
  const names: string[] = [];
  for (const row of rows) {
    const stats = schema.stats(row);
    for (const { key: statName } of stats) {
      if (seen.has(statName)) {
        continue;
      }
      seen.add(statName);
      names.push(statName);
    }
  }
  return names;
}

export function expandRowsFromSchema<TRow>(
  rows: TRow[],
  schema: PivotedStatTableSchema<TRow>
): StatGroupExpandedRow[] {
  const expanded: StatGroupExpandedRow[] = [];
  for (const row of rows) {
    const groups: Record<string, { id: string; label: string }> = {};
    for (const [groupKey, selector] of Object.entries(schema.groups)) {
      const id = selector.id(row);
      groups[groupKey] = { id, label: selector.label?.(row) ?? id };
    }
    const itemId = schema.itemId(row);
    const scopeId = schema.scopeId(row);
    const itemType =
      schema.itemType?.(row) ??
      groups.item_type?.id ??
      groups.item?.id ??
      groups.partition?.id ??
      '-';
    for (const { key: statisticName, value, quantity } of schema.stats(row)) {
      expanded.push({
        groups,
        itemType,
        itemId,
        scopeId,
        statisticName,
        value,
        ...(quantity !== undefined ? { quantity } : {}),
      });
    }
  }
  return expanded;
}

/** Row type constraint for computeRowSpans: only groupKeys with id is used. */
export type RowWithGroupKeys = { groupKeys: Array<{ id: string }> };

export function computeRowSpans<T extends RowWithGroupKeys>(rows: T[]): (number | null)[][] {
  const numCols = rows[0]?.groupKeys.length ?? 0;
  const spans: (number | null)[][] = rows.map(() => new Array(numCols).fill(null));
  if (rows.length === 0) {
    return spans;
  }

  for (let col = 0; col < numCols; col++) {
    let start = 0;
    for (let i = 1; i <= rows.length; i++) {
      const changed =
        i === rows.length ||
        rows[i].groupKeys.slice(0, col + 1).some((gk, j) => gk.id !== rows[i - 1].groupKeys[j]?.id);
      const parentChanged =
        col > 0 &&
        i < rows.length &&
        rows[i].groupKeys.slice(0, col).some((gk, j) => gk.id !== rows[start].groupKeys[j]?.id);
      if (changed || parentChanged) {
        spans[start][col] = i - start;
        start = i;
      }
    }
  }
  return spans;
}

/** Extract the numeric sort value for a stat from a pivoted row. */
export function getSortValue(
  row: PivotedRow,
  stat: string,
  isAgg: boolean,
  aggMode: AggMode
): number | bigint | null {
  if (!isAgg) {
    return getNumericCellValue(row.values.get(stat));
  }
  const agg = row.aggs.get(stat);
  if (!agg || !agg.isNumeric) {
    return null;
  }
  return getAggregateValue(agg, aggMode);
}

type Accumulator = {
  keys: GroupKeyEntry[];
  rowKey: string;
  values: Map<string, Statistic[]>;
  aggBuckets: Map<
    string,
    {
      nums: number[];
      bigints: bigint[];
      count: number;
      itemIds: Set<string>;
      quantities: Set<string | undefined>;
      repeated: boolean;
    }
  >;
  itemIds: Set<string>;
  itemScopeIds: Map<string, string>;
  itemType: string;
};

/**
 * Build pivoted (and optionally aggregated) rows from flat rows.
 *
 * Output ordering: rows are clustered so that same-id runs along the group-key
 * hierarchy are contiguous (which is what `computeRowSpans` needs to merge
 * cells). Within each cluster, ties are broken by the order each distinct id
 * first appeared in `flatRows`, so callers that feed rows in a meaningful
 * sequence (e.g. operator pipeline / execution order) keep that sequence.
 *
 * For already-contiguous inputs (like the operator panels, which feed rows
 * clustered by `partition → item_type → item` naturally) this reordering is a
 * no-op — the stable sort runs over monotonically non-decreasing ranks.
 * For datasets where the natural row order interleaves groups (e.g. a CSV of
 * cars where `Ford, Hyundai, BMW, Hyundai, Honda, BMW, …` alternates brands),
 * it produces the expected grouped view without requiring the caller to
 * pre-sort.
 */
export function buildPivotedRows(
  flatRows: StatGroupExpandedRow[],
  activeIndices: GroupIndexDef[],
  isAggregating: boolean
): PivotedRow[] {
  const groups = new Map<string, Accumulator>();

  for (const row of flatRows) {
    const rk = rowGroupKey(row, activeIndices);
    let group = groups.get(rk);
    if (!group) {
      group = {
        keys: getGroupKeys(row, activeIndices),
        rowKey: rk,
        values: new Map(),
        aggBuckets: new Map(),
        itemIds: new Set(),
        itemScopeIds: new Map(),
        itemType: row.itemType,
      };
      groups.set(rk, group);
    }
    // Used to highlight the table when the DAG is hovered
    group.itemIds.add(row.itemId);
    group.itemScopeIds.set(row.itemId, row.scopeId);

    if (!isAggregating) {
      const statistic: Statistic = {
        key: row.statisticName,
        value: row.value,
        ...(row.quantity !== undefined ? { quantity: row.quantity } : {}),
      };
      const entries = group.values.get(row.statisticName);
      if (entries) {
        entries.push(statistic);
      } else {
        group.values.set(row.statisticName, [statistic]);
      }
    } else {
      let bucket = group.aggBuckets.get(row.statisticName);
      if (!bucket) {
        bucket = {
          nums: [],
          bigints: [],
          count: 0,
          itemIds: new Set(),
          quantities: new Set(),
          repeated: false,
        };
        group.aggBuckets.set(row.statisticName, bucket);
      }
      bucket.repeated ||= bucket.itemIds.has(row.itemId);
      bucket.itemIds.add(row.itemId);
      bucket.quantities.add(row.quantity);
      bucket.count++;
      if (typeof row.value === 'bigint') {
        bucket.bigints.push(row.value);
      } else if (typeof row.value === 'number') {
        bucket.nums.push(row.value);
      }
    }
  }

  const result: PivotedRow[] = [];
  for (const group of groups.values()) {
    const aggs = new Map<string, PivotedRowAgg>();
    if (isAggregating) {
      for (const [stat, bucket] of group.aggBuckets) {
        const aggregates =
          !bucket.repeated && bucket.quantities.size === 1
            ? aggregateNumericValues([...bucket.nums, ...bucket.bigints])
            : null;
        aggs.set(stat, {
          sum: aggregates?.sum ?? null,
          mean: aggregates?.mean ?? null,
          min: aggregates?.min ?? null,
          max: aggregates?.max ?? null,
          stdev: aggregates?.stdev ?? null,
          count: bucket.count,
          isNumeric: aggregates !== null,
          ...(bucket.quantities.size === 1 && bucket.quantities.has(undefined) === false
            ? { quantity: [...bucket.quantities][0] }
            : {}),
        });
      }
    }
    result.push({
      groupKeys: group.keys,
      rowKey: group.rowKey,
      values: group.values,
      aggs,
      itemIds: group.itemIds,
      itemScopeIds: group.itemScopeIds,
      itemType: group.itemType,
    });
  }

  // Cluster rows along the group-key hierarchy without re-sorting them into
  // an alien order. For every column we assign each distinct ancestor-path
  // prefix (col 0 id, then col-0+col-1 ids joined, ...) a rank equal to its
  // first-appearance index among rows seen so far; then we stably sort by
  // the tuple of those ranks. Ranks are scoped by full prefix rather than
  // bare id so within-parent order is preserved even when a child value
  // reappears across parents (e.g. `Sort` shows up in multiple plans but
  // each plan's pipeline order stays intact). For inputs that are already
  // clustered — which is the common case for operator-panel data feeding
  // rows in `partition → item_type → item` order — the stable sort runs
  // over monotonically non-decreasing ranks and is a no-op.
  if (activeIndices.length > 0 && result.length > 1) {
    const pathKey = (row: PivotedRow, c: number): string => {
      let out = row.groupKeys[0].id;
      for (let i = 1; i <= c; i++) {
        out += '\0' + row.groupKeys[i].id;
      }
      return out;
    };
    const rankByCol: Array<Map<string, number>> = activeIndices.map(() => new Map());
    for (const row of result) {
      for (let c = 0; c < row.groupKeys.length; c++) {
        const ranks = rankByCol[c];
        const k = pathKey(row, c);
        if (!ranks.has(k)) {
          ranks.set(k, ranks.size);
        }
      }
    }
    result.sort((a, b) => {
      for (let c = 0; c < a.groupKeys.length; c++) {
        const ra = rankByCol[c].get(pathKey(a, c))!;
        const rb = rankByCol[c].get(pathKey(b, c))!;
        if (ra !== rb) {
          return ra - rb;
        }
      }
      return 0;
    });
  }

  return result;
}
