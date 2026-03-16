import type { StatValue } from '@/services/query-plan/types';

// --- PivotTable base contracts ---

/**
 * Minimal row contract for PivotTable: group columns (with rowSpan) + rowKey for identity.
 */
export interface PivotTableRowBase {
  groupKeys: Array<{ key: string; id: string; label: string }>;
  rowKey: string;
}

export interface PivotTableSortInfo {
  desc: boolean;
}

export interface PivotTableGroupKeyEntry {
  key: string;
  id: string;
  label: string;
}

// --- StatGroupTable types ---

export interface HoveredStatInfo {
  name: string;
  /** item ID → numeric value for this stat */
  values: Map<string, number>;
  min: number;
  max: number;
}

export type IndexKey = 'partition' | 'parent_item_type' | 'parent_item' | 'item_type' | 'item';

export type AggMode = 'value' | 'sum' | 'mean' | 'min' | 'max' | 'stdev';

export type SortDir = 'asc' | 'desc';

export interface FlatRow {
  partitionId: string;
  partitionLabel: string;
  scopeId: string;
  scopeLabel: string;
  parentScopeLabel: string;
  parentItemType: string;
  parentItemName: string;
  itemType: string;
  itemName: string;
  itemId: string;
  statisticName: string;
  value: StatValue;
}

export interface GroupKeyEntry {
  key: IndexKey;
  id: string;
  label: string;
}

export interface PivotedRowAgg {
  sum: number | null;
  mean: number | null;
  min: number | null;
  max: number | null;
  stdev: number | null;
  count: number;
  isNumeric: boolean;
}

export interface PivotedRow {
  groupKeys: GroupKeyEntry[];
  rowKey: string;
  values: Map<string, StatValue>;
  aggs: Map<string, PivotedRowAgg>;
  itemIds: Set<string>;
  itemType: string;
  /** Map from item ID to the scope ID it belongs to */
  itemScopeIds: Map<string, string>;
}
