import type { StatValue } from '@/services/query-plan/types';

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

// --- PivotTable renderer component prop types ---

export interface DataHeaderProps {
  stat: string;
  sortInfo: PivotTableSortInfo | null;
  onSort: () => void;
}

export interface GroupCellProps<TRow extends PivotTableRowBase> {
  row: TRow;
  groupKey: PivotTableGroupKeyEntry;
  rowSpan: number;
  columnIndex: number;
}

export interface DataCellProps<TRow extends PivotTableRowBase> {
  row: TRow;
  stat: string;
}

// --- StatGroupTable types ---

export interface HoveredStatInfo {
  name: string;
  /** item ID → numeric value for this stat */
  values: Map<string, number>;
  min: number;
  max: number;
}

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
  key: string;
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
