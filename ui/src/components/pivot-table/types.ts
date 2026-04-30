// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import type { StatValue } from '@quent/utils';
export type { HoveredStatInfo } from '@quent/hooks';

/**
 * Minimal row contract for GroupedDataTable: group columns (with rowSpan) + rowKey for identity.
 */
export interface GroupedDataTableRowBase {
  groupKeys: Array<{ key: string; id: string; label: string }>;
  rowKey: string;
}

export interface GroupedDataTableSortInfo {
  desc: boolean;
}

export interface GroupedDataTableGroupKeyEntry {
  key: string;
  id: string;
  label: string;
}

// --- GroupedDataTable renderer component prop types ---

export interface DataHeaderProps {
  stat: string;
  sortInfo: GroupedDataTableSortInfo | null;
  onSort: () => void;
  className?: string;
  style?: React.CSSProperties;
}

export interface GroupCellProps<TRow extends GroupedDataTableRowBase> {
  row: TRow;
  groupKey: GroupedDataTableGroupKeyEntry;
  rowSpan: number;
  columnIndex: number;
  style?: React.CSSProperties;
  className?: string;
}

export interface DataCellProps<TRow extends GroupedDataTableRowBase> {
  row: TRow;
  stat: string;
}

// --- PivotedStatTable types ---

export type AggMode = 'value' | 'sum' | 'mean' | 'min' | 'max' | 'stdev';

export type SortDir = 'asc' | 'desc';

export interface StatGroupInputGroupValue {
  id: string;
  label: string;
}

export interface StatGroupExpandedRow {
  groups: Record<string, StatGroupInputGroupValue>;
  itemType: string;
  itemId: string;
  scopeId: string;
  statisticName: string;
  value: StatValue;
}

export interface PivotedStatTableSchema<TRow> {
  /**
   * Group dimensions keyed by group id (e.g. partition, item_type, item).
   * These keys are referenced by activeIndices and indexLabels.
   */
  groups: Record<
    string,
    {
      id: (row: TRow) => string;
      label?: (row: TRow) => string;
    }
  >;
  /** Unique item identity used for hover/selection linkage. */
  itemId: (row: TRow) => string;
  /** Scope identity used for cross-view selection linkage. */
  scopeId: (row: TRow) => string;
  /**
   * Optional item type fallback. If omitted, uses group "item_type" id, then "item" id.
   */
  itemType?: (row: TRow) => string;
  /** Stat map for one logical row; keys become table stat columns. */
  stats: (row: TRow) => Record<string, StatValue>;
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
