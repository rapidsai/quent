import { formatWithPrefix } from '@/services/formatters';
import type { StatValue } from '@/services/query-plan/types';
import type { FlatRow, GroupKeyEntry, IndexKey, PivotedRow, PivotedRowAgg, AggMode } from './types';

export function formatNumber(n: number | null): string {
  if (n === null) return '-';
  if (Number.isInteger(n)) return n.toLocaleString();
  return n.toLocaleString(undefined, { maximumFractionDigits: 4 });
}

export function formatBytes(n: number | null): string {
  if (n === null) return '-';
  return formatWithPrefix(n, 'B', 'Iec', 2);
}

export function isBytesStat(name: string): boolean {
  return name.includes('_bytes') || name.endsWith('_byte') || name.startsWith('bytes_');
}

export function isCountStat(name: string): boolean {
  return (
    name.includes('_rows') ||
    name.endsWith('_row') ||
    name.startsWith('rows_') ||
    name.includes('_batches') ||
    name.endsWith('_batch') ||
    name.startsWith('batches_')
  );
}

export function formatRows(n: number | null): string {
  if (n === null) return '-';
  return formatWithPrefix(n, '', 'Si', 2);
}

export function formatNumericStat(n: number | null, statName: string): string {
  if (n === null) return '-';
  if (isBytesStat(statName)) return formatBytes(n);
  if (isCountStat(statName)) return formatRows(n);
  return formatNumber(n);
}

export function formatStatValue(value: StatValue, statName: string): string {
  if (value === null || value === undefined) return '-';
  if (typeof value === 'number') return formatNumericStat(value, statName);
  if (typeof value === 'boolean') return value ? 'true' : 'false';
  if (Array.isArray(value)) return value.join(', ');
  return String(value);
}

export function formatStatNumber(n: number | null, statName: string): string {
  return formatNumericStat(n, statName);
}

export function isNumericValue(v: StatValue): v is number {
  return typeof v === 'number';
}

// --- color gradient ---

const GRADIENT_COLOR: [number, number, number] = [239, 68, 68]; // red-500

export function gradientBg(value: number, min: number, max: number): string | undefined {
  if (min === max) return undefined;
  const t = (value - min) / (max - min);
  const alpha = t * 0.45;
  return `rgba(${GRADIENT_COLOR[0]}, ${GRADIENT_COLOR[1]}, ${GRADIENT_COLOR[2]}, ${alpha.toFixed(3)})`;
}

// --- grouping helpers ---

export function rowGroupKey(row: FlatRow, enabledIndices: IndexKey[]): string {
  return enabledIndices
    .map(idx => {
      switch (idx) {
        case 'partition':
          return row.partitionId;
        case 'parent_item_type':
          return row.parentItemType;
        case 'parent_item':
          return row.parentItemName;
        case 'item_type':
          return row.itemType;
        case 'item':
          return row.itemId;
      }
    })
    .join('\0');
}

export function getGroupKeys(row: FlatRow, enabledIndices: IndexKey[]): GroupKeyEntry[] {
  return enabledIndices.map(idx => {
    switch (idx) {
      case 'partition':
        return { key: idx, id: row.partitionId, label: row.partitionLabel };
      case 'parent_item_type':
        return { key: idx, id: row.parentItemType, label: row.parentItemType };
      case 'parent_item':
        return { key: idx, id: row.parentItemName, label: row.parentItemName };
      case 'item_type':
        return { key: idx, id: row.itemType, label: row.itemType };
      case 'item':
        return { key: idx, id: row.itemId, label: row.itemName };
    }
  });
}

/** Row type constraint for computeRowSpans: only groupKeys with id is used. */
export type RowWithGroupKeys = { groupKeys: Array<{ id: string }> };

export function computeRowSpans<T extends RowWithGroupKeys>(rows: T[]): (number | null)[][] {
  const numCols = rows[0]?.groupKeys.length ?? 0;
  const spans: (number | null)[][] = rows.map(() => new Array(numCols).fill(null));
  if (rows.length === 0) return spans;

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
): number | null {
  if (!isAgg) {
    const v = row.values.get(stat);
    if (v === undefined) return null;
    return isNumericValue(v) ? v : null;
  }
  const agg = row.aggs.get(stat);
  if (!agg || !agg.isNumeric) return null;
  switch (aggMode) {
    case 'sum':
      return agg.sum;
    case 'mean':
      return agg.mean;
    case 'min':
      return agg.min;
    case 'max':
      return agg.max;
    case 'stdev':
      return agg.stdev;
    default:
      return agg.sum;
  }
}

type Accumulator = {
  keys: GroupKeyEntry[];
  rowKey: string;
  values: Map<string, StatValue>;
  aggBuckets: Map<string, { nums: number[]; count: number }>;
  itemIds: Set<string>;
  itemScopeIds: Map<string, string>;
  itemType: string;
};

/** Build pivoted (and optionally aggregated) rows from flat rows. */
export function buildPivotedRows(
  flatRows: FlatRow[],
  activeIndices: IndexKey[],
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
    group.itemIds.add(row.itemId);
    group.itemScopeIds.set(row.itemId, row.scopeId);

    if (!isAggregating) {
      group.values.set(row.statisticName, row.value);
    } else {
      let bucket = group.aggBuckets.get(row.statisticName);
      if (!bucket) {
        bucket = { nums: [], count: 0 };
        group.aggBuckets.set(row.statisticName, bucket);
      }
      bucket.count++;
      if (isNumericValue(row.value)) {
        bucket.nums.push(row.value);
      }
    }
  }

  const result: PivotedRow[] = [];
  for (const group of groups.values()) {
    const aggs = new Map<string, PivotedRowAgg>();
    if (isAggregating) {
      for (const [stat, bucket] of group.aggBuckets) {
        const hasNum = bucket.nums.length > 0;
        const sum = hasNum ? bucket.nums.reduce((a, b) => a + b, 0) : null;
        const mean = hasNum ? sum! / bucket.nums.length : null;
        const min = hasNum ? Math.min(...bucket.nums) : null;
        const max = hasNum ? Math.max(...bucket.nums) : null;
        let stdev: number | null = null;
        if (mean !== null && bucket.nums.length > 1) {
          const variance =
            bucket.nums.reduce((acc, v) => acc + (v - mean) ** 2, 0) / (bucket.nums.length - 1);
          stdev = Math.sqrt(variance);
        }
        aggs.set(stat, { sum, mean, min, max, stdev, count: bucket.count, isNumeric: hasNum });
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
  return result;
}
